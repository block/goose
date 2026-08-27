//! Device-code OAuth facades for providers that gdk hosts can start.
//!
//! Client IDs here are public installed-app credentials, not secrets.
//! Token persistence and browser/user-code UI stay in the host.

use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};
use reqwest::Client;
use uuid::Uuid;

use super::device_flow::{
    poll_once, refresh_device_flow_token, request_device_code, DeviceFlowConfig, DeviceFlowTokens,
    DevicePollStatus, RequestEncoding, DEFAULT_DEVICE_CODE_LIFETIME_SECS,
    DEFAULT_POLL_INTERVAL_SECS, SLOW_DOWN_BACKOFF_SECS,
};
use super::http_client;

/// Public GitHub Copilot OAuth app client id (`Iv1.` prefix). Not a secret.
pub const GITHUB_COPILOT_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
/// Public Kimi Code installed-app client id. Not a secret.
pub const KIMI_CODE_CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";

const GITHUB_DEVICE_AUTH_URL: &str = "https://github.com/login/device/code";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const KIMI_DEVICE_AUTH_URL: &str = "https://auth.kimi.com/api/oauth/device_authorization";
const KIMI_TOKEN_URL: &str = "https://auth.kimi.com/api/oauth/token";

/// Grants a host may offer. `PkceLoopback` is listed so clients can branch;
/// this crate does not implement the loopback callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthGrant {
    DeviceCode,
    PkceLoopback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProviderId {
    GitHubCopilot,
    KimiCode,
}

impl OAuthProviderId {
    const ALL: [Self; 2] = [Self::GitHubCopilot, Self::KimiCode];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::GitHubCopilot => "github_copilot",
            Self::KimiCode => "kimi_code",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::GitHubCopilot => "GitHub Copilot",
            Self::KimiCode => "Kimi Code",
        }
    }

    pub fn supports_refresh(self) -> bool {
        matches!(self, Self::KimiCode)
    }
}

impl fmt::Display for OAuthProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for OAuthProviderId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "github_copilot" => Ok(Self::GitHubCopilot),
            "kimi_code" => Ok(Self::KimiCode),
            other => Err(anyhow!("unknown OAuth provider: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthProviderInfo {
    pub id: OAuthProviderId,
    pub name: String,
    pub grants: Vec<OAuthGrant>,
    pub supports_refresh: bool,
}

/// Host-driven device-code session. `device_code` is not exposed so it is
/// harder to accidentally log.
pub struct DeviceAuthSession {
    provider: OAuthProviderId,
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    interval_secs: AtomicU64,
    expires_at: DateTime<Utc>,
    token_url: String,
    client_id: String,
    scopes: Option<String>,
    extra_headers: HeaderMap,
    encoding: RequestEncoding,
}

impl fmt::Debug for DeviceAuthSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceAuthSession")
            .field("provider", &self.provider)
            .field("device_code", &"[redacted]")
            .field("user_code", &self.user_code)
            .field("verification_uri", &self.verification_uri)
            .field("interval_secs", &self.interval_secs())
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl DeviceAuthSession {
    pub fn provider(&self) -> OAuthProviderId {
        self.provider
    }

    pub fn user_code(&self) -> &str {
        &self.user_code
    }

    pub fn verification_uri(&self) -> &str {
        self.verification_uri_complete
            .as_deref()
            .unwrap_or(&self.verification_uri)
    }

    pub fn interval_secs(&self) -> u64 {
        self.interval_secs.load(Ordering::Relaxed)
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    fn config(&self) -> DeviceFlowConfig<'_> {
        DeviceFlowConfig {
            device_auth_url: None,
            token_url: &self.token_url,
            client_id: &self.client_id,
            scopes: self.scopes.as_deref(),
            extra_headers: self.extra_headers.clone(),
            encoding: self.encoding,
        }
    }
}

pub fn list_oauth_providers() -> Vec<OAuthProviderInfo> {
    OAuthProviderId::ALL
        .into_iter()
        .map(|id| OAuthProviderInfo {
            id,
            name: id.display_name().to_string(),
            grants: vec![OAuthGrant::DeviceCode],
            supports_refresh: id.supports_refresh(),
        })
        .collect()
}

pub async fn start_device_flow(provider: OAuthProviderId) -> Result<DeviceAuthSession> {
    start_device_flow_with(provider, http_client()?, &provider_config(provider)).await
}

pub async fn poll_device_flow(session: &DeviceAuthSession) -> Result<DevicePollStatus> {
    if session.is_expired() {
        return Ok(DevicePollStatus::Expired);
    }
    let status = poll_once(http_client()?, &session.config(), &session.device_code).await?;
    if matches!(status, DevicePollStatus::SlowDown) {
        session
            .interval_secs
            .fetch_add(SLOW_DOWN_BACKOFF_SECS, Ordering::Relaxed);
    }
    Ok(status)
}

pub async fn refresh_oauth_token(
    provider: OAuthProviderId,
    refresh_token: &str,
) -> Result<DeviceFlowTokens> {
    if !provider.supports_refresh() {
        anyhow::bail!("refresh is not supported for {provider}");
    }
    let cfg = provider_config(provider);
    let flow = DeviceFlowConfig {
        device_auth_url: None,
        token_url: &cfg.token_url,
        client_id: &cfg.client_id,
        scopes: cfg.scopes.as_deref(),
        extra_headers: cfg.extra_headers.clone(),
        encoding: cfg.encoding,
    };
    refresh_device_flow_token(http_client()?, &flow, refresh_token).await
}

struct ProviderEndpoints {
    device_auth_url: String,
    token_url: String,
    client_id: String,
    scopes: Option<String>,
    extra_headers: HeaderMap,
    encoding: RequestEncoding,
}

fn provider_config(provider: OAuthProviderId) -> ProviderEndpoints {
    match provider {
        OAuthProviderId::GitHubCopilot => ProviderEndpoints {
            device_auth_url: GITHUB_DEVICE_AUTH_URL.to_string(),
            token_url: GITHUB_TOKEN_URL.to_string(),
            client_id: GITHUB_COPILOT_CLIENT_ID.to_string(),
            scopes: Some("read:user".to_string()),
            extra_headers: github_copilot_headers(),
            encoding: RequestEncoding::Json,
        },
        OAuthProviderId::KimiCode => ProviderEndpoints {
            device_auth_url: KIMI_DEVICE_AUTH_URL.to_string(),
            token_url: KIMI_TOKEN_URL.to_string(),
            client_id: KIMI_CODE_CLIENT_ID.to_string(),
            scopes: None,
            extra_headers: kimi_headers(),
            encoding: RequestEncoding::Form,
        },
    }
}

pub fn github_copilot_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("GithubCopilot/1.155.0"),
    );
    headers.insert("editor-version", HeaderValue::from_static("vscode/1.85.1"));
    headers.insert(
        "editor-plugin-version",
        HeaderValue::from_static("copilot/1.155.0"),
    );
    headers
}

fn kimi_device_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| Uuid::new_v4().simple().to_string())
}

pub fn kimi_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("X-Msh-Platform", HeaderValue::from_static("kimi_cli"));
    headers.insert("X-Msh-Version", HeaderValue::from_static("0.1.0"));
    if let Ok(value) = HeaderValue::from_str(kimi_device_id()) {
        headers.insert("X-Msh-Device-Id", value);
    }
    headers
}

fn ensure_https_or_loopback_uri(uri: &str) -> Result<()> {
    let parsed = url::Url::parse(uri).map_err(|e| anyhow!("invalid verification URI: {e}"))?;
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
        Err(anyhow!("verification URI must use HTTPS"))
    }
}

async fn start_device_flow_with(
    provider: OAuthProviderId,
    client: &Client,
    endpoints: &ProviderEndpoints,
) -> Result<DeviceAuthSession> {
    let cfg = DeviceFlowConfig {
        device_auth_url: Some(&endpoints.device_auth_url),
        token_url: &endpoints.token_url,
        client_id: &endpoints.client_id,
        scopes: endpoints.scopes.as_deref(),
        extra_headers: endpoints.extra_headers.clone(),
        encoding: endpoints.encoding,
    };
    let device = request_device_code(client, &cfg).await?;
    ensure_https_or_loopback_uri(device.verification_url())?;
    let lifetime = device
        .expires_in
        .unwrap_or(DEFAULT_DEVICE_CODE_LIFETIME_SECS);
    Ok(DeviceAuthSession {
        provider,
        device_code: device.device_code,
        user_code: device.user_code,
        verification_uri: device.verification_uri,
        verification_uri_complete: device.verification_uri_complete,
        interval_secs: AtomicU64::new(device.interval.unwrap_or(DEFAULT_POLL_INTERVAL_SECS)),
        expires_at: Utc::now() + chrono::Duration::seconds(lifetime as i64),
        token_url: endpoints.token_url.clone(),
        client_id: endpoints.client_id.clone(),
        scopes: endpoints.scopes.clone(),
        extra_headers: endpoints.extra_headers.clone(),
        encoding: endpoints.encoding,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn kimi_headers_include_device_id() {
        let headers = kimi_headers();
        assert!(headers.get("X-Msh-Device-Id").is_some());
        assert_eq!(
            headers.get("X-Msh-Platform").and_then(|v| v.to_str().ok()),
            Some("kimi_cli")
        );
    }

    #[test]
    fn lists_device_code_providers_and_shapes_pkce_grant() {
        let listed = list_oauth_providers();
        assert_eq!(listed.len(), 2);
        assert!(listed
            .iter()
            .any(|p| p.id == OAuthProviderId::GitHubCopilot && !p.supports_refresh));
        assert!(listed
            .iter()
            .any(|p| p.id == OAuthProviderId::KimiCode && p.supports_refresh));
        assert!(listed.iter().all(|p| p.grants == [OAuthGrant::DeviceCode]));
        let _ = OAuthGrant::PkceLoopback;
    }

    #[test]
    fn parses_provider_ids() {
        assert_eq!(
            "github_copilot".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::GitHubCopilot
        );
        assert_eq!(
            "kimi_code".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::KimiCode
        );
        assert!("chatgpt_codex".parse::<OAuthProviderId>().is_err());
    }

    #[tokio::test]
    async fn copilot_refresh_is_unsupported() {
        let err = refresh_oauth_token(OAuthProviderId::GitHubCopilot, "rt")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[tokio::test]
    async fn start_and_poll_against_mock_server() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/device"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "device_code": "dc-secret",
                "user_code": "ABCD-EFGH",
                "verification_uri": "https://example.com/activate",
                "interval": 5,
                "expires_in": 600,
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "authorization_pending",
            })))
            .mount(&server)
            .await;

        let endpoints = ProviderEndpoints {
            device_auth_url: format!("{}/device", server.uri()),
            token_url: format!("{}/token", server.uri()),
            client_id: GITHUB_COPILOT_CLIENT_ID.to_string(),
            scopes: Some("read:user".to_string()),
            extra_headers: HeaderMap::new(),
            encoding: RequestEncoding::Json,
        };
        let session =
            start_device_flow_with(OAuthProviderId::GitHubCopilot, &Client::new(), &endpoints)
                .await
                .unwrap();
        assert_eq!(session.user_code(), "ABCD-EFGH");
        assert_eq!(session.verification_uri(), "https://example.com/activate");
        let debug = format!("{session:?}");
        assert!(!debug.contains("dc-secret"), "{debug}");

        let status = poll_device_flow(&session).await.unwrap();
        assert!(matches!(status, DevicePollStatus::Pending));
    }
}
