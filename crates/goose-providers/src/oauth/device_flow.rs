//! Shared OAuth 2.0 Device Authorization Grant (RFC 8628) helper.
//!
//! Protocol only: request a device code, poll the token endpoint, and refresh.
//! Hosts own browser/clipboard UI and token persistence.

use std::fmt;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::api_client::DEFAULT_PROVIDER_TIMEOUT_SECS;

/// Fallback poll interval when the server omits `interval` (RFC 8628 §3.2).
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;

/// Fallback device-code window when the server omits `expires_in` (RFC 8628 §3.2).
pub const DEFAULT_DEVICE_CODE_LIFETIME_SECS: u64 = 300;

/// Extra seconds added to the poll interval after an RFC 8628 `slow_down`.
pub const SLOW_DOWN_BACKOFF_SECS: u64 = 5;

/// How a provider expects the device-authorization and token request bodies to
/// be encoded. RFC 8628 §3.1 specifies `application/x-www-form-urlencoded`, but
/// GitHub accepts JSON when `Accept: application/json` is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestEncoding {
    Form,
    Json,
}

/// Connection details for a provider's device flow.
///
/// The `Client` passed to [`request_device_code`], [`poll_once`], and
/// [`refresh_device_flow_token`] must not follow redirects: a 307/308 would
/// forward `device_code` / `refresh_token` in the body. gdk's client already
/// uses `Policy::none()`.
#[derive(Debug, Clone)]
pub struct DeviceFlowConfig<'a> {
    /// `device_authorization_endpoint` (RFC 8628 §3.1).
    /// `None` when only the refresh grant is needed.
    pub device_auth_url: Option<&'a str>,
    /// `token_endpoint` used for both device-code polling and refresh grants.
    pub token_url: &'a str,
    /// Public OAuth client identifier (installed-app credential, not a secret).
    pub client_id: &'a str,
    /// Space-separated scope string, or `None` to omit the parameter.
    pub scopes: Option<&'a str>,
    /// Provider-specific headers (user-agent, platform markers, `Accept`, etc.).
    pub extra_headers: HeaderMap,
    /// Body encoding for device-auth, polling, and refresh requests.
    pub encoding: RequestEncoding,
}

/// Fields returned by `/device_authorization` (RFC 8628 §3.2).
#[derive(Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    /// Pre-populated URI with user_code embedded, used when the provider
    /// supports it (e.g. Kimi). Fall back to `verification_uri` otherwise.
    pub verification_uri_complete: Option<String>,
    pub interval: Option<u64>,
    pub expires_in: Option<u64>,
}

impl DeviceCodeResponse {
    /// URI the user should visit. Prefers the `_complete` form when present.
    pub fn verification_url(&self) -> &str {
        self.verification_uri_complete
            .as_deref()
            .unwrap_or(&self.verification_uri)
    }

    pub fn poll_interval_secs(&self) -> u64 {
        self.interval.unwrap_or(DEFAULT_POLL_INTERVAL_SECS)
    }

    pub fn lifetime_secs(&self) -> u64 {
        self.expires_in.unwrap_or(DEFAULT_DEVICE_CODE_LIFETIME_SECS)
    }
}

impl fmt::Debug for DeviceCodeResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceCodeResponse")
            .field("device_code", &"[redacted]")
            .field("user_code", &self.user_code)
            .field("verification_uri", &self.verification_uri)
            .field("verification_uri_complete", &self.verification_uri_complete)
            .field("interval", &self.interval)
            .field("expires_in", &self.expires_in)
            .finish()
    }
}

/// Access + optional refresh credentials from a device-code exchange.
#[derive(Clone)]
pub struct DeviceFlowTokens {
    pub access_token: String,
    /// Some providers (GitHub Copilot) do not issue a refresh token.
    pub refresh_token: Option<String>,
    /// Derived from `expires_in` on the token response. `None` when the server
    /// omits it (RFC 6749 §5.1 permits that).
    pub expires_at: Option<DateTime<Utc>>,
}

impl fmt::Debug for DeviceFlowTokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceFlowTokens")
            .field("access_token", &"[redacted]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("token refresh failed ({status})")]
pub struct DeviceFlowTokenRefreshError {
    pub status: reqwest::StatusCode,
    pub error: Option<String>,
}

/// One token-endpoint poll without sleeping. Hosts drive the wait loop.
#[derive(Debug)]
pub enum DevicePollStatus {
    Pending,
    SlowDown,
    Success(DeviceFlowTokens),
    Denied,
    Expired,
}

// ── Public entry points ──────────────────────────────────────────────────────

/// Request a device code from the authorization server.
pub async fn request_device_code(
    client: &Client,
    cfg: &DeviceFlowConfig<'_>,
) -> Result<DeviceCodeResponse> {
    #[derive(Serialize)]
    struct DeviceAuthReq<'a> {
        client_id: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<&'a str>,
    }

    let body = DeviceAuthReq {
        client_id: cfg.client_id,
        scope: cfg.scopes,
    };

    let url = cfg
        .device_auth_url
        .ok_or_else(|| anyhow!("device_auth_url is required for device code request"))?;
    send_request(client, cfg, url, &body)
        .await?
        .error_for_status()?
        .json::<DeviceCodeResponse>()
        .await
        .map_err(Into::into)
}

/// Single token-endpoint poll. Does not sleep; callers wait `interval` themselves.
pub async fn poll_once(
    client: &Client,
    cfg: &DeviceFlowConfig<'_>,
    device_code: &str,
) -> Result<DevicePollStatus> {
    #[derive(Serialize)]
    struct PollReq<'a> {
        client_id: &'a str,
        device_code: &'a str,
        grant_type: &'static str,
    }

    let req = PollReq {
        client_id: cfg.client_id,
        device_code,
        grant_type: "urn:ietf:params:oauth:grant-type:device_code",
    };

    let response = send_request(client, cfg, cfg.token_url, &req).await?;

    Ok(match parse_token_response(response).await? {
        TokenPollOutcome::Issued(tokens) => DevicePollStatus::Success(tokens),
        TokenPollOutcome::Pending => DevicePollStatus::Pending,
        TokenPollOutcome::SlowDown => DevicePollStatus::SlowDown,
        TokenPollOutcome::Denied => DevicePollStatus::Denied,
        TokenPollOutcome::Expired => DevicePollStatus::Expired,
        TokenPollOutcome::Failed(err) => {
            return Err(anyhow!("authorization failed: {}", err));
        }
    })
}

/// Poll the token endpoint until the user authorizes (or the device code expires).
/// Implements RFC 8628 §3.5 — handles `authorization_pending` and `slow_down`.
pub async fn poll_for_tokens(
    client: &Client,
    cfg: &DeviceFlowConfig<'_>,
    device_code: &str,
    interval_secs: u64,
    expires_in_secs: u64,
) -> Result<DeviceFlowTokens> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(expires_in_secs);
    let mut effective_interval = interval_secs;

    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(anyhow!("timed out waiting for user authorization"));
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(effective_interval)).await;

        match poll_once(client, cfg, device_code).await? {
            DevicePollStatus::Success(tokens) => return Ok(tokens),
            DevicePollStatus::Pending => {
                tracing::debug!("authorization pending, continuing to poll");
            }
            DevicePollStatus::SlowDown => {
                tracing::debug!("slow_down received, increasing poll interval");
                effective_interval += SLOW_DOWN_BACKOFF_SECS;
            }
            DevicePollStatus::Denied => {
                return Err(anyhow!("authorization failed: access_denied"));
            }
            DevicePollStatus::Expired => {
                return Err(anyhow!("authorization failed: expired_token"));
            }
        }
    }
}

/// Exchange a refresh token for a new access token (RFC 6749 §6).
pub async fn refresh_device_flow_token(
    client: &Client,
    cfg: &DeviceFlowConfig<'_>,
    refresh_token: &str,
) -> Result<DeviceFlowTokens> {
    #[derive(Serialize)]
    struct RefreshReq<'a> {
        client_id: &'a str,
        grant_type: &'static str,
        refresh_token: &'a str,
    }

    let req = RefreshReq {
        client_id: cfg.client_id,
        grant_type: "refresh_token",
        refresh_token,
    };

    let response = send_request(client, cfg, cfg.token_url, &req).await?;
    let status = response.status();
    let bytes = response.bytes().await?;
    let raw = serde_json::from_slice::<TokenResponseBody>(&bytes);

    if !status.is_success() {
        return Err(anyhow::Error::new(DeviceFlowTokenRefreshError {
            status,
            error: raw.ok().and_then(|body| body.error),
        }));
    }

    let raw = raw?;

    let access_token = raw
        .access_token
        .ok_or_else(|| anyhow!("refresh response missing access_token"))?;
    Ok(DeviceFlowTokens {
        access_token,
        refresh_token: raw.refresh_token,
        expires_at: raw
            .expires_in
            .map(|secs| Utc::now() + Duration::seconds(secs)),
    })
}

// ── Internals ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TokenResponseBody {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    error: Option<String>,
}

enum TokenPollOutcome {
    Issued(DeviceFlowTokens),
    Pending,
    SlowDown,
    Denied,
    Expired,
    Failed(String),
}

async fn parse_token_response(response: reqwest::Response) -> Result<TokenPollOutcome> {
    let status = response.status();
    let bytes = response.bytes().await?;

    let body: TokenResponseBody = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => {
            if !status.is_success() {
                return Err(anyhow!("token poll HTTP {}", status.as_u16()));
            }
            return Err(e.into());
        }
    };

    if let Some(access_token) = body.access_token {
        return Ok(TokenPollOutcome::Issued(DeviceFlowTokens {
            access_token,
            refresh_token: body.refresh_token,
            expires_at: body
                .expires_in
                .map(|secs| Utc::now() + Duration::seconds(secs)),
        }));
    }

    Ok(match body.error.as_deref() {
        Some("authorization_pending") => TokenPollOutcome::Pending,
        Some("slow_down") => TokenPollOutcome::SlowDown,
        Some("access_denied") | Some("authorization_denied") => TokenPollOutcome::Denied,
        Some("expired_token") => TokenPollOutcome::Expired,
        Some(err) => TokenPollOutcome::Failed(err.to_string()),
        None => TokenPollOutcome::Failed(
            "unexpected token response: no access_token and no error code".to_string(),
        ),
    })
}

fn ensure_oauth_url(url: &str) -> Result<()> {
    let parsed = url::Url::parse(url).map_err(|e| anyhow!("invalid OAuth URL: {e}"))?;
    let https = parsed.scheme() == "https";
    let loopback_http = parsed.scheme() == "http"
        && parsed.host().is_some_and(|host| match host {
            url::Host::Ipv4(addr) => addr.is_loopback(),
            url::Host::Ipv6(addr) => addr.is_loopback(),
            url::Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
        });
    if https || loopback_http {
        Ok(())
    } else {
        Err(anyhow!("OAuth endpoint must use HTTPS"))
    }
}

async fn send_request<T: Serialize + ?Sized>(
    client: &Client,
    cfg: &DeviceFlowConfig<'_>,
    url: &str,
    body: &T,
) -> Result<reqwest::Response> {
    ensure_oauth_url(url)?;
    let builder = client
        .post(url)
        .timeout(std::time::Duration::from_secs(
            DEFAULT_PROVIDER_TIMEOUT_SECS,
        ))
        .headers(cfg.extra_headers.clone());
    let builder = match cfg.encoding {
        RequestEncoding::Form => builder.form(body),
        RequestEncoding::Json => builder.json(body),
    };
    builder.send().await.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_cfg<'a>(device_auth_url: Option<&'a str>, token_url: &'a str) -> DeviceFlowConfig<'a> {
        DeviceFlowConfig {
            device_auth_url,
            token_url,
            client_id: "test-client",
            scopes: None,
            extra_headers: HeaderMap::new(),
            encoding: RequestEncoding::Form,
        }
    }

    #[tokio::test]
    async fn poll_returns_issued_tokens_when_server_responds_immediately() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "the_token",
                "refresh_token": "the_refresh",
                "expires_in": 1800,
            })))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);

        let client = Client::new();
        let tokens = poll_for_tokens(&client, &cfg, "device-abc", 0, 30)
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "the_token");
        assert_eq!(tokens.refresh_token.as_deref(), Some("the_refresh"));
        assert!(tokens.expires_at.is_some());
    }

    #[tokio::test]
    async fn poll_handles_authorization_pending_then_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "authorization_pending",
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "issued",
                "expires_in": 900,
            })))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);

        let client = Client::new();
        let tokens = poll_for_tokens(&client, &cfg, "device-abc", 0, 30)
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "issued");
        assert!(tokens.refresh_token.is_none());
    }

    #[tokio::test]
    async fn poll_handles_slow_down_then_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "slow_down",
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "issued",
                "expires_in": 900,
            })))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);

        let client = Client::new();
        let tokens = poll_for_tokens(&client, &cfg, "device-abc", 0, 30)
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "issued");
    }

    #[tokio::test]
    async fn poll_times_out_when_user_never_authorizes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "authorization_pending",
            })))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);

        let client = Client::new();
        let err = poll_for_tokens(&client, &cfg, "device-abc", 0, 0)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("timed out"), "got: {}", err);
    }

    #[tokio::test]
    async fn poll_surfaces_http_status_on_unparseable_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway"))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);

        let client = Client::new();
        let err = poll_for_tokens(&client, &cfg, "device-abc", 0, 5)
            .await
            .unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("502"), "expected status in error: {}", msg);
        assert!(
            !msg.to_ascii_lowercase().contains("bad gateway"),
            "raw HTTP body must not leak: {}",
            msg
        );
    }

    #[tokio::test]
    async fn poll_surfaces_server_error_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "access_denied",
            })))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);

        let client = Client::new();
        let err = poll_for_tokens(&client, &cfg, "device-abc", 0, 5)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("access_denied"), "got: {}", err);
    }

    #[tokio::test]
    async fn request_device_code_parses_complete_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/device_authorization"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "device_code": "dc",
                "user_code": "UC-1",
                "verification_uri": "https://example.com/activate",
                "verification_uri_complete": "https://example.com/activate?user_code=UC-1",
                "interval": 3,
                "expires_in": 600,
            })))
            .mount(&server)
            .await;

        let device_url = format!("{}/device_authorization", server.uri());
        let cfg = make_cfg(Some(&device_url), "");

        let client = Client::new();
        let resp = request_device_code(&client, &cfg).await.unwrap();
        assert_eq!(resp.device_code, "dc");
        assert_eq!(resp.user_code, "UC-1");
        assert_eq!(
            resp.verification_url(),
            "https://example.com/activate?user_code=UC-1"
        );
        assert_eq!(resp.interval, Some(3));
    }

    #[tokio::test]
    async fn refresh_token_returns_new_credentials() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new_access",
                "refresh_token": "new_refresh",
                "expires_in": 3600,
            })))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);

        let client = Client::new();
        let tokens = refresh_device_flow_token(&client, &cfg, "old_refresh")
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "new_access");
        assert_eq!(tokens.refresh_token.as_deref(), Some("new_refresh"));
    }

    #[tokio::test]
    async fn refresh_token_allows_server_to_omit_refresh_token() {
        // RFC 6749 §6: server MAY omit refresh_token; caller should reuse prior.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new_access",
                "expires_in": 3600,
            })))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);

        let client = Client::new();
        let tokens = refresh_device_flow_token(&client, &cfg, "old_refresh")
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "new_access");
        assert!(tokens.refresh_token.is_none());
    }

    #[tokio::test]
    async fn poll_once_returns_pending_without_sleeping() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "authorization_pending",
            })))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);
        let status = poll_once(&Client::new(), &cfg, "device-abc").await.unwrap();
        assert!(matches!(status, DevicePollStatus::Pending));
    }

    #[tokio::test]
    async fn poll_once_maps_denied_and_expired() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "expired_token",
            })))
            .mount(&server)
            .await;

        let token_url = format!("{}/token", server.uri());
        let cfg = make_cfg(None, &token_url);
        let status = poll_once(&Client::new(), &cfg, "device-abc").await.unwrap();
        assert!(matches!(status, DevicePollStatus::Expired));
    }

    #[tokio::test]
    async fn rejects_non_https_remote_token_url() {
        let cfg = make_cfg(None, "http://example.com/token");
        let err = poll_once(&Client::new(), &cfg, "dc").await.unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("HTTPS"), "got: {msg}");
    }

    #[test]
    fn tokens_debug_redacts_secrets() {
        let tokens = DeviceFlowTokens {
            access_token: "secret-access".to_string(),
            refresh_token: Some("secret-refresh".to_string()),
            expires_at: None,
        };
        let rendered = format!("{:?}", tokens);
        assert!(!rendered.contains("secret-access"), "{rendered}");
        assert!(!rendered.contains("secret-refresh"), "{rendered}");
        assert!(rendered.contains("[redacted]"), "{rendered}");
    }
}
