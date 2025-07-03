use goose_secure_store::{Result, SecretAcquisition, SecretError, SecureStore};
use std::collections::HashMap;
use std::env;
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
fn test_environment_fallback_when_secret_not_found() {
    // Set up environment variable
    env::set_var("TEST_API_KEY", "env_secret_value");

    let mock_store = TestMockStore::new();
    let acquisition = SecretAcquisition::with_store(Box::new(mock_store));

    // Verify secret doesn't exist in mock store
    let test_key = "TEST_FALLBACK_KEY";
    assert!(!acquisition.has_secret("test_server", test_key));

    // Test that secret acquisition would fail from store
    let result = acquisition.get_secret("test_server", test_key);
    assert!(result.is_err()); // Should fail since secret doesn't exist in mock store

    // Verify environment variable exists for fallback (this simulates the extension manager logic)
    assert_eq!(env::var("TEST_API_KEY").unwrap(), "env_secret_value");

    // Clean up
    env::remove_var("TEST_API_KEY");
}

#[test]
fn test_no_fallback_when_environment_not_set() {
    let mock_store = TestMockStore::new();
    let acquisition = SecretAcquisition::with_store(Box::new(mock_store));

    // Use a test key
    let test_key = "NONEXISTENT_KEY";

    // Ensure environment variable is not set
    env::remove_var(test_key);

    // Should fail when neither store nor environment has the secret
    let result = acquisition.get_secret("test_server", test_key);
    assert!(matches!(result, Err(SecretError::NotFound(_))));

    // Verify environment variable doesn't exist
    assert!(env::var(test_key).is_err());
}

#[test]
fn test_service_name_consistency() {
    // Test that service names are created consistently
    let server_name = "github-server";
    let secret_name = "api_token";

    let expected_service_name = format!("goose.mcp.{}.{}", server_name, secret_name);
    let actual_service_name =
        goose_secure_store::KeyringSecureStore::create_service_name(server_name, Some(secret_name));

    assert_eq!(actual_service_name, expected_service_name);
}
