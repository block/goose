use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait KeyringBackend: Send + Sync {
    async fn get_password(&self, service: &str, username: &str) -> Result<String>;
    async fn set_password(&self, service: &str, username: &str, password: &str) -> Result<()>;
    async fn delete_password(&self, service: &str, username: &str) -> Result<()>;
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

pub mod mock;
pub mod system;

pub use mock::MockKeyringBackend;
pub use system::SystemKeyringBackend;
