use anyhow::Result;
use google_drive3::yup_oauth2::storage::{TokenInfo, TokenStorage};
use keyring::Entry;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, warn};

const KEYCHAIN_SERVICE: &str = "mcp_google_drive";
const KEYCHAIN_USERNAME: &str = "oauth_credentials";

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
/// with fallback to file system storage if keychain access fails.
pub struct CredentialsManager {
    credentials_path: String,
}

impl CredentialsManager {
    pub fn new(credentials_path: String) -> Self {
        Self { credentials_path }
    }

    pub fn read_credentials(&self) -> Result<String, AuthError> {
        // First try to read from keychain
        let entry = match Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USERNAME) {
            Ok(entry) => entry,
            Err(e) => {
                warn!("Failed to create keychain entry: {}", e);
                return self.read_from_file();
            }
        };

        match entry.get_password() {
            Ok(content) => {
                debug!("Successfully read credentials from keychain");
                Ok(content)
            }
            Err(keyring::Error::NoEntry) => {
                debug!("No credentials found in keychain, falling back to file system");
                self.read_from_file()
            }
            Err(e) => {
                // Categorize errors - some might be critical and should not trigger fallback
                warn!(
                    "Non-critical keychain error: {}, falling back to file system",
                    e
                );
                self.read_from_file()
            }
        }
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
        // Try to write to keychain first
        let entry = match Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USERNAME) {
            Ok(entry) => entry,
            Err(e) => {
                warn!("Failed to create keychain entry: {}", e);
                return self.write_to_file(content);
            }
        };

        // Fallback to writing on disk if we can't write to the keychain
        match entry.set_password(content) {
            Ok(_) => {
                debug!("Successfully wrote credentials to keychain");
                Ok(())
            }
            Err(e) => {
                warn!(
                    "Non-critical keychain error: {}, falling back to file system",
                    e
                );
                self.write_to_file(content)
            }
        }
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

/// Storage entry that includes both the token and the scopes it's valid for
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
        debug!("Storing OAuth token in keychain for scopes: {:?}", key);

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
        debug!("Retrieving OAuth token from keychain for key: {:?}", key);

        match self.credentials_manager.read_credentials() {
            Ok(json) => {
                debug!("Successfully read credentials from storage");
                match serde_json::from_str::<StorageEntry>(&json) {
                    Ok(entry) => {
                        // Check if the stored token has the requested scopes
                        debug!("{} == {}", entry.project_id, self.project_id);
                        if entry.project_id == self.project_id && entry.scopes == key {
                            debug!("Successfully retrieved OAuth token from storage");
                            Some(entry.token)
                        } else {
                            debug!(
                                "Found token but scopes don't match. Stored: {}, Requested: {}",
                                entry.scopes, key
                            );
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_read_credentials() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = CredentialsManager::new(temp_file.path().to_string_lossy().to_string());

        // Write test credentials
        let test_content = r#"{"access_token":"test_token","token_type":"Bearer"}"#;
        manager.write_credentials(test_content).unwrap();

        // Read back and verify
        let read_content = manager.read_credentials().unwrap();
        assert_eq!(read_content, test_content);
    }

    #[tokio::test]
    async fn test_token_storage_set_get() {
        // Create a temporary file for testing
        let temp_file = NamedTempFile::new().unwrap();
        let project_id = "test_project".to_string();
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
    async fn test_token_storage_scope_mismatch() {
        // Create a temporary file for testing
        let temp_file = NamedTempFile::new().unwrap();
        let project_id = "test_project".to_string();
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
