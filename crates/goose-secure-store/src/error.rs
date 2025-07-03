use thiserror::Error;

/// Errors that can occur during secure storage operations
#[derive(Error, Debug)]
pub enum SecretError {
    /// Secret was not found in the secure store
    #[error("Secret not found: {0}")]
    NotFound(String),

    /// Storage backend failure (e.g., keychain unavailable)
    #[error("Storage failure: {0}")]
    StorageFailure(String),

    /// User cancelled the operation
    #[error("User cancelled operation")]
    UserCancelled,

    /// Invalid input parameters
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// Permission denied accessing the secure store
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Generic error for other failures
    #[error("Operation failed: {0}")]
    Other(String),
}

impl From<keyring::Error> for SecretError {
    fn from(err: keyring::Error) -> Self {
        match err {
            keyring::Error::NoEntry => {
                SecretError::NotFound("Secret not found in keyring".to_string())
            }
            keyring::Error::Invalid(msg, _) => SecretError::InvalidParameters(msg),
            keyring::Error::PlatformFailure(err) => {
                SecretError::StorageFailure(format!("Platform error: {}", err))
            }
            keyring::Error::Ambiguous(err) => {
                SecretError::StorageFailure(format!("Ambiguous keyring error: {:?}", err))
            }
            _ => SecretError::Other(format!("Keyring error: {}", err)),
        }
    }
}
