//! GitHub Copilot's second hop: GitHub OAuth token → Copilot API token.
//!
//! Device-code login against github.com yields a GitHub access token, not a
//! Copilot inference token. Hosts must exchange it here (or the Copilot
//! provider in goose does the same against `copilot_internal/v2/token`).

use std::fmt;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::header::{HeaderValue, AUTHORIZATION};
use serde::Deserialize;

use super::http_client;
use super::registry::github_copilot_headers;

const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";

#[derive(Clone)]
pub struct CopilotApiToken {
    pub token: String,
    pub api_endpoint: String,
    pub expires_at: DateTime<Utc>,
}

impl fmt::Debug for CopilotApiToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CopilotApiToken")
            .field("token", &"[redacted]")
            .field("api_endpoint", &self.api_endpoint)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[derive(Deserialize)]
struct CopilotTokenEndpoints {
    api: String,
}

#[derive(Deserialize)]
struct CopilotTokenResponse {
    token: String,
    refresh_in: i64,
    endpoints: CopilotTokenEndpoints,
}

/// Exchange a GitHub OAuth access token for a short-lived Copilot API token.
pub async fn exchange_github_copilot_token(github_token: &str) -> Result<CopilotApiToken> {
    let mut headers = github_copilot_headers();
    let auth = format!("bearer {github_token}");
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth)
            .map_err(|e| anyhow!("invalid GitHub token for header: {e}"))?,
    );

    let response = http_client()?
        .get(COPILOT_TOKEN_URL)
        .timeout(std::time::Duration::from_secs(
            crate::api_client::DEFAULT_PROVIDER_TIMEOUT_SECS,
        ))
        .headers(headers)
        .send()
        .await?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        anyhow::bail!("GitHub Copilot token request failed ({status})");
    }
    let body = response
        .error_for_status()?
        .json::<CopilotTokenResponse>()
        .await?;

    ensure_copilot_api_endpoint(&body.endpoints.api)?;
    Ok(CopilotApiToken {
        token: body.token,
        api_endpoint: body.endpoints.api,
        expires_at: Utc::now() + Duration::seconds(body.refresh_in),
    })
}

pub fn ensure_copilot_api_endpoint(endpoint: &str) -> Result<()> {
    let url = url::Url::parse(endpoint).map_err(|_| anyhow!("invalid Copilot API endpoint"))?;
    let host = url
        .host()
        .ok_or_else(|| anyhow!("invalid Copilot API endpoint"))?;
    let loopback = match host {
        url::Host::Domain(host) => host.eq_ignore_ascii_case("localhost"),
        url::Host::Ipv4(addr) => addr.is_loopback(),
        url::Host::Ipv6(addr) => addr.is_loopback(),
    };
    if url.scheme() == "https" || (url.scheme() == "http" && loopback) {
        Ok(())
    } else {
        Err(anyhow!(
            "Copilot API endpoint must use HTTPS unless it targets loopback"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_plaintext_remote_copilot_endpoint() {
        let err = ensure_copilot_api_endpoint("http://api.example.com").unwrap_err();
        assert!(err.to_string().contains("HTTPS"), "got: {err}");
    }

    #[test]
    fn accepts_https_and_loopback_copilot_endpoint() {
        ensure_copilot_api_endpoint("https://api.githubcopilot.com").unwrap();
        ensure_copilot_api_endpoint("http://127.0.0.1:8080").unwrap();
    }

    #[test]
    fn copilot_token_debug_redacts_secret() {
        let token = CopilotApiToken {
            token: "secret-copilot".to_string(),
            api_endpoint: "https://api.githubcopilot.com".to_string(),
            expires_at: Utc::now(),
        };
        let rendered = format!("{token:?}");
        assert!(!rendered.contains("secret-copilot"), "{rendered}");
    }
}
