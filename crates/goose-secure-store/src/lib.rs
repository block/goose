//! Secure storage for MCP server secrets
//!
//! This crate provides a cross-platform secure storage solution for managing
//! secrets and API keys for MCP servers within the Goose ecosystem.

pub mod acquisition;
pub mod error;
pub mod store;
pub mod validation;

pub use acquisition::SecretAcquisition;
pub use error::SecretError;
pub use store::{FileBackedStore, KeyringSecureStore, LegacyConfigStore, SecureStore};

#[cfg(test)]
pub use store::MockSecureStore;

/// Result type for secure store operations
pub type Result<T> = std::result::Result<T, SecretError>;
