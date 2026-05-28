//! Runtime impl of [`crate::traits::OAuthProvider`] backed by the existing
//! [`oauth_flow`](super::oauth_flow) function and [`GooseCredentialStore`](super::GooseCredentialStore).
//!
//! This is the concrete impl that `crates/goose` exposes to consumers of the
//! `OAuthProvider` trait. It hides `rmcp::transport::AuthorizationManager` and
//! the credential store behind the trait so `agents/extension_manager.rs`
//! (eventually `goose-core`) does not have to import them directly.

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use axum::http::HeaderMap;
use rmcp::transport::auth::{AuthClient, CredentialStore};
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::StreamableHttpClientTransport;
use tracing::warn;

use crate::agents::extension::{ExtensionError, ExtensionResult};
use crate::agents::mcp_client::{GooseMcpClientCapabilities, McpClient, McpClientTrait};
use crate::agents::types::SharedProvider;
use crate::oauth::{oauth_flow, GooseCredentialStore};
use crate::traits::OAuthProvider;

pub const GOOSE_USER_AGENT: reqwest::header::HeaderValue =
    reqwest::header::HeaderValue::from_static(concat!("goose/", env!("CARGO_PKG_VERSION")));

/// Runtime [`OAuthProvider`] impl. Stateless; construct directly with `RuntimeOAuthProvider`.
#[derive(Default, Clone, Copy)]
pub struct RuntimeOAuthProvider;

#[async_trait]
impl OAuthProvider for RuntimeOAuthProvider {
    async fn has_stored_credentials(&self, extension_name: &str) -> bool {
        let store = GooseCredentialStore::new(extension_name.to_string());
        store.load().await.is_ok_and(|c| c.is_some())
    }

    async fn connect_authenticated(
        &self,
        extension_name: &str,
        uri: &str,
        timeout: Duration,
        provider: SharedProvider,
        client_name: String,
        capabilities: GooseMcpClientCapabilities,
        roots_dir: &Path,
    ) -> ExtensionResult<Box<dyn McpClientTrait>> {
        let auth_manager = oauth_flow(&uri.to_string(), &extension_name.to_string())
            .await
            .map_err(|e| {
                warn!("[OAuth:{}] flow failed: {}", extension_name, e);
                ExtensionError::ConfigError(format!("oauth flow failed: {e}"))
            })?;

        let mut auth_headers = HeaderMap::new();
        auth_headers.insert(reqwest::header::USER_AGENT, GOOSE_USER_AGENT);
        #[allow(unused_mut)]
        let mut auth_client_builder = reqwest::Client::builder().default_headers(auth_headers);
        #[cfg(target_os = "linux")]
        {
            auth_client_builder = auth_client_builder.tcp_user_timeout(Some(timeout));
        }
        let auth_http_client = auth_client_builder.build().map_err(|_| {
            ExtensionError::ConfigError("could not construct http client".to_string())
        })?;
        let auth_client = AuthClient::new(auth_http_client, auth_manager);
        let transport = StreamableHttpClientTransport::with_client(
            auth_client,
            StreamableHttpClientTransportConfig::with_uri(uri),
        );
        Ok(Box::new(
            McpClient::connect(
                transport,
                timeout,
                provider,
                client_name,
                capabilities,
                roots_dir.to_path_buf(),
            )
            .await?,
        ))
    }
}
