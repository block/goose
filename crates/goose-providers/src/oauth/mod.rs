//! Host-agnostic OAuth helpers for goose-providers.
//!
//! Device-code protocol and public installed-app client IDs live here. Browser
//! UI, clipboard, goose `Config` / `Paths`, and PKCE loopback do not.

pub mod copilot;
pub mod device_flow;
pub mod registry;

pub use copilot::{ensure_copilot_api_endpoint, exchange_github_copilot_token, CopilotApiToken};
pub use device_flow::{
    poll_for_tokens, poll_once, refresh_device_flow_token, request_device_code, DeviceCodeResponse,
    DeviceFlowConfig, DeviceFlowTokenRefreshError, DeviceFlowTokens, DevicePollStatus,
    RequestEncoding, DEFAULT_DEVICE_CODE_LIFETIME_SECS, DEFAULT_POLL_INTERVAL_SECS,
    SLOW_DOWN_BACKOFF_SECS,
};
pub use registry::{
    github_copilot_headers, kimi_headers, list_oauth_providers, poll_device_flow,
    refresh_oauth_token, start_device_flow, DeviceAuthSession, OAuthGrant, OAuthProviderId,
    OAuthProviderInfo, GITHUB_COPILOT_CLIENT_ID, KIMI_CODE_CLIENT_ID,
};

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::redirect::Policy;
use reqwest::Client;

use crate::api_client::DEFAULT_PROVIDER_TIMEOUT_SECS;

/// HTTP client for OAuth token endpoints. Does not follow redirects so a
/// 307/308 cannot forward `device_code` / `refresh_token` in the body.
pub(crate) fn oauth_http_client() -> anyhow::Result<Client> {
    Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS))
        .build()
        .map_err(Into::into)
}

pub(crate) fn http_client() -> anyhow::Result<&'static Client> {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = oauth_http_client()?;
    Ok(CLIENT.get_or_init(|| client))
}
