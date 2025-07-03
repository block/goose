use crate::{Result, SecretError};
use keyring::Entry;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

/// Trait for secure storage backends
pub trait SecureStore: Send + Sync {
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

    /// Create a new SecureStore with file fallback support
    pub fn with_file_fallback(fallback_path: Option<PathBuf>) -> Box<dyn SecureStore> {
        if let Some(path) = fallback_path {
            Box::new(FileBackedStore::new(path))
        } else {
            Box::new(KeyringSecureStore::new())
        }
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
        let entry = Entry::new(service, username).map_err(|e| {
            SecretError::StorageFailure(format!("Failed to create keyring entry: {}", e))
        })?;

        entry.set_password(secret)?;
        Ok(())
    }

    fn get_secret(&self, service: &str, username: &str) -> Result<String> {
        let entry = Entry::new(service, username).map_err(|e| {
            SecretError::StorageFailure(format!("Failed to create keyring entry: {}", e))
        })?;

        let password = entry.get_password()?;
        Ok(password)
    }

    fn delete_secret(&self, service: &str, username: &str) -> Result<()> {
        let entry = Entry::new(service, username).map_err(|e| {
            SecretError::StorageFailure(format!("Failed to create keyring entry: {}", e))
        })?;

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

/// File-backed store implementation for when keyring is unavailable
pub struct FileBackedStore {
    file_path: PathBuf,
}

impl FileBackedStore {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    fn load_secrets(&self) -> Result<HashMap<String, String>> {
        if self.file_path.exists() {
            let content = std::fs::read_to_string(&self.file_path).map_err(|e| {
                SecretError::StorageFailure(format!("Failed to read secrets file: {}", e))
            })?;
            let secrets: HashMap<String, String> = serde_yaml::from_str(&content)
                .map_err(|e| SecretError::StorageFailure(format!("YAML parse error: {}", e)))?;
            Ok(secrets)
        } else {
            Ok(HashMap::new())
        }
    }

    fn save_secrets(&self, secrets: &HashMap<String, String>) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SecretError::StorageFailure(format!("Failed to create secrets directory: {}", e))
            })?;
        }
        let content = serde_yaml::to_string(secrets)
            .map_err(|e| SecretError::StorageFailure(format!("YAML serialize error: {}", e)))?;
        std::fs::write(&self.file_path, content).map_err(|e| {
            SecretError::StorageFailure(format!("Failed to write secrets file: {}", e))
        })?;
        Ok(())
    }

    fn make_key(&self, service: &str, username: &str) -> String {
        format!("{}:{}", service, username)
    }
}

impl SecureStore for FileBackedStore {
    fn set_secret(&self, service: &str, username: &str, secret: &str) -> Result<()> {
        let mut secrets = self.load_secrets()?;
        let key = self.make_key(service, username);
        secrets.insert(key, secret.to_string());
        self.save_secrets(&secrets)
    }

    fn get_secret(&self, service: &str, username: &str) -> Result<String> {
        let secrets = self.load_secrets()?;
        let key = self.make_key(service, username);
        secrets
            .get(&key)
            .cloned()
            .ok_or_else(|| SecretError::NotFound(format!("Secret not found: {}", key)))
    }

    fn delete_secret(&self, service: &str, username: &str) -> Result<()> {
        let mut secrets = self.load_secrets()?;
        let key = self.make_key(service, username);
        secrets.remove(&key);
        self.save_secrets(&secrets)
    }

    fn has_secret(&self, service: &str, username: &str) -> bool {
        let key = self.make_key(service, username);
        self.load_secrets()
            .map(|secrets| secrets.contains_key(&key))
            .unwrap_or(false)
    }
}

/// Legacy compatibility wrapper for the config system
pub struct LegacyConfigStore {
    inner: Box<dyn SecureStore>,
}

impl LegacyConfigStore {
    pub fn new() -> Self {
        Self {
            inner: Box::new(KeyringSecureStore::new()),
        }
    }

    pub fn with_file_fallback(fallback_path: Option<PathBuf>) -> Self {
        let store: Box<dyn SecureStore> = if let Some(path) = fallback_path {
            Box::new(FileBackedStore::new(path))
        } else {
            Box::new(KeyringSecureStore::new())
        };

        Self { inner: store }
    }

    pub fn get_secret<T>(&self, key: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let service = "goose.config";
        let secret_str = self.inner.get_secret(service, key)?;
        let value: Value =
            serde_json::from_str(&secret_str).unwrap_or_else(|_| Value::String(secret_str));
        serde_json::from_value(value).map_err(SecretError::SerializationError)
    }

    pub fn set_secret(&self, key: &str, value: Value) -> Result<()> {
        let service = "goose.config";
        let secret_str = serde_json::to_string(&value)?;
        self.inner.set_secret(service, key, &secret_str)
    }

    pub fn delete_secret(&self, key: &str) -> Result<()> {
        let service = "goose.config";
        self.inner.delete_secret(service, key)
    }

    pub fn load_secrets(&self) -> Result<HashMap<String, Value>> {
        // Note: keyring doesn't support enumeration, so we return empty map
        // This maintains compatibility with the existing API but doesn't provide
        // bulk loading capability when using keyring storage
        Ok(HashMap::new())
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
        store
            .set_secret("goose.mcp.github", "api_key", "github_secret")
            .unwrap();
        store
            .set_secret("goose.mcp.jira", "api_key", "jira_secret")
            .unwrap();

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

    #[test]
    fn test_file_backed_store() {
        let temp_dir = std::env::temp_dir().join("goose_test_file_store");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("test_secrets.yaml");

        let store = FileBackedStore::new(file_path.clone());

        // Test basic operations
        assert!(store
            .set_secret("test.service", "test_key", "test_secret")
            .is_ok());
        assert!(store.has_secret("test.service", "test_key"));

        let retrieved = store.get_secret("test.service", "test_key").unwrap();
        assert_eq!(retrieved, "test_secret");

        // Test persistence across instances
        let store2 = FileBackedStore::new(file_path.clone());
        let retrieved2 = store2.get_secret("test.service", "test_key").unwrap();
        assert_eq!(retrieved2, "test_secret");

        // Test deletion
        assert!(store.delete_secret("test.service", "test_key").is_ok());
        assert!(!store.has_secret("test.service", "test_key"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_legacy_config_store() {
        let temp_dir = std::env::temp_dir().join("goose_test_legacy_store");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("legacy_secrets.yaml");

        let store = LegacyConfigStore::with_file_fallback(Some(file_path.clone()));

        // Test setting and getting different value types
        assert!(store
            .set_secret(
                "api_key",
                serde_json::Value::String("secret123".to_string())
            )
            .is_ok());
        assert!(store
            .set_secret("port", serde_json::Value::Number(8080.into()))
            .is_ok());
        assert!(store
            .set_secret("enabled", serde_json::Value::Bool(true))
            .is_ok());

        let api_key: String = store.get_secret("api_key").unwrap();
        assert_eq!(api_key, "secret123");

        let port: u16 = store.get_secret("port").unwrap();
        assert_eq!(port, 8080);

        let enabled: bool = store.get_secret("enabled").unwrap();
        assert_eq!(enabled, true);

        // Test deletion
        assert!(store.delete_secret("api_key").is_ok());
        assert!(store.get_secret::<String>("api_key").is_err());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
