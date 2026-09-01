pub mod server;

#[cfg(test)]
mod tests;

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{distr::Alphanumeric, RngExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::timeout;

use self::server::CallbackResult;

/// Model the provider is set to after a successful sign-in. Any catalog id
/// works; this one is a broadly capable default the user can change with
/// `/model`.
const AIMLAPI_DEFAULT_MODEL: &str = "anthropic/claude-sonnet-5";

/// Account/consent host. Overridable so one build can be pointed at a
/// non-production environment for testing; the default is production and no
/// user ever needs to set it.
const AIMLAPI_APP_URL_ENV: &str = "AIMLAPI_APP_URL";
pub(crate) const AIMLAPI_APP_URL_DEFAULT: &str = "https://app.aimlapi.com";

/// Where the browser consent screen lives (the page the user actually sees).
///
/// This is sent as `verificationBaseUrl`, and the server hands it straight back
/// with `/agent/authorize` appended. The web app is served under an `/app/`
/// base path, so the base has to carry it: dropping the `/app` yields
/// `https://aimlapi.com/agent/authorize`, which is a 404 and strands the user
/// on the very first step of the flow.
const AIMLAPI_WEB_URL_ENV: &str = "AIMLAPI_WEB_URL";
pub(crate) const AIMLAPI_WEB_URL_DEFAULT: &str = "https://aimlapi.com/app";

/// Partner attribution. AI/ML API expects a registered partner id on the
/// authorization request; it identifies goose as the integration that brought
/// the user, and carries no user data — no account, no prompt, no usage.
///
/// goose's own registered id ships compiled in, so a normal install needs no
/// configuration. The environment variable exists to point a build at another
/// AI/ML API environment during testing, alongside the two URLs above.
const AIMLAPI_PARTNER_ID_ENV: &str = "AIMLAPI_PARTNER_ID";
pub(crate) const AIMLAPI_PARTNER_ID_DEFAULT: &str = "part_R2KG8QMDBjtWAubVMgG0GF9L";

/// Loopback port the consent screen redirects back to. Chosen from the
/// ephemeral range and fixed, because it has to be registered with the
/// authorization request before the browser opens.
const CALLBACK_PORT: u16 = 53682;

const AUTH_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Authorization-code + PKCE sign-in for AI/ML API (RFC 7636).
///
/// Unlike a provider that takes the challenge as a browser query parameter,
/// AI/ML API starts the request server-side: the CLI POSTs the challenge and
/// its loopback redirect, gets back a consent URL, and the browser is sent
/// there. The code that comes back to the loopback listener is exchanged with
/// the verifier for an api-key — the key is minted only at that exchange, and
/// only once.
#[derive(Debug)]
pub struct PkceAuthFlow {
    code_verifier: String,
    code_challenge: String,
    state: String,
    request_id: Option<String>,
    server_shutdown_tx: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Serialize)]
struct CreateAuthorizationRequest {
    #[serde(rename = "partnerId")]
    partner_id: String,
    #[serde(rename = "agentName")]
    agent_name: String,
    #[serde(rename = "codeChallenge")]
    code_challenge: String,
    #[serde(rename = "codeChallengeMethod")]
    code_challenge_method: String,
    #[serde(rename = "redirectUri")]
    redirect_uri: String,
    state: String,
    #[serde(rename = "verificationBaseUrl")]
    verification_base_url: String,
}

#[derive(Debug, Deserialize)]
struct CreateAuthorizationResponse {
    #[serde(rename = "requestId")]
    request_id: String,
    #[serde(rename = "verificationUriComplete")]
    verification_uri_complete: String,
}

#[derive(Debug, Serialize)]
struct ExchangeRequest {
    code: String,
    #[serde(rename = "codeVerifier")]
    code_verifier: String,
}

#[derive(Debug, Deserialize)]
struct ExchangeResponse {
    status: String,
    #[serde(rename = "apiKey")]
    api_key: Option<String>,
}

impl PkceAuthFlow {
    pub fn new() -> Result<Self> {
        // RFC 7636 §4.1 allows 43..128 unreserved characters.
        let code_verifier: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(96)
            .map(char::from)
            .collect();

        let mut hasher = Sha256::new();
        hasher.update(&code_verifier);
        let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

        let state: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(24)
            .map(char::from)
            .collect();

        Ok(Self {
            code_verifier,
            code_challenge,
            state,
            request_id: None,
            server_shutdown_tx: None,
        })
    }

    fn redirect_uri() -> String {
        format!("http://127.0.0.1:{}/", CALLBACK_PORT)
    }

    /// Registers the authorization request and returns the consent URL to open.
    async fn start_authorization(&mut self) -> Result<String> {
        let app_url = env_or(AIMLAPI_APP_URL_ENV, AIMLAPI_APP_URL_DEFAULT);
        let partner_id = env_or(AIMLAPI_PARTNER_ID_ENV, AIMLAPI_PARTNER_ID_DEFAULT);

        let body = CreateAuthorizationRequest {
            partner_id,
            agent_name: "goose".to_string(),
            code_challenge: self.code_challenge.clone(),
            code_challenge_method: "S256".to_string(),
            redirect_uri: Self::redirect_uri(),
            state: self.state.clone(),
            verification_base_url: env_or(AIMLAPI_WEB_URL_ENV, AIMLAPI_WEB_URL_DEFAULT),
        };

        let response = Client::new()
            .post(format!("{}/v1/agent-auth/authorizations", app_url))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Could not start AI/ML API sign-in: {} - {}",
                status,
                detail
            ));
        }

        let created: CreateAuthorizationResponse = response.json().await?;
        self.request_id = Some(created.request_id);
        Ok(created.verification_uri_complete)
    }

    /// Starts the loopback listener and waits for the browser to come back.
    async fn wait_for_callback(&mut self) -> Result<CallbackResult> {
        let (code_tx, code_rx) = oneshot::channel::<CallbackResult>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        self.server_shutdown_tx = Some(shutdown_tx);

        tokio::spawn(async move {
            if let Err(e) = server::run_callback_server(code_tx, shutdown_rx).await {
                eprintln!("Callback server error: {}", e);
            }
        });

        match timeout(AUTH_TIMEOUT, code_rx).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Err(anyhow!("Did not receive an authorization code")),
            Err(_) => Err(anyhow!("Sign-in timed out - please try again")),
        }
    }

    /// Exchanges the one-time code plus the verifier for the api-key.
    async fn exchange_code(&self, code: String) -> Result<String> {
        let app_url = env_or(AIMLAPI_APP_URL_ENV, AIMLAPI_APP_URL_DEFAULT);
        let response = Client::new()
            .post(format!("{}/v1/agent-auth/token/code", app_url))
            .json(&ExchangeRequest {
                code,
                code_verifier: self.code_verifier.clone(),
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            return Err(anyhow!("Key exchange failed: {} - {}", status, detail));
        }

        let exchanged: ExchangeResponse = response.json().await?;
        match (exchanged.status.as_str(), exchanged.api_key) {
            ("approved", Some(key)) => Ok(key),
            // The server deliberately does not distinguish an unknown code from
            // a bad verifier from a real expiry, so neither does this message.
            (status, _) => Err(anyhow!(
                "Sign-in did not complete ({}). Please run the sign-in again.",
                status
            )),
        }
    }

    /// Full flow: register, open the browser, catch the redirect, exchange.
    pub async fn complete_flow(&mut self) -> Result<String> {
        let consent_url = self.start_authorization().await?;

        println!("Opening your browser to authorize goose...");
        if let Err(e) = webbrowser::open(&consent_url) {
            eprintln!("Could not open the browser automatically: {}", e);
            println!("Open this URL manually: {}", consent_url);
        }

        println!("Waiting for you to approve in the browser...");
        let callback = self.wait_for_callback().await?;

        // RFC 6749 §10.12: a redirect whose state is not the one we sent did not
        // come from the request we started, so its code is not ours to redeem.
        if callback.state.as_deref() != Some(self.state.as_str()) {
            return Err(anyhow!(
                "The sign-in response did not match this request - please try again"
            ));
        }

        let api_key = self.exchange_code(callback.code).await?;

        if let Some(tx) = self.server_shutdown_tx.take() {
            let _ = tx.send(());
        }

        Ok(api_key)
    }
}

pub use self::PkceAuthFlow as AimlapiAuth;

use crate::config::Config;

pub fn configure_aimlapi(config: &Config, api_key: String) -> Result<()> {
    config.set_secret("AIMLAPI_API_KEY", &api_key)?;
    crate::config::set_active_provider(config, "aimlapi", AIMLAPI_DEFAULT_MODEL)?;
    Ok(())
}
