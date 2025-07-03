use etcetera::AppStrategy;
use goose_secure_store::{FileBackedStore, KeyringSecureStore, SecretError, SecureStore};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Secret storage error: {0}")]
    SecretError(#[from] SecretError),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("No credentials found")]
    NotFound,
}

/// Simplified credentials manager using secure store
pub struct CredentialsManager {
    store: Box<dyn SecureStore>,
    service: String,
    username: String,
}

impl CredentialsManager {
    pub fn new(
        _credentials_path: String, // Ignored - kept for API compatibility
        fallback_to_disk: bool,
        _keychain_service: String,  // Ignored - use standard naming
        _keychain_username: String, // Ignored - use standard naming
    ) -> Self {
        let store: Box<dyn SecureStore> = if fallback_to_disk {
            // Use file fallback - but store in standard location
            let config_dir = etcetera::choose_app_strategy(etcetera::AppStrategyArgs {
                top_level_domain: "Block".to_string(),
                author: "Block".to_string(),
                app_name: "goose".to_string(),
            })
            .expect("Failed to get config dir")
            .config_dir();
            let fallback_path = config_dir.join("google_drive_credentials.yaml");
            Box::new(FileBackedStore::new(fallback_path))
        } else {
            Box::new(KeyringSecureStore::new())
        };

        Self {
            store,
            service: "goose.mcp.google_drive".to_string(), // Use standard naming
            username: "oauth_credentials".to_string(),
        }
    }

    pub fn read_credentials<T: DeserializeOwned>(&self) -> Result<T, StorageError> {
        let json_str = self
            .store
            .get_secret(&self.service, &self.username)
            .map_err(|e| match e {
                SecretError::NotFound(_) => StorageError::NotFound,
                _ => StorageError::SecretError(e),
            })?;

        serde_json::from_str(&json_str).map_err(StorageError::SerializationError)
    }

    pub fn write_credentials<T: Serialize>(&self, content: &T) -> Result<(), StorageError> {
        let json_str = serde_json::to_string(content)?;
        self.store
            .set_secret(&self.service, &self.username, &json_str)
            .map_err(StorageError::SecretError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestCredentials {
        access_token: String,
        refresh_token: String,
        expiry: u64,
    }

    impl TestCredentials {
        fn new() -> Self {
            Self {
                access_token: "test_access_token".to_string(),
                refresh_token: "test_refresh_token".to_string(),
                expiry: 1672531200,
            }
        }
    }

    #[test]
    fn test_read_write_from_keychain() {
        // Create a temporary directory for test files
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let cred_path = temp_dir.path().join("test_credentials.json");
        let cred_path_str = cred_path.to_str().unwrap().to_string();

        // Create a credentials manager with fallback enabled
        // Using a unique service name to ensure keychain operation fails
        let manager = CredentialsManager::new(
            cred_path_str,
            true, // fallback to disk
            "test_service".to_string(),
            "test_user".to_string(),
        );

        // Test credentials to store
        let creds = TestCredentials::new();

        // Write should write to keychain
        let write_result = manager.write_credentials(&creds);
        assert!(write_result.is_ok(), "Write should succeed with fallback");

        // Read should read from keychain
        let read_result = manager.read_credentials::<TestCredentials>();
        assert!(read_result.is_ok(), "Read should succeed with fallback");

        // Verify the read credentials match what we wrote
        assert_eq!(
            read_result.unwrap(),
            creds,
            "Read credentials should match written credentials"
        );
    }

    #[test]
    fn test_no_fallback_not_found() {
        // Create a temporary directory for test files
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let cred_path = temp_dir.path().join("nonexistent_credentials.json");
        let cred_path_str = cred_path.to_str().unwrap().to_string();

        // Create a credentials manager with fallback disabled
        let manager = CredentialsManager::new(
            cred_path_str,
            false, // no fallback to disk
            "test_service_that_should_not_exist".to_string(),
            "test_user_no_fallback".to_string(),
        );

        // Read should fail with NotFound or KeyringError depending on the system
        let read_result = manager.read_credentials::<TestCredentials>();
        println!("{:?}", read_result);
        assert!(
            read_result.is_err(),
            "Read should fail when credentials don't exist"
        );
    }

    #[test]
    fn test_serialization_error() {
        // This test verifies that serialization errors are properly handled
        let error = serde_json::from_str::<TestCredentials>("invalid json").unwrap_err();
        let storage_error = StorageError::SerializationError(error);
        assert!(matches!(storage_error, StorageError::SerializationError(_)));
    }

    #[test]
    fn test_write_read_credentials() {
        // Test basic write and read functionality with file fallback
        let manager = CredentialsManager::new(
            "/tmp/test_credentials".to_string(),
            true, // Enable file fallback
            "test_service".to_string(),
            "test_user".to_string(),
        );

        // Create test credentials
        let creds = TestCredentials::new();

        // Write and read credentials
        let write_result = manager.write_credentials(&creds);
        assert!(write_result.is_ok(), "Write should succeed");

        let read_result = manager.read_credentials::<TestCredentials>();
        assert!(read_result.is_ok(), "Read should succeed");

        let read_creds = read_result.unwrap();
        assert_eq!(read_creds, creds, "Credentials should match");
    }
}
