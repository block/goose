//! MCP Gateway Bundles
//!
//! User bundle management for organizing MCP servers and tools.

use super::errors::GatewayError;
use super::permissions::AllowList;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

/// Bundle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleStatus {
    /// Bundle is active and usable
    Active,
    /// Bundle is suspended
    Suspended,
    /// Bundle is archived
    Archived,
    /// Bundle is pending approval
    Pending,
}

/// Bundle definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    /// Unique bundle identifier
    pub id: String,
    /// Bundle name
    pub name: String,
    /// Bundle description
    pub description: Option<String>,
    /// Bundle status
    pub status: BundleStatus,
    /// MCP servers included in bundle
    pub servers: Vec<String>,
    /// Tool allow list (if None, all server tools are allowed)
    pub allowed_tools: Option<HashSet<String>>,
    /// Tool deny list (takes precedence over allow list)
    pub denied_tools: HashSet<String>,
    /// Maximum concurrent tool executions
    pub max_concurrent_executions: Option<u32>,
    /// Rate limit (calls per minute)
    pub rate_limit_per_minute: Option<u32>,
    /// When bundle was created
    pub created_at: DateTime<Utc>,
    /// When bundle was last modified
    pub modified_at: DateTime<Utc>,
    /// Bundle owner
    pub owner: String,
    /// Users assigned to this bundle
    pub users: HashSet<String>,
    /// Groups assigned to this bundle
    pub groups: HashSet<String>,
    /// Bundle metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Bundle {
    /// Create a new bundle
    pub fn new(id: impl Into<String>, name: impl Into<String>, owner: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            status: BundleStatus::Active,
            servers: Vec::new(),
            allowed_tools: None,
            denied_tools: HashSet::new(),
            max_concurrent_executions: None,
            rate_limit_per_minute: None,
            created_at: now,
            modified_at: now,
            owner: owner.into(),
            users: HashSet::new(),
            groups: HashSet::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add server to bundle
    pub fn with_server(mut self, server_id: impl Into<String>) -> Self {
        self.servers.push(server_id.into());
        self
    }

    /// Set allowed tools
    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = Some(tools.into_iter().collect());
        self
    }

    /// Add denied tool
    pub fn deny_tool(mut self, tool: impl Into<String>) -> Self {
        self.denied_tools.insert(tool.into());
        self
    }

    /// Set rate limit
    pub fn with_rate_limit(mut self, per_minute: u32) -> Self {
        self.rate_limit_per_minute = Some(per_minute);
        self
    }

    /// Add user to bundle
    pub fn add_user(&mut self, user_id: impl Into<String>) {
        self.users.insert(user_id.into());
        self.modified_at = Utc::now();
    }

    /// Remove user from bundle
    pub fn remove_user(&mut self, user_id: &str) -> bool {
        let removed = self.users.remove(user_id);
        if removed {
            self.modified_at = Utc::now();
        }
        removed
    }

    /// Add group to bundle
    pub fn add_group(&mut self, group: impl Into<String>) {
        self.groups.insert(group.into());
        self.modified_at = Utc::now();
    }

    /// Check if tool is allowed in bundle
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        // Denied list takes precedence
        if self.denied_tools.contains(tool_name) {
            return false;
        }

        // If no allow list, all tools are allowed
        match &self.allowed_tools {
            None => true,
            Some(allowed) => allowed.contains(tool_name),
        }
    }

    /// Check if user has access to bundle
    pub fn has_user_access(&self, user_id: &str, user_groups: &[String]) -> bool {
        if self.status != BundleStatus::Active {
            return false;
        }

        // Direct user assignment
        if self.users.contains(user_id) {
            return true;
        }

        // Group membership
        for group in user_groups {
            if self.groups.contains(group) {
                return true;
            }
        }

        false
    }

    /// Convert to allow list
    pub fn to_allow_list(&self) -> AllowList {
        let tools = match &self.allowed_tools {
            Some(allowed) => allowed
                .iter()
                .filter(|t| !self.denied_tools.contains(*t))
                .cloned()
                .collect(),
            None => HashSet::new(), // Empty means all allowed (handled elsewhere)
        };

        AllowList {
            id: format!("bundle-{}", self.id),
            bundle_id: self.id.clone(),
            tools,
            created_at: self.created_at,
            expires_at: None,
            description: self.description.clone(),
        }
    }
}

/// Bundle manager
pub struct BundleManager {
    bundles: RwLock<HashMap<String, Bundle>>,
    user_bundles: RwLock<HashMap<String, Vec<String>>>, // user_id -> bundle_ids
}

impl BundleManager {
    /// Create new bundle manager
    pub fn new() -> Self {
        Self {
            bundles: RwLock::new(HashMap::new()),
            user_bundles: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new bundle
    pub async fn create_bundle(&self, bundle: Bundle) -> Result<String, GatewayError> {
        let bundle_id = bundle.id.clone();

        let mut bundles = self.bundles.write().await;
        if bundles.contains_key(&bundle_id) {
            return Err(GatewayError::Internal(format!(
                "Bundle already exists: {}",
                bundle_id
            )));
        }

        // Update user mappings
        let mut user_bundles = self.user_bundles.write().await;
        for user_id in &bundle.users {
            user_bundles
                .entry(user_id.clone())
                .or_default()
                .push(bundle_id.clone());
        }

        bundles.insert(bundle_id.clone(), bundle);

        tracing::info!(bundle_id = %bundle_id, "Created bundle");

        Ok(bundle_id)
    }

    /// Get bundle by ID
    pub async fn get_bundle(&self, bundle_id: &str) -> Option<Bundle> {
        let bundles = self.bundles.read().await;
        bundles.get(bundle_id).cloned()
    }

    /// Update bundle
    pub async fn update_bundle(&self, bundle: Bundle) -> Result<(), GatewayError> {
        let mut bundles = self.bundles.write().await;
        if !bundles.contains_key(&bundle.id) {
            return Err(GatewayError::BundleNotFound {
                bundle_id: bundle.id.clone(),
            });
        }

        bundles.insert(bundle.id.clone(), bundle);
        Ok(())
    }

    /// Delete bundle
    pub async fn delete_bundle(&self, bundle_id: &str) -> Result<(), GatewayError> {
        let mut bundles = self.bundles.write().await;
        let bundle = bundles.remove(bundle_id).ok_or_else(|| GatewayError::BundleNotFound {
            bundle_id: bundle_id.to_string(),
        })?;

        // Update user mappings
        let mut user_bundles = self.user_bundles.write().await;
        for user_id in &bundle.users {
            if let Some(ids) = user_bundles.get_mut(user_id) {
                ids.retain(|id| id != bundle_id);
            }
        }

        tracing::info!(bundle_id = %bundle_id, "Deleted bundle");

        Ok(())
    }

    /// Assign user to bundle
    pub async fn assign_user(&self, bundle_id: &str, user_id: &str) -> Result<(), GatewayError> {
        let mut bundles = self.bundles.write().await;
        let bundle = bundles
            .get_mut(bundle_id)
            .ok_or_else(|| GatewayError::BundleNotFound {
                bundle_id: bundle_id.to_string(),
            })?;

        bundle.add_user(user_id);

        // Update user mappings
        let mut user_bundles = self.user_bundles.write().await;
        user_bundles
            .entry(user_id.to_string())
            .or_default()
            .push(bundle_id.to_string());

        Ok(())
    }

    /// Remove user from bundle
    pub async fn remove_user(&self, bundle_id: &str, user_id: &str) -> Result<(), GatewayError> {
        let mut bundles = self.bundles.write().await;
        let bundle = bundles
            .get_mut(bundle_id)
            .ok_or_else(|| GatewayError::BundleNotFound {
                bundle_id: bundle_id.to_string(),
            })?;

        bundle.remove_user(user_id);

        // Update user mappings
        let mut user_bundles = self.user_bundles.write().await;
        if let Some(ids) = user_bundles.get_mut(user_id) {
            ids.retain(|id| id != bundle_id);
        }

        Ok(())
    }

    /// Get bundles for user
    pub async fn get_user_bundles(&self, user_id: &str, user_groups: &[String]) -> Vec<Bundle> {
        let bundles = self.bundles.read().await;

        bundles
            .values()
            .filter(|b| b.has_user_access(user_id, user_groups))
            .cloned()
            .collect()
    }

    /// Check if user can access tool through any bundle
    pub async fn can_user_access_tool(
        &self,
        user_id: &str,
        user_groups: &[String],
        tool_name: &str,
    ) -> bool {
        let user_bundles = self.get_user_bundles(user_id, user_groups).await;

        for bundle in user_bundles {
            if bundle.is_tool_allowed(tool_name) {
                return true;
            }
        }

        false
    }

    /// List all bundles
    pub async fn list_bundles(&self) -> Vec<Bundle> {
        let bundles = self.bundles.read().await;
        bundles.values().cloned().collect()
    }

    /// Get bundle count
    pub async fn count(&self) -> usize {
        let bundles = self.bundles.read().await;
        bundles.len()
    }

    /// Update bundle status
    pub async fn set_status(
        &self,
        bundle_id: &str,
        status: BundleStatus,
    ) -> Result<(), GatewayError> {
        let mut bundles = self.bundles.write().await;
        let bundle = bundles
            .get_mut(bundle_id)
            .ok_or_else(|| GatewayError::BundleNotFound {
                bundle_id: bundle_id.to_string(),
            })?;

        bundle.status = status;
        bundle.modified_at = Utc::now();

        Ok(())
    }
}

impl Default for BundleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_creation() {
        let bundle = Bundle::new("bundle1", "Test Bundle", "admin")
            .with_description("A test bundle")
            .with_server("server1")
            .with_server("server2")
            .with_rate_limit(100);

        assert_eq!(bundle.id, "bundle1");
        assert_eq!(bundle.name, "Test Bundle");
        assert_eq!(bundle.owner, "admin");
        assert_eq!(bundle.servers.len(), 2);
        assert_eq!(bundle.rate_limit_per_minute, Some(100));
    }

    #[test]
    fn test_bundle_tool_access() {
        let bundle = Bundle::new("bundle1", "Test", "admin")
            .with_allowed_tools(vec!["read".to_string(), "write".to_string()])
            .deny_tool("delete");

        assert!(bundle.is_tool_allowed("read"));
        assert!(bundle.is_tool_allowed("write"));
        assert!(!bundle.is_tool_allowed("execute"));
        assert!(!bundle.is_tool_allowed("delete")); // Denied takes precedence
    }

    #[test]
    fn test_bundle_no_allow_list() {
        let mut bundle = Bundle::new("bundle1", "Test", "admin");
        bundle.denied_tools.insert("dangerous".to_string());

        // No allow list means all tools allowed (except denied)
        assert!(bundle.is_tool_allowed("anything"));
        assert!(bundle.is_tool_allowed("read"));
        assert!(!bundle.is_tool_allowed("dangerous"));
    }

    #[test]
    fn test_bundle_user_access() {
        let mut bundle = Bundle::new("bundle1", "Test", "admin");
        bundle.add_user("user1");
        bundle.groups.insert("developers".to_string());

        // Direct user access
        assert!(bundle.has_user_access("user1", &[]));

        // Group access
        assert!(bundle.has_user_access("user2", &["developers".to_string()]));

        // No access
        assert!(!bundle.has_user_access("user3", &["other_group".to_string()]));
    }

    #[test]
    fn test_bundle_inactive_no_access() {
        let mut bundle = Bundle::new("bundle1", "Test", "admin");
        bundle.add_user("user1");
        bundle.status = BundleStatus::Suspended;

        assert!(!bundle.has_user_access("user1", &[]));
    }

    #[tokio::test]
    async fn test_bundle_manager_create() {
        let manager = BundleManager::new();

        let bundle = Bundle::new("bundle1", "Test Bundle", "admin");
        let id = manager.create_bundle(bundle).await.unwrap();

        assert_eq!(id, "bundle1");
        assert_eq!(manager.count().await, 1);

        let retrieved = manager.get_bundle("bundle1").await.unwrap();
        assert_eq!(retrieved.name, "Test Bundle");
    }

    #[tokio::test]
    async fn test_bundle_manager_user_assignment() {
        let manager = BundleManager::new();

        let bundle = Bundle::new("bundle1", "Test Bundle", "admin");
        manager.create_bundle(bundle).await.unwrap();

        manager.assign_user("bundle1", "user1").await.unwrap();

        let user_bundles = manager.get_user_bundles("user1", &[]).await;
        assert_eq!(user_bundles.len(), 1);
        assert_eq!(user_bundles[0].id, "bundle1");
    }

    #[tokio::test]
    async fn test_bundle_manager_tool_access_check() {
        let manager = BundleManager::new();

        let bundle = Bundle::new("bundle1", "Test Bundle", "admin")
            .with_allowed_tools(vec!["read".to_string(), "write".to_string()]);
        manager.create_bundle(bundle).await.unwrap();
        manager.assign_user("bundle1", "user1").await.unwrap();

        assert!(manager.can_user_access_tool("user1", &[], "read").await);
        assert!(!manager.can_user_access_tool("user1", &[], "delete").await);
        assert!(!manager.can_user_access_tool("user2", &[], "read").await);
    }

    #[tokio::test]
    async fn test_bundle_manager_delete() {
        let manager = BundleManager::new();

        let bundle = Bundle::new("bundle1", "Test Bundle", "admin");
        manager.create_bundle(bundle).await.unwrap();
        manager.assign_user("bundle1", "user1").await.unwrap();

        manager.delete_bundle("bundle1").await.unwrap();

        assert!(manager.get_bundle("bundle1").await.is_none());
        assert!(manager.get_user_bundles("user1", &[]).await.is_empty());
    }

    #[tokio::test]
    async fn test_bundle_status_change() {
        let manager = BundleManager::new();

        let mut bundle = Bundle::new("bundle1", "Test Bundle", "admin");
        bundle.add_user("user1");
        manager.create_bundle(bundle).await.unwrap();

        // User has access when active
        assert!(manager.can_user_access_tool("user1", &[], "anything").await);

        // Suspend bundle
        manager
            .set_status("bundle1", BundleStatus::Suspended)
            .await
            .unwrap();

        // User no longer has access
        assert!(!manager.can_user_access_tool("user1", &[], "anything").await);
    }
}
