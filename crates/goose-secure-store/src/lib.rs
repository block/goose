//! Secure storage for MCP server secrets
//! 
//! This crate provides a cross-platform secure storage solution for managing
//! secrets and API keys for MCP servers within the Goose ecosystem.

pub mod error;
pub mod store;
pub mod acquisition;
pub mod validation;

pub use error::SecretError;
pub use store::{SecureStore, KeyringSecureStore, LegacyConfigStore, FileBackedStore};
pub use acquisition::SecretAcquisition;

#[cfg(test)]
pub use store::MockSecureStore;

/// Result type for secure store operations
pub type Result<T> = std::result::Result<T, SecretError>;
