//! `OAuthProvider` — abstracts the OAuth flow so the extension manager doesn't
//! have to depend directly on the `oauth2` crate or on `rmcp::transport::AuthorizationManager`.
//!
//! The runtime impl in this crate ([`crate::oauth::RuntimeOAuthProvider`]) wraps
//! the existing [`crate::oauth::oauth_flow`] function and the keychain-backed
//! credential store. `goose-core` consumers will only see the trait.

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

use crate::agents::extension::{ExtensionError, ExtensionResult};
use crate::agents::mcp_client::{GooseMcpClientCapabilities, McpClientTrait};
use crate::agents::types::SharedProvider;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("OAuth flow failed: {0}")]
    Flow(String),
    #[error("credential store error: {0}")]
    Store(String),
}

/// Connect to an MCP server that requires OAuth, hiding the underlying
/// auth manager and credential store types.
///
/// The trait method does the whole flow: check stored credentials, attempt
/// silent refresh, fall back to interactive auth if needed, then return a
/// ready-to-use MCP client transport. Callers get a `Box<dyn McpClientTrait>`,
/// they never see `AuthorizationManager` or `GooseCredentialStore`.
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// True if the credential store already has credentials for this extension.
    /// Cheap check — does not perform any OAuth network calls.
    async fn has_stored_credentials(&self, extension_name: &str) -> bool;

    /// Connect to `uri` with OAuth authentication.
    ///
    /// Returns a connected MCP client. The implementation handles credential
    /// loading, token refresh, and interactive browser auth as needed.
    #[allow(clippy::too_many_arguments)]
    async fn connect_authenticated(
        &self,
        extension_name: &str,
        uri: &str,
        timeout: Duration,
        provider: SharedProvider,
        client_name: String,
        capabilities: GooseMcpClientCapabilities,
        roots_dir: &Path,
    ) -> ExtensionResult<Box<dyn McpClientTrait>>;
}

impl From<anyhow::Error> for OAuthError {
    fn from(err: anyhow::Error) -> Self {
        OAuthError::Flow(err.to_string())
    }
}

impl From<OAuthError> for ExtensionError {
    fn from(err: OAuthError) -> Self {
        ExtensionError::ConfigError(err.to_string())
    }
}
