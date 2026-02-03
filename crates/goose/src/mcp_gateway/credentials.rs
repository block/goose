//! MCP Gateway Credential Management
//!
//! Secure credential storage for MCP server authentication.

use super::errors::GatewayError;
use super::permissions::UserContext;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Credential type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialType {
    /// API key authentication
    ApiKey,
    /// Bearer token authentication
    BearerToken,
    /// Basic authentication
    BasicAuth { username: String },
    /// OAuth2 client credentials
    OAuth2 { client_id: String },
    /// Custom authentication type
    Custom { name: String },
}

/// Credential scope
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum CredentialScope {
    /// Shared across organization
    Organization,
    /// Per-user credentials
    User { user_id: String },
    /// Per-session (temporary)
    Session { session_id: String },
}

impl CredentialScope {
    /// Create organization scope
    pub fn organization() -> Self {
        CredentialScope::Organization
    }

    /// Create user scope
    pub fn user(user_id: impl Into<String>) -> Self {
        CredentialScope::User {
            user_id: user_id.into(),
        }
    }

    /// Create session scope
    pub fn session(session_id: impl Into<String>) -> Self {
        CredentialScope::Session {
            session_id: session_id.into(),
        }
    }
}

/// Credentials for MCP server authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    /// Credential type
    pub credential_type: CredentialType,
    /// The secret value (encrypted in storage)
    #[serde(skip_serializing)]
    pub value: String,
    /// When the credential expires
    pub expires_at: Option<DateTime<Utc>>,
    /// When the credential was created
    pub created_at: DateTime<Utc>,
    /// When the credential was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Credentials {
    /// Create new API key credentials
    pub fn api_key(value: impl Into<String>) -> Self {
        Self {
            credential_type: CredentialType::ApiKey,
            value: value.into(),
            expires_at: None,
            created_at: Utc::now(),
            last_used_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Create new bearer token credentials
    pub fn bearer_token(value: impl Into<String>) -> Self {
        Self {
            credential_type: CredentialType::BearerToken,
            value: value.into(),
            expires_at: None,
            created_at: Utc::now(),
            last_used_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Create new basic auth credentials
    pub fn basic_auth(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            credential_type: CredentialType::BasicAuth {
                username: username.into(),
            },
            value: password.into(),
            expires_at: None,
            created_at: Utc::now(),
            last_used_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Set expiration
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if credentials are expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Get authorization header value
    pub fn to_header_value(&self) -> String {
        match &self.credential_type {
            CredentialType::ApiKey => self.value.clone(),
            CredentialType::BearerToken => format!("Bearer {}", self.value),
            CredentialType::BasicAuth { username } => {
                let encoded = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    format!("{}:{}", username, self.value),
                );
                format!("Basic {}", encoded)
            }
            CredentialType::OAuth2 { .. } => format!("Bearer {}", self.value),
            CredentialType::Custom { .. } => self.value.clone(),
        }
    }
}

/// Storage key for credentials (for potential future use with keyring)
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CredentialKey {
    server_id: String,
    scope: CredentialScope,
}

/// Credential storage trait
#[async_trait]
pub trait CredentialStore: Send + Sync {
    /// Get credentials for a server
    async fn get_credentials(
        &self,
        server_id: &str,
        user_context: &UserContext,
    ) -> Result<Option<Credentials>, GatewayError>;

    /// Store credentials
    async fn store_credentials(
        &self,
        server_id: &str,
        credentials: Credentials,
        scope: CredentialScope,
    ) -> Result<(), GatewayError>;

    /// Delete credentials
    async fn delete_credentials(
        &self,
        server_id: &str,
        scope: CredentialScope,
    ) -> Result<bool, GatewayError>;

    /// Rotate credentials (generate new, store, return new credentials)
    async fn rotate_credentials(
        &self,
        server_id: &str,
        scope: CredentialScope,
    ) -> Result<Credentials, GatewayError>;

    /// Update last used timestamp
    async fn mark_used(&self, server_id: &str, scope: CredentialScope) -> Result<(), GatewayError>;
}

/// In-memory credential store (for testing and development)
pub struct MemoryCredentialStore {
    credentials: RwLock<HashMap<(String, CredentialScope), Credentials>>,
}

impl MemoryCredentialStore {
    /// Create a new in-memory credential store
    pub fn new() -> Self {
        Self {
            credentials: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CredentialStore for MemoryCredentialStore {
    async fn get_credentials(
        &self,
        server_id: &str,
        user_context: &UserContext,
    ) -> Result<Option<Credentials>, GatewayError> {
        let credentials = self.credentials.read().await;

        // Try user-specific first
        let user_key = (
            server_id.to_string(),
            CredentialScope::User {
                user_id: user_context.user_id.clone(),
            },
        );
        if let Some(creds) = credentials.get(&user_key) {
            if !creds.is_expired() {
                return Ok(Some(creds.clone()));
            }
        }

        // Try session-specific
        if let Some(session_id) = &user_context.session_id {
            let session_key = (
                server_id.to_string(),
                CredentialScope::Session {
                    session_id: session_id.clone(),
                },
            );
            if let Some(creds) = credentials.get(&session_key) {
                if !creds.is_expired() {
                    return Ok(Some(creds.clone()));
                }
            }
        }

        // Fall back to org-shared
        let org_key = (server_id.to_string(), CredentialScope::Organization);
        if let Some(creds) = credentials.get(&org_key) {
            if !creds.is_expired() {
                return Ok(Some(creds.clone()));
            }
        }

        Ok(None)
    }

    async fn store_credentials(
        &self,
        server_id: &str,
        credentials: Credentials,
        scope: CredentialScope,
    ) -> Result<(), GatewayError> {
        let mut store = self.credentials.write().await;
        store.insert((server_id.to_string(), scope), credentials);
        Ok(())
    }

    async fn delete_credentials(
        &self,
        server_id: &str,
        scope: CredentialScope,
    ) -> Result<bool, GatewayError> {
        let mut store = self.credentials.write().await;
        Ok(store.remove(&(server_id.to_string(), scope)).is_some())
    }

    async fn rotate_credentials(
        &self,
        _server_id: &str,
        _scope: CredentialScope,
    ) -> Result<Credentials, GatewayError> {
        // In-memory store doesn't support rotation
        Err(GatewayError::Internal(
            "Credential rotation not supported in memory store".to_string(),
        ))
    }

    async fn mark_used(&self, server_id: &str, scope: CredentialScope) -> Result<(), GatewayError> {
        let mut store = self.credentials.write().await;
        if let Some(creds) = store.get_mut(&(server_id.to_string(), scope)) {
            creds.last_used_at = Some(Utc::now());
        }
        Ok(())
    }
}

/// Credential manager that wraps a credential store
pub struct CredentialManager {
    store: Arc<dyn CredentialStore>,
}

impl CredentialManager {
    /// Create a new credential manager
    pub fn new(store: Arc<dyn CredentialStore>) -> Self {
        Self { store }
    }

    /// Create with in-memory store (for testing)
    pub fn memory() -> Self {
        Self {
            store: Arc::new(MemoryCredentialStore::new()),
        }
    }

    /// Get credentials for a server
    pub async fn get_credentials(
        &self,
        server_id: &str,
        user_context: &UserContext,
    ) -> Result<Credentials, GatewayError> {
        let creds = self
            .store
            .get_credentials(server_id, user_context)
            .await?
            .ok_or_else(|| GatewayError::CredentialNotFound {
                server_id: server_id.to_string(),
            })?;

        if creds.is_expired() {
            return Err(GatewayError::CredentialExpired {
                server_id: server_id.to_string(),
            });
        }

        Ok(creds)
    }

    /// Store credentials
    pub async fn store_credentials(
        &self,
        server_id: &str,
        credentials: Credentials,
        scope: CredentialScope,
    ) -> Result<(), GatewayError> {
        self.store
            .store_credentials(server_id, credentials, scope)
            .await
    }

    /// Delete credentials
    pub async fn delete_credentials(
        &self,
        server_id: &str,
        scope: CredentialScope,
    ) -> Result<bool, GatewayError> {
        self.store.delete_credentials(server_id, scope).await
    }

    /// Check if credentials exist for a server
    pub async fn has_credentials(&self, server_id: &str, user_context: &UserContext) -> bool {
        self.store
            .get_credentials(server_id, user_context)
            .await
            .ok()
            .flatten()
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store_basic() {
        let store = MemoryCredentialStore::new();
        let user_context = UserContext::new("user1");

        // Store credentials
        let creds = Credentials::api_key("test-key");
        store
            .store_credentials("server1", creds, CredentialScope::Organization)
            .await
            .unwrap();

        // Retrieve credentials
        let retrieved = store
            .get_credentials("server1", &user_context)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.value, "test-key");
    }

    #[tokio::test]
    async fn test_memory_store_user_precedence() {
        let store = MemoryCredentialStore::new();
        let user_context = UserContext::new("user1");

        // Store org-level credentials
        let org_creds = Credentials::api_key("org-key");
        store
            .store_credentials("server1", org_creds, CredentialScope::Organization)
            .await
            .unwrap();

        // Store user-level credentials
        let user_creds = Credentials::api_key("user-key");
        store
            .store_credentials(
                "server1",
                user_creds,
                CredentialScope::User {
                    user_id: "user1".to_string(),
                },
            )
            .await
            .unwrap();

        // User-level should take precedence
        let retrieved = store
            .get_credentials("server1", &user_context)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.value, "user-key");
    }

    #[tokio::test]
    async fn test_memory_store_expired() {
        let store = MemoryCredentialStore::new();
        let user_context = UserContext::new("user1");

        // Store expired credentials
        let creds = Credentials::api_key("test-key")
            .with_expiration(Utc::now() - chrono::Duration::hours(1));
        store
            .store_credentials("server1", creds, CredentialScope::Organization)
            .await
            .unwrap();

        // Should not return expired credentials
        let retrieved = store
            .get_credentials("server1", &user_context)
            .await
            .unwrap();

        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_credential_manager() {
        let manager = CredentialManager::memory();
        let user_context = UserContext::new("user1");

        // Store credentials
        let creds = Credentials::api_key("test-key");
        manager
            .store_credentials("server1", creds, CredentialScope::Organization)
            .await
            .unwrap();

        // Get credentials
        let retrieved = manager
            .get_credentials("server1", &user_context)
            .await
            .unwrap();
        assert_eq!(retrieved.value, "test-key");
    }

    #[tokio::test]
    async fn test_credential_manager_not_found() {
        let manager = CredentialManager::memory();
        let user_context = UserContext::new("user1");

        let result = manager.get_credentials("nonexistent", &user_context).await;
        assert!(matches!(
            result,
            Err(GatewayError::CredentialNotFound { .. })
        ));
    }

    #[test]
    fn test_credential_header_value() {
        let api_key = Credentials::api_key("test-key");
        assert_eq!(api_key.to_header_value(), "test-key");

        let bearer = Credentials::bearer_token("my-token");
        assert_eq!(bearer.to_header_value(), "Bearer my-token");

        let basic = Credentials::basic_auth("user", "pass");
        assert!(basic.to_header_value().starts_with("Basic "));
    }

    #[test]
    fn test_credential_expiration() {
        let not_expired =
            Credentials::api_key("key").with_expiration(Utc::now() + chrono::Duration::hours(1));
        assert!(!not_expired.is_expired());

        let expired =
            Credentials::api_key("key").with_expiration(Utc::now() - chrono::Duration::hours(1));
        assert!(expired.is_expired());

        let no_expiry = Credentials::api_key("key");
        assert!(!no_expiry.is_expired());
    }
}
