use crate::{Result, SecretError};
use keyring::Entry;

#[cfg(test)]
use std::collections::HashMap;

/// Trait for secure storage backends
pub trait SecureStore {
    /// Store a secret in the secure store
    /// 
    /// # Arguments
    /// * `service` - The service identifier (e.g., "goose.mcp.github")
    /// * `username` - The username/key identifier
    /// * `secret` - The secret value to store
    fn set_secret(&self, service: &str, username: &str, secret: &str) -> Result<()>;

    /// Retrieve a secret from the secure store
    /// 
    /// # Arguments
    /// * `service` - The service identifier
    /// * `username` - The username/key identifier
    fn get_secret(&self, service: &str, username: &str) -> Result<String>;

    /// Delete a secret from the secure store
    /// 
    /// # Arguments
    /// * `service` - The service identifier
    /// * `username` - The username/key identifier
    fn delete_secret(&self, service: &str, username: &str) -> Result<()>;

    /// Check if a secret exists in the secure store
    /// 
    /// # Arguments
    /// * `service` - The service identifier
    /// * `username` - The username/key identifier
    fn has_secret(&self, service: &str, username: &str) -> bool;
}

/// Implementation of SecureStore using the system keyring
pub struct KeyringSecureStore;

impl KeyringSecureStore {
    /// Create a new KeyringSecureStore instance
    pub fn new() -> Self {
        Self
    }

    /// Create a namespaced service identifier for MCP servers
    /// 
    /// # Arguments
    /// * `server_name` - The name of the MCP server
    /// * `secret_name` - The name of the secret (optional)
    pub fn create_service_name(server_name: &str, secret_name: Option<&str>) -> String {
        match secret_name {
            Some(name) => format!("goose.mcp.{}.{}", server_name, name),
            None => format!("goose.mcp.{}", server_name),
        }
    }
}

impl Default for KeyringSecureStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStore for KeyringSecureStore {
    fn set_secret(&self, service: &str, username: &str, secret: &str) -> Result<()> {
        let entry = Entry::new(service, username)
            .map_err(|e| SecretError::StorageFailure(format!("Failed to create keyring entry: {}", e)))?;
        
        entry.set_password(secret)?;
        Ok(())
    }

    fn get_secret(&self, service: &str, username: &str) -> Result<String> {
        let entry = Entry::new(service, username)
            .map_err(|e| SecretError::StorageFailure(format!("Failed to create keyring entry: {}", e)))?;
        
        let password = entry.get_password()?;
        Ok(password)
    }

    fn delete_secret(&self, service: &str, username: &str) -> Result<()> {
        let entry = Entry::new(service, username)
            .map_err(|e| SecretError::StorageFailure(format!("Failed to create keyring entry: {}", e)))?;
        
        entry.delete_credential()?;
        Ok(())
    }

    fn has_secret(&self, service: &str, username: &str) -> bool {
        match Entry::new(service, username) {
            Ok(entry) => entry.get_password().is_ok(),
            Err(_) => false,
        }
    }
}

/// Mock implementation for testing
#[cfg(test)]
pub struct MockSecureStore {
    storage: std::sync::Mutex<HashMap<String, String>>,
}

#[cfg(test)]
impl MockSecureStore {
    pub fn new() -> Self {
        Self {
            storage: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn make_key(service: &str, username: &str) -> String {
        format!("{}:{}", service, username)
    }
}

#[cfg(test)]
impl Default for MockSecureStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl SecureStore for MockSecureStore {
    fn set_secret(&self, service: &str, username: &str, secret: &str) -> Result<()> {
        let key = Self::make_key(service, username);
        let mut storage = self.storage.lock().unwrap();
        storage.insert(key, secret.to_string());
        Ok(())
    }

    fn get_secret(&self, service: &str, username: &str) -> Result<String> {
        let key = Self::make_key(service, username);
        let storage = self.storage.lock().unwrap();
        storage
            .get(&key)
            .cloned()
            .ok_or_else(|| SecretError::NotFound(format!("Secret not found: {}", key)))
    }

    fn delete_secret(&self, service: &str, username: &str) -> Result<()> {
        let key = Self::make_key(service, username);
        let mut storage = self.storage.lock().unwrap();
        storage
            .remove(&key)
            .ok_or_else(|| SecretError::NotFound(format!("Secret not found: {}", key)))?;
        Ok(())
    }

    fn has_secret(&self, service: &str, username: &str) -> bool {
        let key = Self::make_key(service, username);
        let storage = self.storage.lock().unwrap();
        storage.contains_key(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_service_name() {
        assert_eq!(
            KeyringSecureStore::create_service_name("github", Some("api_key")),
            "goose.mcp.github.api_key"
        );
        assert_eq!(
            KeyringSecureStore::create_service_name("github", None),
            "goose.mcp.github"
        );
    }

    #[test]
    fn test_mock_store_basic_operations() {
        let store = MockSecureStore::new();
        let service = "test.service";
        let username = "test_user";
        let secret = "test_secret";

        // Test setting and getting
        assert!(store.set_secret(service, username, secret).is_ok());
        assert!(store.has_secret(service, username));
        
        let retrieved = store.get_secret(service, username).unwrap();
        assert_eq!(retrieved, secret);

        // Test deletion
        assert!(store.delete_secret(service, username).is_ok());
        assert!(!store.has_secret(service, username));
        
        // Test getting non-existent secret
        assert!(store.get_secret(service, username).is_err());
    }

    #[test]
    fn test_mock_store_namespacing() {
        let store = MockSecureStore::new();
        
        // Store secrets for different servers
        store.set_secret("goose.mcp.github", "api_key", "github_secret").unwrap();
        store.set_secret("goose.mcp.jira", "api_key", "jira_secret").unwrap();
        
        // Verify they don't interfere with each other
        assert_eq!(
            store.get_secret("goose.mcp.github", "api_key").unwrap(),
            "github_secret"
        );
        assert_eq!(
            store.get_secret("goose.mcp.jira", "api_key").unwrap(),
            "jira_secret"
        );
    }
}
