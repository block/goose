use anyhow::Result;
use google_drive3::yup_oauth2::storage::{TokenInfo, TokenStorage};
use keyring::Entry;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, warn};

const KEYCHAIN_SERVICE: &str = "mcp_google_drive";
const KEYCHAIN_USERNAME: &str = "oauth_credentials";
const KEYCHAIN_DISK_FALLBACK_ENV: &str = "GOOGLE_DRIVE_DISK_FALLBACK";

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Failed to access keychain: {0}")]
    KeyringError(#[from] keyring::Error),
    #[error("Failed to access file system: {0}")]
    FileSystemError(#[from] std::io::Error),
    #[error("No credentials found")]
    NotFound,
    #[error("Critical error: {0}")]
    Critical(String),
    #[error("Failed to serialize/deserialize: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// CredentialsManager handles secure storage of OAuth credentials.
/// It attempts to store credentials in the system keychain first,
/// with fallback to file system storage if keychain access fails and fallback is enabled.
pub struct CredentialsManager {
    credentials_path: String,
    fallback_to_disk: bool,
}

impl CredentialsManager {
    pub fn new(credentials_path: String) -> Self {
        // Check if we should fall back to disk, must be explicitly enabled
        let fallback_to_disk = match env::var(KEYCHAIN_DISK_FALLBACK_ENV) {
            Ok(value) => value.to_lowercase() == "true",
            Err(_) => false,
        };

        Self {
            credentials_path,
            fallback_to_disk,
        }
    }

    pub fn read_credentials(&self) -> Result<String, AuthError> {
        Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USERNAME)
            .and_then(|entry| entry.get_password())
            .inspect(|_| {
                debug!("Successfully read credentials from keychain");
            })
            .or_else(|e| {
                if self.fallback_to_disk {
                    debug!("Falling back to file system due to keyring error: {}", e);
                    self.read_from_file()
                } else {
                    match e {
                        keyring::Error::NoEntry => Err(AuthError::NotFound),
                        _ => Err(AuthError::KeyringError(e)),
                    }
                }
            })
    }

    fn read_from_file(&self) -> Result<String, AuthError> {
        let path = Path::new(&self.credentials_path);
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    debug!("Successfully read credentials from file system");
                    Ok(content)
                }
                Err(e) => {
                    error!("Failed to read credentials file: {}", e);
                    Err(AuthError::FileSystemError(e))
                }
            }
        } else {
            debug!("No credentials found in file system");
            Err(AuthError::NotFound)
        }
    }

    pub fn write_credentials(&self, content: &str) -> Result<(), AuthError> {
        Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USERNAME)
            .and_then(|entry| entry.set_password(content))
            .inspect(|_| {
                debug!("Successfully wrote credentials to keychain");
            })
            .or_else(|e| {
                if self.fallback_to_disk {
                    warn!("Falling back to file system due to keyring error: {}", e);
                    self.write_to_file(content)
                } else {
                    Err(AuthError::KeyringError(e))
                }
            })
    }

    fn write_to_file(&self, content: &str) -> Result<(), AuthError> {
        let path = Path::new(&self.credentials_path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                match fs::create_dir_all(parent) {
                    Ok(_) => debug!("Created parent directories for credentials file"),
                    Err(e) => {
                        error!("Failed to create directories for credentials file: {}", e);
                        return Err(AuthError::FileSystemError(e));
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
                Err(AuthError::FileSystemError(e))
            }
        }
    }
}

/// Storage entry that includes the token, scopes and project it's valid for
#[derive(serde::Serialize, serde::Deserialize)]
struct StorageEntry {
    token: TokenInfo,
    scopes: String,
    project_id: String,
}

/// KeychainTokenStorage implements the TokenStorage trait from yup_oauth2
/// to enable secure storage of OAuth tokens in the system keychain.
pub struct KeychainTokenStorage {
    project_id: String,
    credentials_manager: Arc<CredentialsManager>,
}

impl KeychainTokenStorage {
    /// Create a new KeychainTokenStorage with the given CredentialsManager
    pub fn new(project_id: String, credentials_manager: Arc<CredentialsManager>) -> Self {
        Self {
            project_id,
            credentials_manager,
        }
    }

    fn generate_scoped_key(&self, scopes: &[&str]) -> String {
        // Create a key based on the scopes and project_id
        // Sort so we can be consistent using scopes as the key
        let mut sorted_scopes = scopes.to_vec();
        sorted_scopes.sort();
        sorted_scopes.join(" ")
    }
}

#[async_trait::async_trait]
impl TokenStorage for KeychainTokenStorage {
    /// Store a token in the keychain
    async fn set(&self, scopes: &[&str], token_info: TokenInfo) -> Result<()> {
        let key = self.generate_scoped_key(scopes);

        // Create a storage entry that includes the scopes
        let storage_entry = StorageEntry {
            token: token_info,
            scopes: key,
            project_id: self.project_id.clone(),
        };

        let json = serde_json::to_string(&storage_entry)?;
        self.credentials_manager
            .write_credentials(&json)
            .map_err(|e| {
                error!("Failed to write token to keychain: {}", e);
                anyhow::anyhow!("Failed to write token to keychain: {}", e)
            })
    }

    /// Retrieve a token from the keychain
    async fn get(&self, scopes: &[&str]) -> Option<TokenInfo> {
        let key = self.generate_scoped_key(scopes);

        match self.credentials_manager.read_credentials() {
            Ok(json) => {
                debug!("Successfully read credentials from storage");
                match serde_json::from_str::<StorageEntry>(&json) {
                    Ok(entry) => {
                        // Check if token has the requested scopes and matches the project_id
                        if entry.project_id == self.project_id && entry.scopes == key {
                            debug!("Successfully retrieved OAuth token from storage");
                            Some(entry.token)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        warn!("Failed to deserialize token from storage: {}", e);
                        None
                    }
                }
            }
            Err(AuthError::NotFound) => {
                debug!("No OAuth token found in storage");
                None
            }
            Err(e) => {
                warn!("Error reading OAuth token from storage: {}", e);
                None
            }
        }
    }
}

impl Clone for CredentialsManager {
    fn clone(&self) -> Self {
        Self {
            credentials_path: self.credentials_path.clone(),
            fallback_to_disk: self.fallback_to_disk,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::NamedTempFile;

    #[tokio::test]
    #[serial]
    async fn test_token_storage_set_get() {
        // Create a temporary file for testing
        let temp_file = NamedTempFile::new().unwrap();
        let project_id = "test_project_1".to_string();
        let credentials_manager = Arc::new(CredentialsManager::new(
            temp_file.path().to_string_lossy().to_string(),
        ));

        let storage = KeychainTokenStorage::new(project_id, credentials_manager);

        // Create a test token
        let token_info = TokenInfo {
            access_token: Some("test_access_token".to_string()),
            refresh_token: Some("test_refresh_token".to_string()),
            expires_at: None,
            id_token: None,
        };

        let scopes = &["https://www.googleapis.com/auth/drive.readonly"];

        // Store the token
        storage.set(scopes, token_info.clone()).await.unwrap();

        // Retrieve the token
        let retrieved = storage.get(scopes).await.unwrap();

        // Verify the token matches
        assert_eq!(retrieved.access_token, token_info.access_token);
        assert_eq!(retrieved.refresh_token, token_info.refresh_token);
    }

    #[tokio::test]
    #[serial]
    async fn test_token_storage_scope_mismatch() {
        // Create a temporary file for testing
        let temp_file = NamedTempFile::new().unwrap();
        let project_id = "test_project_2".to_string();
        let credentials_manager = Arc::new(CredentialsManager::new(
            temp_file.path().to_string_lossy().to_string(),
        ));

        let storage = KeychainTokenStorage::new(project_id, credentials_manager);

        // Create a test token
        let token_info = TokenInfo {
            access_token: Some("test_access_token".to_string()),
            refresh_token: Some("test_refresh_token".to_string()),
            expires_at: None,
            id_token: None,
        };

        let scopes1 = &["https://www.googleapis.com/auth/drive.readonly"];
        let scopes2 = &["https://www.googleapis.com/auth/drive.file"];

        // Store the token with scopes1
        storage.set(scopes1, token_info).await.unwrap();

        // Try to retrieve with different scopes
        let result = storage.get(scopes2).await;
        assert!(result.is_none());
    }
}
