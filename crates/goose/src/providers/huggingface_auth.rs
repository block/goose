use crate::config::paths::Paths;
use crate::config::{Config, ConfigError};
use anyhow::{anyhow, Result};
use axum::{extract::Query, response::Html, routing::get, Router};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tokio::sync::{oneshot, Mutex as TokioMutex};

pub const HUGGINGFACE_PROVIDER_NAME: &str = "huggingface";
pub const HUGGINGFACE_DISPLAY_NAME: &str = "Hugging Face";
pub const HUGGINGFACE_TOKEN_SECRET_KEY: &str = "HF_TOKEN";
pub const HUGGINGFACE_OAUTH_TOKEN_NAME: &str = "OAuth token";
pub const HUGGINGFACE_OAUTH_CACHE_PATH: &str = "huggingface/oauth/tokens.json";

const AUTHORIZE_URL: &str = "https://huggingface.co/oauth/authorize";
const TOKEN_URL: &str = "https://huggingface.co/oauth/token";
const OAUTH_SCOPES: &str = "read-repos gated-repos";
const BUNDLED_OAUTH_CLIENT_ID: &str = "1d30af6e-2b6c-4b7c-a97e-45fdc1af476b";
// This URI must match the redirect URI registered on the public goose OAuth app.
const OAUTH_HOST: [u8; 4] = [127, 0, 0, 1];
const OAUTH_PORT: u16 = 17863;
const OAUTH_REDIRECT_PATH: &str = "/oauth/huggingface/callback";
const OAUTH_TIMEOUT_SECS: u64 = 300;
const HTML_AUTO_CLOSE_TIMEOUT_MS: u64 = 2000;

static HUGGINGFACE_OAUTH_MUTEX: LazyLock<TokioMutex<()>> = LazyLock::new(|| TokioMutex::new(()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceTokenData {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
}

impl HuggingFaceTokenData {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
    }
}

pub fn oauth_client_id() -> Option<&'static str> {
    option_env!("GOOSE_HUGGINGFACE_OAUTH_CLIENT_ID")
        .filter(|client_id| !client_id.trim().is_empty())
        .or_else(|| (!BUNDLED_OAUTH_CLIENT_ID.is_empty()).then_some(BUNDLED_OAUTH_CLIENT_ID))
}

pub fn oauth_cache_path() -> PathBuf {
    Paths::in_config_dir(HUGGINGFACE_OAUTH_CACHE_PATH)
}

pub fn load_oauth_token() -> Option<HuggingFaceTokenData> {
    load_oauth_token_from_path(&oauth_cache_path())
}

fn load_oauth_token_from_path(path: &std::path::Path) -> Option<HuggingFaceTokenData> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub fn has_oauth_token() -> bool {
    load_oauth_token().is_some()
}

pub fn usable_oauth_token() -> Option<String> {
    usable_oauth_token_from_path(&oauth_cache_path())
}

fn usable_oauth_token_from_path(path: &std::path::Path) -> Option<String> {
    let token = load_oauth_token_from_path(path)?;
    (!token.is_expired()).then_some(token.access_token)
}

pub fn hf_token_secret() -> Result<Option<String>> {
    match Config::global().get_secret::<String>(HUGGINGFACE_TOKEN_SECRET_KEY) {
        Ok(token) => Ok(Some(token)),
        Err(ConfigError::NotFound(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn resolve_token() -> Result<Option<String>> {
    Ok(usable_oauth_token().or(hf_token_secret()?))
}

pub fn resolve_token_with_fallback(fallback: Option<String>) -> Result<Option<String>> {
    Ok(usable_oauth_token().or(fallback).or(hf_token_secret()?))
}

pub fn clear_oauth_token() -> Result<()> {
    let path = oauth_cache_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub async fn configure_oauth() -> Result<()> {
    let client_id = oauth_client_id().ok_or_else(|| {
        anyhow!("Hugging Face OAuth client ID is not configured in this goose build")
    })?;

    let token_data = perform_loopback_oauth_flow(client_id).await?;
    save_oauth_token(token_data)
}

fn save_oauth_token(token_data: HuggingFaceTokenData) -> Result<()> {
    let path = oauth_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = serde_json::to_string(&token_data)?;
    std::fs::write(path, contents)?;
    Ok(())
}

struct PkceChallenge {
    verifier: String,
    challenge: String,
}

fn generate_pkce() -> PkceChallenge {
    let verifier = nanoid::nanoid!(64);
    let digest = sha2::Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    PkceChallenge {
        verifier,
        challenge,
    }
}

fn generate_state() -> String {
    nanoid::nanoid!(32)
}

fn redirect_uri() -> String {
    format!(
        "http://{}.{}.{}.{}:{}{}",
        OAUTH_HOST[0], OAUTH_HOST[1], OAUTH_HOST[2], OAUTH_HOST[3], OAUTH_PORT, OAUTH_REDIRECT_PATH
    )
}

fn build_authorize_url(client_id: &str, pkce: &PkceChallenge, state: &str) -> Result<String> {
    let redirect = redirect_uri();
    let params = [
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", redirect.as_str()),
        ("scope", OAUTH_SCOPES),
        ("code_challenge", pkce.challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("state", state),
    ];
    let query = serde_urlencoded::to_string(params)?;
    Ok(format!("{}?{}", AUTHORIZE_URL, query))
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

fn token_data_from_response(response: TokenResponse) -> HuggingFaceTokenData {
    HuggingFaceTokenData {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        expires_at: response
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs)),
    }
}

async fn exchange_code_for_tokens(
    client_id: &str,
    code: &str,
    pkce: &PkceChallenge,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let redirect = redirect_uri();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect.as_str()),
        ("client_id", client_id),
        ("code_verifier", pkce.verifier.as_str()),
    ];

    let resp = client
        .post(TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Hugging Face token exchange failed ({}): {}",
            status,
            text
        ));
    }

    Ok(resp.json().await?)
}

const HTML_SUCCESS_TEMPLATE: &str = r#"<!doctype html>
<html>
  <head>
    <title>goose - Hugging Face Authorization Successful</title>
    <script>setTimeout(() => window.close(), {timeout_ms});</script>
    <style>
      body {{
        font-family: system-ui, -apple-system, sans-serif;
        display: flex; justify-content: center; align-items: center;
        height: 100vh; margin: 0; background: #171717; color: #fafafa;
      }}
      .container {{ text-align: center; padding: 2rem; }}
      h1 {{ color: #ff9d00; margin-bottom: 1rem; }}
      p {{ color: #c7c7c7; }}
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Authorization Successful</h1>
      <p>You can close this window and return to goose.</p>
    </div>
  </body>
</html>"#;

fn html_success() -> String {
    HTML_SUCCESS_TEMPLATE.replace("{timeout_ms}", &HTML_AUTO_CLOSE_TIMEOUT_MS.to_string())
}

fn html_error(error: &str) -> String {
    let safe_error = v_htmlescape::escape_fmt(error);
    format!(
        r#"<!doctype html>
<html>
  <head>
    <title>goose - Hugging Face Authorization Failed</title>
    <style>
      body {{
        font-family: system-ui, -apple-system, sans-serif;
        display: flex; justify-content: center; align-items: center;
        height: 100vh; margin: 0; background: #171717; color: #fafafa;
      }}
      .container {{ text-align: center; padding: 2rem; }}
      h1 {{ color: #ff6b35; margin-bottom: 1rem; }}
      p {{ color: #c7c7c7; }}
      .error {{
        color: #ffb199; font-family: monospace; margin-top: 1rem;
        padding: 1rem; background: #3b180d; border-radius: 0.5rem;
      }}
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Authorization Failed</h1>
      <p>An error occurred during authorization.</p>
      <div class="error">{}</div>
    </div>
  </body>
</html>"#,
        safe_error
    )
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

fn oauth_callback_router(
    expected_state: String,
    tx: Arc<TokioMutex<Option<oneshot::Sender<Result<String>>>>>,
) -> Router {
    Router::new().route(
        OAUTH_REDIRECT_PATH,
        get(move |Query(params): Query<CallbackParams>| {
            let tx = tx.clone();
            let expected = expected_state.clone();
            async move {
                if let Some(error) = params.error {
                    let msg = params.error_description.unwrap_or(error);
                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send(Err(anyhow!("{}", msg)));
                    }
                    return Html(html_error(&msg));
                }

                let code = match params.code {
                    Some(c) => c,
                    None => {
                        let msg = "Missing authorization code";
                        if let Some(sender) = tx.lock().await.take() {
                            let _ = sender.send(Err(anyhow!("{}", msg)));
                        }
                        return Html(html_error(msg));
                    }
                };

                if params.state.as_deref() != Some(&expected) {
                    let msg = "Invalid state - potential CSRF attack";
                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send(Err(anyhow!("{}", msg)));
                    }
                    return Html(html_error(msg));
                }

                if let Some(sender) = tx.lock().await.take() {
                    let _ = sender.send(Ok(code));
                }
                Html(html_success())
            }
        }),
    )
}

async fn spawn_oauth_server(app: Router) -> Result<tokio::task::JoinHandle<()>> {
    let addr = SocketAddr::from((OAUTH_HOST, OAUTH_PORT));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        if e.kind() == io::ErrorKind::AddrInUse {
            anyhow!(
                "Hugging Face OAuth callback server failed to bind to {}: port {} is already in use",
                addr,
                OAUTH_PORT
            )
        } else {
            anyhow!(
                "Hugging Face OAuth callback server failed to bind to {}: {}",
                addr,
                e
            )
        }
    })?;
    Ok(tokio::spawn(async move {
        let server = axum::serve(listener, app);
        let _ = server.await;
    }))
}

struct ServerHandleGuard(Option<tokio::task::JoinHandle<()>>);

impl ServerHandleGuard {
    fn new(handle: tokio::task::JoinHandle<()>) -> Self {
        Self(Some(handle))
    }

    fn abort(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
    }
}

impl Drop for ServerHandleGuard {
    fn drop(&mut self) {
        self.abort();
    }
}

async fn wait_for_oauth_code(rx: oneshot::Receiver<Result<String>>) -> Result<String> {
    let code_result =
        tokio::time::timeout(std::time::Duration::from_secs(OAUTH_TIMEOUT_SECS), rx).await;
    code_result
        .map_err(|_| anyhow!("Hugging Face OAuth flow timed out"))??
        .map_err(|e| anyhow!("Hugging Face OAuth callback error: {}", e))
}

async fn perform_loopback_oauth_flow(client_id: &str) -> Result<HuggingFaceTokenData> {
    let _guard = HUGGINGFACE_OAUTH_MUTEX.try_lock().map_err(|_| {
        anyhow!("Another Hugging Face OAuth flow is already in progress; please try again later")
    })?;

    let pkce = generate_pkce();
    let csrf_state = generate_state();
    let auth_url = build_authorize_url(client_id, &pkce, &csrf_state)?;

    let (tx, rx) = oneshot::channel::<Result<String>>();
    let tx = Arc::new(TokioMutex::new(Some(tx)));
    let app = oauth_callback_router(csrf_state.clone(), tx);
    let server_handle = spawn_oauth_server(app).await?;
    let mut server_guard = ServerHandleGuard::new(server_handle);

    if webbrowser::open(&auth_url).is_err() {
        tracing::info!(
            "Please open this URL in your browser to authorize goose with Hugging Face:\n{}",
            auth_url
        );
    }

    let code_result = wait_for_oauth_code(rx).await;
    server_guard.abort();
    let code = code_result?;

    let tokens = exchange_code_for_tokens(client_id, &code, &pkce).await?;
    Ok(token_data_from_response(tokens))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn token_path(dir: &TempDir) -> PathBuf {
        dir.path().join(HUGGINGFACE_OAUTH_CACHE_PATH)
    }

    fn with_token_path<T>(f: impl FnOnce(PathBuf) -> T) -> T {
        let dir = TempDir::new().unwrap();
        f(token_path(&dir))
    }

    #[test]
    fn pkce_challenge_is_url_safe_base64_of_sha256_of_verifier() {
        let pkce = generate_pkce();
        assert_eq!(pkce.verifier.len(), 64);
        assert_eq!(pkce.challenge.len(), 43);
        assert!(!pkce.challenge.contains('='));
        assert!(!pkce.challenge.contains('+'));
        assert!(!pkce.challenge.contains('/'));
    }

    #[test]
    fn authorize_url_contains_required_oauth_params() {
        let pkce = PkceChallenge {
            verifier: "v".repeat(64),
            challenge: "challenge-fixture".to_string(),
        };
        let url = build_authorize_url("client-fixture", &pkce, "state-fixture").unwrap();
        assert!(url.starts_with(AUTHORIZE_URL));
        assert!(url.contains("client_id=client-fixture"));
        assert!(url.contains("code_challenge=challenge-fixture"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=state-fixture"));
        assert!(url.contains("scope=read-repos"));
        assert!(url.contains("gated-repos"));
    }

    #[test]
    fn redirect_uri_matches_huggingface_oauth_app_value() {
        assert_eq!(
            redirect_uri(),
            "http://127.0.0.1:17863/oauth/huggingface/callback"
        );
    }

    #[test]
    fn token_data_from_response_stores_expires_in_as_expires_at() {
        let token_data = token_data_from_response(TokenResponse {
            access_token: "token".to_string(),
            refresh_token: None,
            expires_in: Some(60),
        });

        let expires_at = token_data.expires_at.unwrap();
        assert!(expires_at > Utc::now());
        assert!(expires_at <= Utc::now() + chrono::Duration::seconds(60));
    }

    #[test]
    fn usable_oauth_token_skips_expired_token() {
        with_token_path(|path| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                &path,
                serde_json::to_string(&HuggingFaceTokenData {
                    access_token: "expired".to_string(),
                    refresh_token: None,
                    expires_at: Some(Utc::now() - chrono::Duration::minutes(1)),
                })
                .unwrap(),
            )
            .unwrap();

            assert_eq!(usable_oauth_token_from_path(&path), None);
        });
    }

    #[test]
    fn usable_oauth_token_returns_unexpired_token() {
        with_token_path(|path| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                &path,
                serde_json::to_string(&HuggingFaceTokenData {
                    access_token: "valid".to_string(),
                    refresh_token: None,
                    expires_at: Some(Utc::now() + chrono::Duration::minutes(1)),
                })
                .unwrap(),
            )
            .unwrap();

            assert_eq!(
                usable_oauth_token_from_path(&path).as_deref(),
                Some("valid")
            );
        });
    }

    #[test]
    fn resolver_prefers_oauth_over_fallback() {
        with_token_path(|path| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                &path,
                serde_json::to_string(&HuggingFaceTokenData {
                    access_token: "oauth".to_string(),
                    refresh_token: None,
                    expires_at: Some(Utc::now() + chrono::Duration::minutes(1)),
                })
                .unwrap(),
            )
            .unwrap();

            assert_eq!(
                usable_oauth_token_from_path(&path)
                    .or(Some("api-key".to_string()))
                    .as_deref(),
                Some("oauth")
            );
        });
    }
}
