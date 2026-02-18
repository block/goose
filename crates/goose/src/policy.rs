use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::identity::{AuthMethod, UserIdentity};

/// Result of evaluating a policy rule against a request.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
    Abstain,
}

/// What a rule does when matched.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// A single policy rule evaluated against identity + action + resource.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PolicyRule {
    pub id: String,
    pub description: String,
    pub priority: i32,
    pub effect: PolicyEffect,
    /// Action patterns like "execute:*", "manage:agents", "read:sessions"
    pub actions: Vec<String>,
    /// Resource patterns like "agent:*", "session:abc123"
    pub resources: Vec<String>,
    /// If set, rule only applies to these auth methods
    pub auth_methods: Option<Vec<String>>,
    /// If set, rule only applies within this tenant
    pub tenant: Option<String>,
    /// Deny reason (for Deny rules)
    pub reason: Option<String>,
}

impl PolicyRule {
    fn action_matches(&self, action: &str) -> bool {
        self.actions
            .iter()
            .any(|pattern| match pattern.strip_suffix('*') {
                Some(prefix) => action.starts_with(prefix),
                None => pattern == action,
            })
    }

    fn resource_matches(&self, resource: &str) -> bool {
        self.resources
            .iter()
            .any(|pattern| match pattern.strip_suffix('*') {
                Some(prefix) => resource.starts_with(prefix),
                None => pattern == resource,
            })
    }

    fn auth_method_matches(&self, user: &UserIdentity) -> bool {
        match &self.auth_methods {
            None => true,
            Some(methods) => {
                let tag = auth_method_tag(&user.auth_method);
                methods.iter().any(|m| m == &tag)
            }
        }
    }

    fn tenant_matches(&self, user: &UserIdentity) -> bool {
        match (&self.tenant, &user.tenant) {
            (None, _) => true,
            (Some(rule_tenant), Some(user_tenant)) => rule_tenant == user_tenant,
            (Some(_), None) => false,
        }
    }

    fn applies_to(&self, user: &UserIdentity, action: &str, resource: &str) -> bool {
        self.action_matches(action)
            && self.resource_matches(resource)
            && self.auth_method_matches(user)
            && self.tenant_matches(user)
    }
}

fn auth_method_tag(method: &AuthMethod) -> String {
    match method {
        AuthMethod::Guest => "guest".to_string(),
        AuthMethod::Oidc { provider, .. } => format!("oidc:{provider}"),
        AuthMethod::ApiKey => "apikey".to_string(),
        AuthMethod::Password => "password".to_string(),
        AuthMethod::ServiceAccount { service_name } => {
            format!("service:{service_name}")
        }
    }
}

/// Evaluates an ordered set of rules against a request.
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl PolicyEngine {
    pub fn new(mut rules: Vec<PolicyRule>) -> Self {
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        Self { rules }
    }

    pub fn evaluate(&self, user: &UserIdentity, action: &str, resource: &str) -> PolicyDecision {
        for rule in &self.rules {
            if rule.applies_to(user, action, resource) {
                return match rule.effect {
                    PolicyEffect::Allow => PolicyDecision::Allow,
                    PolicyEffect::Deny => PolicyDecision::Deny {
                        reason: rule
                            .reason
                            .clone()
                            .unwrap_or_else(|| format!("denied by rule '{}'", rule.id)),
                    },
                };
            }
        }
        PolicyDecision::Abstain
    }
}

/// Builder for constructing PolicyRule instances.
pub struct PolicyRuleBuilder {
    rule: PolicyRule,
}

impl PolicyRuleBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            rule: PolicyRule {
                id: id.into(),
                description: String::new(),
                priority: 0,
                effect: PolicyEffect::Allow,
                actions: vec![],
                resources: vec!["*".to_string()],
                auth_methods: None,
                tenant: None,
                reason: None,
            },
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.rule.description = desc.into();
        self
    }

    pub fn priority(mut self, p: i32) -> Self {
        self.rule.priority = p;
        self
    }

    pub fn allow(mut self) -> Self {
        self.rule.effect = PolicyEffect::Allow;
        self
    }

    pub fn deny(mut self) -> Self {
        self.rule.effect = PolicyEffect::Deny;
        self
    }

    pub fn actions(mut self, actions: Vec<String>) -> Self {
        self.rule.actions = actions;
        self
    }

    pub fn resources(mut self, resources: Vec<String>) -> Self {
        self.rule.resources = resources;
        self
    }

    pub fn auth_methods(mut self, methods: Vec<String>) -> Self {
        self.rule.auth_methods = Some(methods);
        self
    }

    pub fn tenant(mut self, t: impl Into<String>) -> Self {
        self.rule.tenant = Some(t.into());
        self
    }

    pub fn reason(mut self, r: impl Into<String>) -> Self {
        self.rule.reason = Some(r.into());
        self
    }

    pub fn build(self) -> PolicyRule {
        self.rule
    }
}

/// Thread-safe policy store with default rules and per-tenant overrides.
pub struct PolicyStore {
    global_rules: Arc<RwLock<Vec<PolicyRule>>>,
    tenant_overrides: Arc<RwLock<HashMap<String, Vec<PolicyRule>>>>,
}

impl PolicyStore {
    pub fn new() -> Self {
        Self {
            global_rules: Arc::new(RwLock::new(default_rules())),
            tenant_overrides: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_rule(&self, rule: PolicyRule) {
        self.global_rules.write().await.push(rule);
    }

    pub async fn add_tenant_rule(&self, tenant: &str, rule: PolicyRule) {
        self.tenant_overrides
            .write()
            .await
            .entry(tenant.to_string())
            .or_default()
            .push(rule);
    }

    pub async fn remove_rule(&self, id: &str) -> bool {
        let mut rules = self.global_rules.write().await;
        let before = rules.len();
        rules.retain(|r| r.id != id);
        rules.len() < before
    }

    pub async fn engine_for(&self, tenant: Option<&str>) -> PolicyEngine {
        let global = self.global_rules.read().await.clone();
        let mut all_rules = global;
        if let Some(t) = tenant {
            if let Some(overrides) = self.tenant_overrides.read().await.get(t) {
                all_rules.extend(overrides.clone());
            }
        }
        PolicyEngine::new(all_rules)
    }
}

impl Default for PolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

fn default_rules() -> Vec<PolicyRule> {
    vec![
        PolicyRuleBuilder::new("deny-guest-management")
            .description("Guests cannot manage agents or configuration")
            .priority(100)
            .deny()
            .actions(vec!["manage:*".to_string()])
            .auth_methods(vec!["guest".to_string()])
            .reason("Authentication required for management operations")
            .build(),
        PolicyRuleBuilder::new("allow-execute")
            .description("All authenticated users can execute agents")
            .priority(50)
            .allow()
            .actions(vec!["execute:*".to_string()])
            .build(),
        PolicyRuleBuilder::new("allow-read")
            .description("All users can read public resources")
            .priority(50)
            .allow()
            .actions(vec!["read:*".to_string()])
            .build(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::AuthMethod;

    fn make_guest() -> UserIdentity {
        UserIdentity {
            id: "guest-1".to_string(),
            name: "Guest".to_string(),
            auth_method: AuthMethod::Guest,
            tenant: None,
        }
    }

    fn make_oidc_user(tenant: Option<&str>) -> UserIdentity {
        UserIdentity {
            id: "user-1".to_string(),
            name: "Alice".to_string(),
            auth_method: AuthMethod::Oidc {
                provider: "google".to_string(),
                subject: "alice@example.com".to_string(),
            },
            tenant: tenant.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_guest_denied_management() {
        let engine = PolicyEngine::new(default_rules());
        let guest = make_guest();
        assert!(matches!(
            engine.evaluate(&guest, "manage:agents", "agent:x"),
            PolicyDecision::Deny { .. }
        ));
    }

    #[test]
    fn test_guest_can_execute() {
        let engine = PolicyEngine::new(default_rules());
        let guest = make_guest();
        assert_eq!(
            engine.evaluate(&guest, "execute:agent", "agent:x"),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn test_oidc_user_can_execute() {
        let engine = PolicyEngine::new(default_rules());
        let user = make_oidc_user(None);
        assert_eq!(
            engine.evaluate(&user, "execute:agent", "agent:x"),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn test_wildcard_action() {
        let rule = PolicyRuleBuilder::new("test")
            .allow()
            .actions(vec!["execute:*".to_string()])
            .build();
        let engine = PolicyEngine::new(vec![rule]);
        let user = make_oidc_user(None);
        assert_eq!(
            engine.evaluate(&user, "execute:sub-agent", "agent:x"),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn test_prefix_resource_match() {
        let rule = PolicyRuleBuilder::new("test")
            .allow()
            .actions(vec!["read:*".to_string()])
            .resources(vec!["session:tenant-a/*".to_string()])
            .build();
        let engine = PolicyEngine::new(vec![rule]);
        let user = make_oidc_user(None);
        assert_eq!(
            engine.evaluate(&user, "read:session", "session:tenant-a/123"),
            PolicyDecision::Allow
        );
        assert_eq!(
            engine.evaluate(&user, "read:session", "session:tenant-b/456"),
            PolicyDecision::Abstain
        );
    }

    #[test]
    fn test_priority_ordering() {
        let deny = PolicyRuleBuilder::new("deny-all")
            .priority(10)
            .deny()
            .actions(vec!["execute:*".to_string()])
            .reason("blocked")
            .build();
        let allow = PolicyRuleBuilder::new("allow-all")
            .priority(100)
            .allow()
            .actions(vec!["execute:*".to_string()])
            .build();
        let engine = PolicyEngine::new(vec![deny, allow]);
        let user = make_oidc_user(None);
        assert_eq!(
            engine.evaluate(&user, "execute:agent", "agent:x"),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn test_auth_method_filter() {
        let rule = PolicyRuleBuilder::new("oidc-only")
            .allow()
            .actions(vec!["manage:*".to_string()])
            .auth_methods(vec!["oidc:google".to_string()])
            .build();
        let engine = PolicyEngine::new(vec![rule]);
        let oidc_user = make_oidc_user(None);
        let api_user = UserIdentity {
            id: "api-1".to_string(),
            name: "Bot".to_string(),
            auth_method: AuthMethod::ApiKey,
            tenant: None,
        };
        assert_eq!(
            engine.evaluate(&oidc_user, "manage:settings", "config:x"),
            PolicyDecision::Allow
        );
        assert_eq!(
            engine.evaluate(&api_user, "manage:settings", "config:x"),
            PolicyDecision::Abstain
        );
    }

    #[test]
    fn test_tenant_scoping() {
        let rule = PolicyRuleBuilder::new("tenant-a-only")
            .allow()
            .actions(vec!["execute:*".to_string()])
            .tenant("tenant-a")
            .build();
        let engine = PolicyEngine::new(vec![rule]);
        let user_a = make_oidc_user(Some("tenant-a"));
        let user_b = make_oidc_user(Some("tenant-b"));
        assert_eq!(
            engine.evaluate(&user_a, "execute:agent", "agent:x"),
            PolicyDecision::Allow
        );
        assert_eq!(
            engine.evaluate(&user_b, "execute:agent", "agent:x"),
            PolicyDecision::Abstain
        );
    }

    #[test]
    fn test_abstain_on_no_match() {
        let engine = PolicyEngine::new(vec![]);
        let user = make_oidc_user(None);
        assert_eq!(
            engine.evaluate(&user, "unknown:action", "resource:x"),
            PolicyDecision::Abstain
        );
    }

    #[test]
    fn test_rule_builder() {
        let rule = PolicyRuleBuilder::new("test-rule")
            .description("A test")
            .priority(42)
            .deny()
            .actions(vec!["manage:*".to_string()])
            .resources(vec!["agent:special".to_string()])
            .auth_methods(vec!["guest".to_string()])
            .tenant("acme")
            .reason("not allowed")
            .build();
        assert_eq!(rule.id, "test-rule");
        assert_eq!(rule.priority, 42);
        assert_eq!(rule.effect, PolicyEffect::Deny);
        assert_eq!(rule.tenant, Some("acme".to_string()));
    }

    #[tokio::test]
    async fn test_policy_store() {
        let store = PolicyStore::new();
        let engine = store.engine_for(None).await;
        let guest = make_guest();
        assert!(matches!(
            engine.evaluate(&guest, "manage:agents", "agent:x"),
            PolicyDecision::Deny { .. }
        ));
    }

    #[tokio::test]
    async fn test_policy_store_tenant_override() {
        let store = PolicyStore::new();
        let extra = PolicyRuleBuilder::new("acme-extra")
            .priority(200)
            .allow()
            .actions(vec!["manage:*".to_string()])
            .build();
        store.add_tenant_rule("acme", extra).await;
        let engine = store.engine_for(Some("acme")).await;
        let guest = make_guest();
        assert_eq!(
            engine.evaluate(&guest, "manage:agents", "agent:x"),
            PolicyDecision::Allow
        );
    }

    #[tokio::test]
    async fn test_remove_rule() {
        let store = PolicyStore::new();
        // Before removal: guest is denied management
        let engine = store.engine_for(None).await;
        let guest = make_guest();
        assert!(matches!(
            engine.evaluate(&guest, "manage:agents", "agent:x"),
            PolicyDecision::Deny { .. }
        ));
        // Remove the deny rule
        assert!(store.remove_rule("deny-guest-management").await);
        // After removal: no rule matches manage:*, so abstain
        let engine = store.engine_for(None).await;
        assert_eq!(
            engine.evaluate(&guest, "manage:agents", "agent:x"),
            PolicyDecision::Abstain
        );
        // But execute still works
        assert_eq!(
            engine.evaluate(&guest, "execute:agent", "agent:x"),
            PolicyDecision::Allow
        );
    }
}
