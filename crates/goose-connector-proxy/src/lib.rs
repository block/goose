//! Embedded proxy server for custom LLM format conversion.
//!
//! This crate provides an OpenAI Chat Completions-compatible HTTP server that
//! translates requests to a custom enterprise LLM format. It runs as an embedded
//! tokio task within the Goose binary.
//!
//! # Usage
//!
//! ```rust,no_run
//! # async fn example() -> anyhow::Result<()> {
//! use goose_connector_proxy::{start_proxy_server, ProxyConfig};
//!
//! let config = ProxyConfig::from_env()?;
//! let port = start_proxy_server(config).await?;
//! // Proxy is now running at http://127.0.0.1:{port}
//! # Ok(())
//! # }
//! ```

pub mod auth;
pub mod converter;
pub mod models;
pub mod server;
pub mod stream;
pub mod structured_output;
pub mod tool_injection;

pub use models::ProxyConfig;

use axum::routing::{get, post};
use axum::Router;
use server::{chat_completions_handler, list_models_handler, AppState};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::info;

/// Start the embedded proxy server on a random available port.
///
/// Returns the port number the server is listening on.
///
/// The server runs as a background tokio task and will be automatically
/// dropped when the Goose process exits.
pub async fn start_proxy_server(config: ProxyConfig) -> anyhow::Result<u16> {
    let timeout = Duration::from_secs(config.timeout_secs);

    let http_client = reqwest::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(true) // match Python's verify=False
        .build()?;

    let state = AppState {
        config: Arc::new(config),
        http_client,
    };

    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions_handler))
        .route("/v1/models", get(list_models_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    info!("Connector proxy server started on 127.0.0.1:{}", port);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Connector proxy server error: {}", e);
        }
    });

    Ok(port)
}

/// Check if the connector proxy should be enabled based on environment variables.
///
/// Returns `true` if `CONNECTOR_API_KEY` is set.
pub fn should_enable_proxy() -> bool {
    std::env::var("CONNECTOR_API_KEY").is_ok()
}

/// Initialize the connector proxy if configured.
///
/// If `CONNECTOR_API_KEY` is set:
/// 1. Start the proxy server
/// 2. Set `OPENAI_BASE_URL` to point to the local proxy
/// 3. Set `GOOSE_PROVIDER` to "openai"
///
/// Returns `Ok(Some(port))` if proxy was started, `Ok(None)` if not needed.
pub async fn maybe_start_proxy() -> anyhow::Result<Option<u16>> {
    if !should_enable_proxy() {
        return Ok(None);
    }

    let config = ProxyConfig::from_env()?;
    let port = start_proxy_server(config).await?;

    // Set environment variables so Goose uses the proxy as an OpenAI endpoint.
    // Goose's OpenAI provider reads OPENAI_HOST (not OPENAI_BASE_URL),
    // so we must set OPENAI_HOST to point to our proxy.
    // We also set OPENAI_API_KEY to a dummy value since the proxy handles
    // authentication internally (via CONNECTOR_API_KEY packed headers).
    unsafe {
        std::env::set_var("OPENAI_HOST", format!("http://127.0.0.1:{}", port));
        std::env::set_var("OPENAI_BASE_PATH", "v1/chat/completions");
        std::env::set_var("OPENAI_API_KEY", "connector-proxy-internal");
        std::env::set_var("GOOSE_PROVIDER", "openai");
    }

    info!(
        "Connector proxy enabled: OPENAI_HOST=http://127.0.0.1:{}, GOOSE_PROVIDER=openai",
        port
    );

    Ok(Some(port))
}
