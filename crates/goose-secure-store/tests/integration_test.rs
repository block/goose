use goose_secure_store::{SecretAcquisition, KeyringSecureStore, SecureStore, SecretError, Result};
use std::collections::HashMap;
use std::sync::Mutex;

// Simple mock store for integration tests
struct TestMockStore {
    storage: Mutex<HashMap<String, String>>,
}

impl TestMockStore {
    fn new() -> Self {
        Self {
            storage: Mutex::new(HashMap::new()),
        }
    }

    fn make_key(service: &str, username: &str) -> String {
        format!("{}:{}", service, username)
    }
}

impl SecureStore for TestMockStore {
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

#[test]
fn test_secret_acquisition_basic() {
    let mock_store = TestMockStore::new();
    let acquisition = SecretAcquisition::with_store(Box::new(mock_store));
    
    // Test that we can check for non-existent secrets
    assert!(!acquisition.has_secret("test_server", "api_key"));
}

#[test]
fn test_service_name_creation() {
    assert_eq!(
        KeyringSecureStore::create_service_name("github", Some("token")),
        "goose.mcp.github.token"
    );
    
    assert_eq!(
        KeyringSecureStore::create_service_name("jira", None),
        "goose.mcp.jira"
    );
}
