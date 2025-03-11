use anyhow::Result;
use keyring::Entry;
use serde::{de::DeserializeOwned, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, error, warn};

const KEYCHAIN_SERVICE: &str = "mcp_google_drive";
const KEYCHAIN_USERNAME: &str = "oauth_credentials";
const KEYCHAIN_DISK_FALLBACK_ENV: &str = "GOOGLE_DRIVE_DISK_FALLBACK";

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum StorageError {
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

    /// Reads and deserializes credentials from secure storage.
    ///
    /// This method attempts to read credentials from the system keychain first.
    /// If keychain access fails and fallback is enabled, it will try to read from the file system.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type to deserialize the credentials into. Must implement `serde::de::DeserializeOwned`.
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - The deserialized credentials
    /// * `Err(StorageError)` - If reading or deserialization fails
    ///
    /// # Examples
    ///
    /// ```
    /// # use goose_mcp::google_drive::token_storage::CredentialsManager;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct OAuthToken {
    ///     access_token: String,
    ///     refresh_token: String,
    ///     expiry: u64,
    /// }
    ///
    /// let manager = CredentialsManager::new(String::from("/path/to/credentials.json"));
    /// match manager.read_credentials::<OAuthToken>() {
    ///     Ok(token) => println!("Token expires at: {}", token.expiry),
    ///     Err(e) => eprintln!("Failed to read token: {}", e),
    /// }
    /// ```
    pub fn read_credentials<T>(&self) -> Result<T, StorageError>
    where
        T: DeserializeOwned,
    {
        let json_str = Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USERNAME)
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
                        keyring::Error::NoEntry => Err(StorageError::NotFound),
                        _ => Err(StorageError::KeyringError(e)),
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
    /// ```
    /// # use goose_mcp::google_drive::token_storage::CredentialsManager;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct OAuthToken {
    ///     access_token: String,
    ///     refresh_token: String,
    ///     expiry: u64,
    /// }
    ///
    /// let token = OAuthToken {
    ///     access_token: String::from("access_token_value"),
    ///     refresh_token: String::from("refresh_token_value"),
    ///     expiry: 1672531200, // Unix timestamp
    /// };
    ///
    /// let manager = CredentialsManager::new(String::from("/path/to/credentials.json"));
    /// if let Err(e) = manager.write_credentials(&token) {
    ///     eprintln!("Failed to write token: {}", e);
    /// }
    /// ```
    pub fn write_credentials<T>(&self, content: &T) -> Result<(), StorageError>
    where
        T: Serialize,
    {
        let json_str = serde_json::to_string(content).map_err(StorageError::SerializationError)?;

        Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USERNAME)
            .and_then(|entry| entry.set_password(&json_str))
            .inspect(|_| {
                debug!("Successfully wrote credentials to keychain");
            })
            .or_else(|e| {
                if self.fallback_to_disk {
                    warn!("Falling back to file system due to keyring error: {}", e);
                    self.write_to_file(&json_str)
                } else {
                    Err(StorageError::KeyringError(e))
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
        }
    }
}
