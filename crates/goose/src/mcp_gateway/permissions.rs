//! MCP Gateway Permission System
//!
//! Function-level permissions for MCP tools.

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

/// User context for permission checks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserContext {
    /// User identifier
    pub user_id: String,
    /// User's groups
    pub groups: Vec<String>,
    /// User's roles
    pub roles: Vec<String>,
    /// Session identifier
    pub session_id: Option<String>,
    /// Additional attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

impl UserContext {
    /// Create a new user context
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            groups: vec![],
            roles: vec![],
            session_id: None,
            attributes: HashMap::new(),
        }
    }

    /// Add user to a group
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.groups.push(group.into());
        self
    }

    /// Add role to user
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Create a snapshot for audit logging
    pub fn snapshot(&self) -> UserContextSnapshot {
        UserContextSnapshot {
            user_id: self.user_id.clone(),
            groups: self.groups.clone(),
            roles: self.roles.clone(),
            session_id: self.session_id.clone(),
        }
    }
}

/// Snapshot of user context for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContextSnapshot {
    pub user_id: String,
    pub groups: Vec<String>,
    pub roles: Vec<String>,
    pub session_id: Option<String>,
}

/// Subject that a permission rule applies to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Subject {
    /// Specific user
    User { id: String },
    /// Group of users
    Group { name: String },
    /// Role
    Role { name: String },
    /// All users
    All,
}

impl Subject {
    /// Check if subject matches user context
    pub fn matches(&self, context: &UserContext) -> bool {
        match self {
            Subject::User { id } => context.user_id == *id,
            Subject::Group { name } => context.groups.contains(name),
            Subject::Role { name } => context.roles.contains(name),
            Subject::All => true,
        }
    }
}

/// Permission decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecision {
    /// Allow the action
    Allow,
    /// Deny the action
    Deny,
    /// Require human approval
    RequireApproval,
}

/// Condition for permission rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Time-based condition
    TimeRange {
        start_hour: u32,
        end_hour: u32,
        timezone: Option<String>,
    },
    /// Day of week condition
    DayOfWeek { days: Vec<String> },
    /// Argument value condition
    ArgumentEquals {
        arg_name: String,
        value: serde_json::Value,
    },
    /// Argument contains condition
    ArgumentContains { arg_name: String, value: String },
    /// Rate limit condition
    RateLimit {
        max_calls: u32,
        period_seconds: u64,
    },
    /// Custom condition
    Custom {
        name: String,
        params: HashMap<String, serde_json::Value>,
    },
}

/// Permission rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Rule identifier
    pub id: String,
    /// Tool name pattern (supports wildcards)
    pub tool_pattern: String,
    /// Subject this rule applies to
    pub subject: Subject,
    /// Permission decision
    pub decision: PermissionDecision,
    /// Optional conditions
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// Rule description
    pub description: Option<String>,
}

impl PermissionRule {
    /// Check if tool name matches pattern
    pub fn matches_tool(&self, tool_name: &str) -> bool {
        // Support simple wildcards: * matches any characters
        let pattern = self
            .tool_pattern
            .replace(".", r"\.")
            .replace("*", ".*")
            .replace("?", ".");

        if let Ok(re) = Regex::new(&format!("^{}$", pattern)) {
            re.is_match(tool_name)
        } else {
            self.tool_pattern == tool_name
        }
    }
}

/// Permission policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    /// Policy identifier
    pub id: String,
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: Option<String>,
    /// Rules in this policy
    pub rules: Vec<PermissionRule>,
    /// Policy priority (higher = evaluated first)
    pub priority: i32,
    /// Whether policy is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Allow list for a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowList {
    /// Allow list identifier
    pub id: String,
    /// Bundle this allow list belongs to
    pub bundle_id: String,
    /// Allowed tools
    pub tools: HashSet<String>,
    /// When created
    pub created_at: DateTime<Utc>,
    /// When this allow list expires
    pub expires_at: Option<DateTime<Utc>>,
    /// Description
    pub description: Option<String>,
}

impl AllowList {
    /// Check if a tool is allowed
    pub fn contains(&self, tool_name: &str) -> bool {
        self.tools.contains(tool_name)
    }

    /// Check if allow list is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
}

/// Default policy behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DefaultPolicy {
    /// Allow by default
    Allow,
    /// Deny by default
    Deny,
    /// Require approval by default
    #[default]
    RequireApproval,
}

impl From<DefaultPolicy> for PermissionCheckResult {
    fn from(policy: DefaultPolicy) -> Self {
        match policy {
            DefaultPolicy::Allow => PermissionCheckResult::Allowed,
            DefaultPolicy::Deny => PermissionCheckResult::Denied {
                reason: "Default policy: deny".to_string(),
            },
            DefaultPolicy::RequireApproval => PermissionCheckResult::RequiresApproval {
                approvers: vec![],
            },
        }
    }
}

/// Result of a permission check
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PermissionCheckResult {
    /// Permission granted
    Allowed,
    /// Permission denied
    Denied { reason: String },
    /// Requires human approval
    RequiresApproval { approvers: Vec<String> },
}

impl PermissionCheckResult {
    /// Check if permission is granted
    pub fn is_allowed(&self) -> bool {
        matches!(self, PermissionCheckResult::Allowed)
    }

    /// Check if permission is denied
    pub fn is_denied(&self) -> bool {
        matches!(self, PermissionCheckResult::Denied { .. })
    }
}

/// Permission manager
pub struct PermissionManager {
    policies: RwLock<Vec<PermissionPolicy>>,
    allow_lists: RwLock<HashMap<String, AllowList>>,
    user_allow_lists: RwLock<HashMap<String, String>>, // user_id -> allow_list_id
    default_policy: DefaultPolicy,
}

impl PermissionManager {
    /// Create a new permission manager with default policy
    pub fn new(default_policy: DefaultPolicy) -> Self {
        Self {
            policies: RwLock::new(Vec::new()),
            allow_lists: RwLock::new(HashMap::new()),
            user_allow_lists: RwLock::new(HashMap::new()),
            default_policy,
        }
    }

    /// Add a permission policy
    pub async fn add_policy(&self, policy: PermissionPolicy) {
        let mut policies = self.policies.write().await;
        policies.push(policy);
        // Sort by priority (descending)
        policies.sort_by_key(|p| std::cmp::Reverse(p.priority));
    }

    /// Remove a policy by ID
    pub async fn remove_policy(&self, policy_id: &str) -> bool {
        let mut policies = self.policies.write().await;
        let len_before = policies.len();
        policies.retain(|p| p.id != policy_id);
        policies.len() < len_before
    }

    /// Create an allow list
    pub async fn create_allow_list(
        &self,
        bundle_id: &str,
        tools: Vec<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> AllowList {
        let allow_list = AllowList {
            id: uuid::Uuid::new_v4().to_string(),
            bundle_id: bundle_id.to_string(),
            tools: tools.into_iter().collect(),
            created_at: Utc::now(),
            expires_at,
            description: None,
        };

        let mut allow_lists = self.allow_lists.write().await;
        allow_lists.insert(allow_list.id.clone(), allow_list.clone());

        allow_list
    }

    /// Assign allow list to user
    pub async fn assign_allow_list(&self, user_id: &str, allow_list_id: &str) {
        let mut user_allow_lists = self.user_allow_lists.write().await;
        user_allow_lists.insert(user_id.to_string(), allow_list_id.to_string());
    }

    /// Get allow list for user
    pub async fn get_allow_list(&self, user_context: &UserContext) -> Option<AllowList> {
        let user_allow_lists = self.user_allow_lists.read().await;
        let allow_list_id = user_allow_lists.get(&user_context.user_id)?;

        let allow_lists = self.allow_lists.read().await;
        allow_lists.get(allow_list_id).cloned()
    }

    /// Check if a user can execute a tool
    pub async fn check_permission(
        &self,
        tool_name: &str,
        user_context: &UserContext,
    ) -> PermissionCheckResult {
        // 1. Check allow lists first
        if let Some(allow_list) = self.get_allow_list(user_context).await {
            if allow_list.is_expired() {
                return PermissionCheckResult::Denied {
                    reason: format!("Allow list expired for bundle: {}", allow_list.bundle_id),
                };
            }
            if !allow_list.contains(tool_name) {
                return PermissionCheckResult::Denied {
                    reason: format!("Tool '{}' not in allow list", tool_name),
                };
            }
        }

        // 2. Evaluate policies in priority order
        let policies = self.policies.read().await;
        for policy in policies.iter() {
            if !policy.enabled {
                continue;
            }

            for rule in &policy.rules {
                if !rule.matches_tool(tool_name) {
                    continue;
                }

                if !rule.subject.matches(user_context) {
                    continue;
                }

                // TODO: Evaluate conditions

                match rule.decision {
                    PermissionDecision::Allow => {
                        return PermissionCheckResult::Allowed;
                    }
                    PermissionDecision::Deny => {
                        return PermissionCheckResult::Denied {
                            reason: rule
                                .description
                                .clone()
                                .unwrap_or_else(|| format!("Denied by rule: {}", rule.id)),
                        };
                    }
                    PermissionDecision::RequireApproval => {
                        return PermissionCheckResult::RequiresApproval {
                            approvers: vec![], // TODO: Get approvers from rule
                        };
                    }
                }
            }
        }

        // 3. Apply default policy
        self.default_policy.into()
    }

    /// Get all policies
    pub async fn list_policies(&self) -> Vec<PermissionPolicy> {
        let policies = self.policies.read().await;
        policies.clone()
    }

    /// Get policy by ID
    pub async fn get_policy(&self, policy_id: &str) -> Option<PermissionPolicy> {
        let policies = self.policies.read().await;
        policies.iter().find(|p| p.id == policy_id).cloned()
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new(DefaultPolicy::RequireApproval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_permission_manager_default_deny() {
        let manager = PermissionManager::new(DefaultPolicy::Deny);
        let context = UserContext::new("user1");

        let result = manager.check_permission("some_tool", &context).await;
        assert!(result.is_denied());
    }

    #[tokio::test]
    async fn test_permission_manager_default_allow() {
        let manager = PermissionManager::new(DefaultPolicy::Allow);
        let context = UserContext::new("user1");

        let result = manager.check_permission("some_tool", &context).await;
        assert!(result.is_allowed());
    }

    #[tokio::test]
    async fn test_permission_policy_rule() {
        let manager = PermissionManager::new(DefaultPolicy::Deny);

        let policy = PermissionPolicy {
            id: "policy1".to_string(),
            name: "Test Policy".to_string(),
            description: None,
            rules: vec![PermissionRule {
                id: "rule1".to_string(),
                tool_pattern: "file_*".to_string(),
                subject: Subject::All,
                decision: PermissionDecision::Allow,
                conditions: vec![],
                description: None,
            }],
            priority: 100,
            enabled: true,
        };

        manager.add_policy(policy).await;

        let context = UserContext::new("user1");

        // file_read should be allowed
        let result = manager.check_permission("file_read", &context).await;
        assert!(result.is_allowed());

        // bash should be denied (no matching rule)
        let result = manager.check_permission("bash", &context).await;
        assert!(result.is_denied());
    }

    #[tokio::test]
    async fn test_permission_user_specific_rule() {
        let manager = PermissionManager::new(DefaultPolicy::Deny);

        let policy = PermissionPolicy {
            id: "policy1".to_string(),
            name: "User Policy".to_string(),
            description: None,
            rules: vec![PermissionRule {
                id: "rule1".to_string(),
                tool_pattern: "*".to_string(),
                subject: Subject::User {
                    id: "admin".to_string(),
                },
                decision: PermissionDecision::Allow,
                conditions: vec![],
                description: None,
            }],
            priority: 100,
            enabled: true,
        };

        manager.add_policy(policy).await;

        // Admin should have access
        let admin_context = UserContext::new("admin");
        let result = manager.check_permission("any_tool", &admin_context).await;
        assert!(result.is_allowed());

        // Regular user should be denied
        let user_context = UserContext::new("user1");
        let result = manager.check_permission("any_tool", &user_context).await;
        assert!(result.is_denied());
    }

    #[tokio::test]
    async fn test_permission_group_rule() {
        let manager = PermissionManager::new(DefaultPolicy::Deny);

        let policy = PermissionPolicy {
            id: "policy1".to_string(),
            name: "Group Policy".to_string(),
            description: None,
            rules: vec![PermissionRule {
                id: "rule1".to_string(),
                tool_pattern: "deploy_*".to_string(),
                subject: Subject::Group {
                    name: "devops".to_string(),
                },
                decision: PermissionDecision::Allow,
                conditions: vec![],
                description: None,
            }],
            priority: 100,
            enabled: true,
        };

        manager.add_policy(policy).await;

        // DevOps group member should have access
        let devops_context = UserContext::new("user1").with_group("devops");
        let result = manager.check_permission("deploy_staging", &devops_context).await;
        assert!(result.is_allowed());

        // Non-devops user should be denied
        let dev_context = UserContext::new("user2").with_group("developers");
        let result = manager.check_permission("deploy_staging", &dev_context).await;
        assert!(result.is_denied());
    }

    #[tokio::test]
    async fn test_allow_list() {
        let manager = PermissionManager::new(DefaultPolicy::Deny);

        // Create allow list
        let allow_list = manager
            .create_allow_list(
                "bundle1",
                vec!["file_read".to_string(), "file_write".to_string()],
                None,
            )
            .await;

        // Assign to user
        manager.assign_allow_list("user1", &allow_list.id).await;

        // Add permissive policy
        let policy = PermissionPolicy {
            id: "policy1".to_string(),
            name: "Allow All".to_string(),
            description: None,
            rules: vec![PermissionRule {
                id: "rule1".to_string(),
                tool_pattern: "*".to_string(),
                subject: Subject::All,
                decision: PermissionDecision::Allow,
                conditions: vec![],
                description: None,
            }],
            priority: 100,
            enabled: true,
        };
        manager.add_policy(policy).await;

        let context = UserContext::new("user1");

        // Allowed tool should work
        let result = manager.check_permission("file_read", &context).await;
        assert!(result.is_allowed());

        // Not in allow list - denied even with permissive policy
        let result = manager.check_permission("bash", &context).await;
        assert!(result.is_denied());
    }

    #[tokio::test]
    async fn test_allow_list_expired() {
        let manager = PermissionManager::new(DefaultPolicy::Deny);

        // Create expired allow list
        let allow_list = manager
            .create_allow_list(
                "bundle1",
                vec!["file_read".to_string()],
                Some(Utc::now() - chrono::Duration::hours(1)), // Expired
            )
            .await;

        manager.assign_allow_list("user1", &allow_list.id).await;

        let context = UserContext::new("user1");
        let result = manager.check_permission("file_read", &context).await;

        assert!(result.is_denied());
        if let PermissionCheckResult::Denied { reason } = result {
            assert!(reason.contains("expired"));
        }
    }

    #[tokio::test]
    async fn test_policy_priority() {
        let manager = PermissionManager::new(DefaultPolicy::Deny);

        // Low priority - deny all
        let deny_policy = PermissionPolicy {
            id: "deny".to_string(),
            name: "Deny All".to_string(),
            description: None,
            rules: vec![PermissionRule {
                id: "deny_all".to_string(),
                tool_pattern: "*".to_string(),
                subject: Subject::All,
                decision: PermissionDecision::Deny,
                conditions: vec![],
                description: None,
            }],
            priority: 10,
            enabled: true,
        };

        // High priority - allow for admin
        let allow_policy = PermissionPolicy {
            id: "allow_admin".to_string(),
            name: "Allow Admin".to_string(),
            description: None,
            rules: vec![PermissionRule {
                id: "allow_admin".to_string(),
                tool_pattern: "*".to_string(),
                subject: Subject::User {
                    id: "admin".to_string(),
                },
                decision: PermissionDecision::Allow,
                conditions: vec![],
                description: None,
            }],
            priority: 100, // Higher priority
            enabled: true,
        };

        manager.add_policy(deny_policy).await;
        manager.add_policy(allow_policy).await;

        // Admin should be allowed (high priority policy)
        let admin_context = UserContext::new("admin");
        let result = manager.check_permission("any_tool", &admin_context).await;
        assert!(result.is_allowed());

        // Regular user should be denied (low priority policy)
        let user_context = UserContext::new("user1");
        let result = manager.check_permission("any_tool", &user_context).await;
        assert!(result.is_denied());
    }

    #[test]
    fn test_rule_pattern_matching() {
        let rule = PermissionRule {
            id: "test".to_string(),
            tool_pattern: "file_*".to_string(),
            subject: Subject::All,
            decision: PermissionDecision::Allow,
            conditions: vec![],
            description: None,
        };

        assert!(rule.matches_tool("file_read"));
        assert!(rule.matches_tool("file_write"));
        assert!(rule.matches_tool("file_delete"));
        assert!(!rule.matches_tool("bash"));
        assert!(!rule.matches_tool("myfile_read"));
    }

    #[test]
    fn test_rule_exact_matching() {
        let rule = PermissionRule {
            id: "test".to_string(),
            tool_pattern: "bash".to_string(),
            subject: Subject::All,
            decision: PermissionDecision::Allow,
            conditions: vec![],
            description: None,
        };

        assert!(rule.matches_tool("bash"));
        assert!(!rule.matches_tool("bash_exec"));
        assert!(!rule.matches_tool("mybash"));
    }
}
