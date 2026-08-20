//! CLI-facing OAuth device-flow helper.
//!
//! Protocol lives in `goose_providers::oauth`. This module re-exports it and
//! adds the host UI (clipboard, browser, stderr) used by goose configure.

pub use goose_providers::oauth::{
    poll_for_tokens, refresh_device_flow_token, request_device_code, DeviceCodeResponse,
    DeviceFlowConfig, DeviceFlowTokenRefreshError, DeviceFlowTokens, RequestEncoding,
    DEFAULT_DEVICE_CODE_LIFETIME_SECS, DEFAULT_POLL_INTERVAL_SECS,
};

use anyhow::Result;
use reqwest::Client;

tokio::task_local! {
    /// When set, called instead of the default CLI announce when a device code
    /// is obtained. Args: (user_code, verification_uri, expires_in_secs).
    /// Set by the ACP server to forward the code to the desktop UI.
    static DEVICE_CODE_ANNOUNCE: Box<dyn Fn(String, String, u64) + Send + Sync>;
}

pub async fn with_device_code_announce<F, T>(
    announce: Box<dyn Fn(String, String, u64) + Send + Sync>,
    fut: F,
) -> T
where
    F: std::future::Future<Output = T>,
{
    DEVICE_CODE_ANNOUNCE.scope(announce, fut).await
}

/// High-level flow: request a device code, print user-facing instructions,
/// open the browser, and poll until tokens are issued.
pub async fn run_device_flow(
    client: &Client,
    cfg: &DeviceFlowConfig<'_>,
) -> Result<DeviceFlowTokens> {
    let device = request_device_code(client, cfg).await?;
    announce_user_action(&device);

    let interval = device.poll_interval_secs();
    let expires_in = device.lifetime_secs();

    poll_for_tokens(client, cfg, &device.device_code, interval, expires_in).await
}

fn announce_user_action(device: &DeviceCodeResponse) {
    let verify_url = device.verification_url().to_string();

    if DEVICE_CODE_ANNOUNCE
        .try_with(|f| {
            let expires_in = device
                .expires_in
                .unwrap_or(DEFAULT_DEVICE_CODE_LIFETIME_SECS);
            f(device.user_code.clone(), verify_url.clone(), expires_in)
        })
        .is_ok()
    {
        return;
    }

    let copied = arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.set_text(&device.user_code).ok())
        .is_some();
    // stderr keeps stdout clean for CLI workflows parsing provider output.
    let clipboard_hint = if copied { " (copied to clipboard)" } else { "" };
    eprintln!(
        "Please visit {} and enter code {}{}",
        verify_url, device.user_code, clipboard_hint
    );
    if verification_uri_is_safe(&verify_url) {
        if let Err(e) = webbrowser::open(&verify_url) {
            tracing::warn!("Failed to open browser: {}", e);
        }
    } else {
        tracing::warn!(
            "Refusing to open untrusted verification URI: {}",
            verify_url
        );
    }
}

fn verification_uri_is_safe(uri: &str) -> bool {
    let Ok(url) = url::Url::parse(uri) else {
        return false;
    };
    if url.scheme() != "https" {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    host == "github.com"
        || host.ends_with(".github.com")
        || host.ends_with(".ghe.com")
        || host == "kimi.com"
        || host.ends_with(".kimi.com")
}

#[cfg(test)]
mod tests {
    use super::verification_uri_is_safe;

    #[test]
    fn opens_github_and_kimi_https_hosts() {
        assert!(verification_uri_is_safe("https://github.com/login/device"));
        assert!(verification_uri_is_safe(
            "https://auth.kimi.com/activate?user_code=AB"
        ));
        assert!(verification_uri_is_safe(
            "https://my-enterprise.ghe.com/login/device"
        ));
    }

    #[test]
    fn refuses_non_https_and_unknown_hosts() {
        assert!(!verification_uri_is_safe("http://github.com/login/device"));
        assert!(!verification_uri_is_safe("https://evil.example/phish"));
        assert!(!verification_uri_is_safe("file:///etc/passwd"));
    }
}
