use anyhow::Result;
use goose::keyring::{KeyringBackend, KeyringError};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, warn};

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Failed to access keyring: {0}")]
    KeyringError(String),
    #[error("Failed to access file system: {0}")]
    FileSystemError(#[from] std::io::Error),
    #[error("No credentials found")]
    NotFound,
    #[error("Critical error: {0}")]
    Critical(String),
    #[error("Failed to serialize/deserialize: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl From<KeyringError> for StorageError {
    fn from(err: KeyringError) -> Self {
        match err {
            KeyringError::NotFound { .. } => StorageError::NotFound,
            _ => StorageError::KeyringError(err.to_string()),
        }
    }
}

/// CredentialsManager handles secure storage of OAuth credentials.
/// It attempts to store credentials in the system keychain first,
/// with fallback to file system storage if keychain access fails and fallback is enabled.
pub struct CredentialsManager {
    credentials_path: String,
    fallback_to_disk: bool,
    keychain_service: String,
    keychain_username: String,
    keyring: Arc<dyn KeyringBackend>,
}

impl CredentialsManager {
    pub fn new(
        credentials_path: String,
        fallback_to_disk: bool,
        keychain_service: String,
        keychain_username: String,
        keyring: Arc<dyn KeyringBackend>,
    ) -> Self {
        Self {
            credentials_path,
            fallback_to_disk,
            keychain_service,
            keychain_username,
            keyring,
        }
    }

    /// Reads and deserializes credentials from secure storage.
    ///
    /// This method attempts to read credentials from the system keychain first.
    /// If keychain access fails and fallback is enabled, it will try to read from the file system.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type to deserialize to. Must implement `serde::DeserializeOwned`.
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - The deserialized data
    /// * `Err(StorageError)` - If reading or deserialization fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use goose_mcp::google_drive::storage::CredentialsManager;
    /// # use goose::keyring::SystemKeyringBackend;
    /// # use std::sync::Arc;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct OAuthToken {
    ///     access_token: String,
    ///     refresh_token: String,
    ///     expiry: u64,
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let keyring = Arc::new(SystemKeyringBackend);
    /// let manager = CredentialsManager::new(
    ///     String::from("/path/to/credentials.json"),
    ///     true,  // fallback to disk if keychain fails
    ///     String::from("test_service"),
    ///     String::from("test_user"),
    ///     keyring
    /// );
    /// match manager.read_credentials::<OAuthToken>() {
    ///     Ok(token) => println!("Access token: {}", token.access_token),
    ///     Err(e) => eprintln!("Failed to read token: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_credentials<T>(&self) -> Result<T, StorageError>
    where
        T: DeserializeOwned,
    {
        let json_str = self
            .keyring
            .get_password(&self.keychain_service, &self.keychain_username)
            .inspect(|_| {
                debug!("Successfully read credentials from keychain");
            })
            .or_else(|e| {
                if self.fallback_to_disk {
                    warn!("Falling back to file system due to keyring error: {}", e);
                    self.read_from_file()
                } else {
                    // Convert anyhow::Error back to our error type
                    if let Some(keyring_err) = e.downcast_ref::<KeyringError>() {
                        match keyring_err {
                            KeyringError::NotFound { .. } => Err(StorageError::NotFound),
                            _ => Err(StorageError::KeyringError(e.to_string())),
                        }
                    } else {
                        Err(StorageError::KeyringError(e.to_string()))
                    }
                }
            })?;

        serde_json::from_str(&json_str).map_err(StorageError::SerializationError)
    }

    fn read_from_file(&self) -> Result<String, StorageError> {
        let path = Path::new(&self.credentials_path);
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    debug!("Successfully read credentials from file system");
                    Ok(content)
                }
                Err(e) => {
                    error!("Failed to read credentials file: {}", e);
                    Err(StorageError::FileSystemError(e))
                }
            }
        } else {
            debug!("No credentials found in file system");
            Err(StorageError::NotFound)
        }
    }

    /// Serializes and writes credentials to secure storage.
    ///
    /// This method attempts to write credentials to the system keychain first.
    /// If keychain access fails and fallback is enabled, it will try to write to the file system.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type to serialize. Must implement `serde::Serialize`.
    ///
    /// # Parameters
    ///
    /// * `content` - The data to serialize and store
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If writing succeeds
    /// * `Err(StorageError)` - If serialization or writing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use goose_mcp::google_drive::storage::CredentialsManager;
    /// # use goose::keyring::SystemKeyringBackend;
    /// # use std::sync::Arc;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct OAuthToken {
    ///     access_token: String,
    ///     refresh_token: String,
    ///     expiry: u64,
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let token = OAuthToken {
    ///     access_token: String::from("access_token_value"),
    ///     refresh_token: String::from("refresh_token_value"),
    ///     expiry: 1672531200, // Unix timestamp
    /// };
    ///
    /// let keyring = Arc::new(SystemKeyringBackend);
    /// let manager = CredentialsManager::new(
    ///     String::from("/path/to/credentials.json"),
    ///     true,  // fallback to disk if keychain fails
    ///     String::from("test_service"),
    ///     String::from("test_user"),
    ///     keyring
    /// );
    /// if let Err(e) = manager.write_credentials(&token) {
    ///     eprintln!("Failed to write token: {}", e);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn write_credentials<T>(&self, content: &T) -> Result<(), StorageError>
    where
        T: Serialize,
    {
        let json_str = serde_json::to_string(content).map_err(StorageError::SerializationError)?;

        self.keyring
            .set_password(&self.keychain_service, &self.keychain_username, &json_str)
            .inspect(|_| {
                debug!("Successfully wrote credentials to keychain");
            })
            .or_else(|e| {
                if self.fallback_to_disk {
                    warn!("Falling back to file system due to keyring error: {}", e);
                    self.write_to_file(&json_str)
                } else {
                    Err(StorageError::KeyringError(e.to_string()))
                }
            })
    }

    fn write_to_file(&self, content: &str) -> Result<(), StorageError> {
        let path = Path::new(&self.credentials_path);

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                match fs::create_dir_all(parent) {
                    Ok(_) => debug!("Created parent directories for credentials file"),
                    Err(e) => {
                        error!("Failed to create directories for credentials file: {}", e);
                        return Err(StorageError::FileSystemError(e));
                    }
                }
            }
        }

        match fs::write(path, content) {
            Ok(_) => {
                debug!("Successfully wrote credentials to file system");
                Ok(())
            }
            Err(e) => {
                error!("Failed to write credentials to file system: {}", e);
                Err(StorageError::FileSystemError(e))
            }
        }
    }
}

impl Clone for CredentialsManager {
    fn clone(&self) -> Self {
        Self {
            credentials_path: self.credentials_path.clone(),
            fallback_to_disk: self.fallback_to_disk,
            keychain_service: self.keychain_service.clone(),
            keychain_username: self.keychain_username.clone(),
            keyring: self.keyring.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::keyring::MockKeyringBackend;
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

        // Create a mock keyring backend
        let keyring = Arc::new(MockKeyringBackend::new());

        // Create a credentials manager with fallback enabled
        let manager = CredentialsManager::new(
            cred_path_str,
            true, // fallback to disk
            "test_service".to_string(),
            "test_user".to_string(),
            keyring,
        );

        // Test credentials to store
        let creds = TestCredentials::new();

        // Write should succeed with mock keyring
        let write_result = manager.write_credentials(&creds);
        assert!(
            write_result.is_ok(),
            "Write should succeed with mock keyring"
        );

        // Read should succeed with mock keyring
        let read_result = manager.read_credentials::<TestCredentials>();
        assert!(read_result.is_ok(), "Read should succeed with mock keyring");

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

        // Create a mock keyring backend (empty by default)
        let keyring = Arc::new(MockKeyringBackend::new());

        // Create a credentials manager with fallback disabled
        let manager = CredentialsManager::new(
            cred_path_str,
            false, // no fallback to disk
            "test_service_that_should_not_exist".to_string(),
            "test_user_no_fallback".to_string(),
            keyring,
        );

        // Read should fail with NotFound since mock keyring is empty and no fallback
        let read_result = manager.read_credentials::<TestCredentials>();
        println!("{:?}", read_result);
        assert!(
            read_result.is_err(),
            "Read should fail when credentials don't exist"
        );
        assert!(
            matches!(read_result.unwrap_err(), StorageError::NotFound),
            "Should return NotFound error"
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
    fn test_file_system_error_handling() {
        // Test handling of file system errors by using an invalid path
        let invalid_path = String::from("/nonexistent_directory/credentials.json");
        let keyring = Arc::new(MockKeyringBackend::new());
        let manager = CredentialsManager::new(
            invalid_path,
            true,
            "test_service".to_string(),
            "test_user".to_string(),
            keyring,
        );

        // Create test credentials
        let creds = TestCredentials::new();

        // Attempt to write to an invalid path should result in FileSystemError
        let result = manager.write_to_file(&serde_json::to_string(&creds).unwrap());
        assert!(matches!(result, Err(StorageError::FileSystemError(_))));
    }
}
