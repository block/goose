use anyhow::Result;

pub trait KeyringBackend: Send + Sync {
    fn get_password(&self, service: &str, username: &str) -> Result<String>;
    fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()>;
    fn delete_password(&self, service: &str, username: &str) -> Result<()>;
}

#[derive(Debug, thiserror::Error)]
pub enum KeyringError {
    #[error("No entry found for service '{service}' user '{username}'")]
    NotFound { service: String, username: String },
    #[error("Access denied to keyring")]
    AccessDenied,
    #[error("Keyring backend error: {0}")]
    Backend(String),
}

pub mod factory;
pub mod file;
pub mod mock;
pub mod system;

pub use factory::{DefaultKeyringConfig, KeyringFactory};
pub use file::FileKeyringBackend;
pub use mock::MockKeyringBackend;
pub use system::SystemKeyringBackend;

/// Convenience function for creating a keyring backend with environment-based defaults.
///
/// This is the most common use case - it automatically selects the appropriate
/// keyring backend based on environment variables:
/// - `GOOSE_USE_MOCK_KEYRING=true` → MockKeyringBackend (for testing)
/// - `GOOSE_DISABLE_KEYRING=true` → FileKeyringBackend (for systems without keyring)
/// - Default → SystemKeyringBackend (for normal operation)
///
/// # Examples
///
/// ```rust,no_run
/// use goose::keyring::create_default_keyring;
///
/// let keyring = create_default_keyring();
/// keyring.set_password("service", "user", "password").unwrap();
/// ```
pub fn create_default_keyring() -> std::sync::Arc<dyn KeyringBackend> {
    KeyringFactory::create_default()
}

/// Convenience function for creating a keyring backend with a custom file path.
///
/// This is useful when you need to store secrets in a specific location
/// while still respecting the environment variable hierarchy.
///
/// # Examples
///
/// ```rust,no_run
/// use goose::keyring::create_keyring_with_file_path;
/// use std::path::PathBuf;
///
/// let keyring = create_keyring_with_file_path(PathBuf::from("/custom/secrets.yaml"));
/// ```
pub fn create_keyring_with_file_path(
    file_path: std::path::PathBuf,
) -> std::sync::Arc<dyn KeyringBackend> {
    KeyringFactory::create_with_config(DefaultKeyringConfig::new().with_file_path(file_path))
}
