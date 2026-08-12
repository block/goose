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
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use tokio::sync::{oneshot, Mutex as TokioMutex};

pub const HUGGINGFACE_PROVIDER_NAME: &str = "huggingface";
pub const HUGGINGFACE_DISPLAY_NAME: &str = "Hugging Face";
pub const HUGGINGFACE_TOKEN_SECRET_KEY: &str = "HF_TOKEN";
pub const HUGGINGFACE_OAUTH_TOKEN_NAME: &str = "OAuth token";
pub const HUGGINGFACE_OAUTH_CACHE_PATH: &str = "huggingface/oauth/tokens.json";

const AUTHORIZE_URL: &str = "https://huggingface.co/oauth/authorize";
const TOKEN_URL: &str = "https://huggingface.co/oauth/token";
const OAUTH_SCOPES: &str = "read-repos gated-repos inference-api";
const HUGGINGFACE_OAUTH_CLIENT_METADATA_URL: &str =
    "https://goose-docs.ai/oauth/huggingface-client-metadata.json";
// This URI must match the redirect URI in the Hugging Face CIMD metadata.
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

pub fn oauth_client_id() -> &'static str {
    option_env!("GOOSE_HUGGINGFACE_OAUTH_CLIENT_ID")
        .filter(|client_id| !client_id.trim().is_empty())
        .unwrap_or(HUGGINGFACE_OAUTH_CLIENT_METADATA_URL)
}

pub fn oauth_cache_path() -> PathBuf {
    Paths::in_config_dir(HUGGINGFACE_OAUTH_CACHE_PATH)
}

pub fn load_oauth_token() -> Option<HuggingFaceTokenData> {
    load_oauth_token_from_path(&oauth_cache_path())
}

fn load_oauth_token_from_path(path: &Path) -> Option<HuggingFaceTokenData> {
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

pub fn has_usable_or_refreshable_oauth_token() -> bool {
    has_usable_or_refreshable_oauth_token_from_path(&oauth_cache_path())
}

fn has_usable_or_refreshable_oauth_token_from_path(path: &std::path::Path) -> bool {
    load_oauth_token_from_path(path).is_some_and(|token| {
        !token.is_expired()
            || token
                .refresh_token
                .as_deref()
                .is_some_and(|token| !token.is_empty())
    })
}

pub fn has_configured_token() -> Result<bool> {
    has_configured_token_from_sources(has_usable_or_refreshable_oauth_token(), hf_token_secret)
}

fn has_configured_token_from_sources(
    has_oauth_token: bool,
    secret_fallback: impl FnOnce() -> Result<Option<String>>,
) -> Result<bool> {
    if has_oauth_token {
        return Ok(true);
    }

    Ok(secret_fallback()?.is_some())
}

pub fn hf_token_secret() -> Result<Option<String>> {
    match Config::global().get_secret::<String>(HUGGINGFACE_TOKEN_SECRET_KEY) {
        Ok(token) => Ok(Some(token)),
        Err(ConfigError::NotFound(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn resolve_token() -> Result<Option<String>> {
    resolve_token_from_sources(None, usable_oauth_token(), hf_token_secret)
}

pub fn resolve_token_with_provider_token(provider_token: Option<String>) -> Result<Option<String>> {
    resolve_token_from_sources(provider_token, usable_oauth_token(), hf_token_secret)
}

pub async fn resolve_token_async() -> Result<Option<String>> {
    resolve_token_async_with_provider_token(None).await
}

pub async fn resolve_token_async_with_provider_token(
    provider_token: Option<String>,
) -> Result<Option<String>> {
    resolve_token_async_from_sources(
        provider_token,
        refreshed_or_usable_oauth_token_from_path(
            &oauth_cache_path(),
            oauth_client_id(),
            TOKEN_URL,
        ),
        hf_token_secret,
    )
    .await
}

async fn resolve_token_async_from_sources(
    provider_token: Option<String>,
    oauth_token: impl std::future::Future<Output = Result<Option<String>>>,
    secret_fallback: impl FnOnce() -> Result<Option<String>>,
) -> Result<Option<String>> {
    if provider_token.is_some() {
        return Ok(provider_token);
    }

    match oauth_token.await {
        Ok(Some(token)) => return Ok(Some(token)),
        Ok(None) => {}
        Err(refresh_error) => {
            return match secret_fallback()? {
                Some(token) => Ok(Some(token)),
                None => Err(refresh_error),
            };
        }
    }

    secret_fallback()
}

fn resolve_token_from_sources(
    provider_token: Option<String>,
    oauth_token: Option<String>,
    secret_fallback: impl FnOnce() -> Result<Option<String>>,
) -> Result<Option<String>> {
    if provider_token.is_some() {
        return Ok(provider_token);
    }

    if oauth_token.is_some() {
        return Ok(oauth_token);
    }

    secret_fallback()
}

pub fn clear_oauth_token() -> Result<()> {
    let path = oauth_cache_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub async fn configure_oauth() -> Result<()> {
    let token_data = perform_loopback_oauth_flow(oauth_client_id()).await?;
    save_oauth_token(token_data)
}

fn save_oauth_token(token_data: HuggingFaceTokenData) -> Result<()> {
    let path = oauth_cache_path();
    save_oauth_token_to_path(&path, &token_data)
}

fn save_oauth_token_to_path(path: &Path, token_data: &HuggingFaceTokenData) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = serde_json::to_string(&token_data)?;
    std::fs::write(path, contents)?;
    restrict_token_file_permissions(path)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_token_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_token_file_permissions(_path: &Path) -> Result<()> {
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
    token_data_from_response_with_refresh_fallback(response, None)
}

fn token_data_from_response_with_refresh_fallback(
    response: TokenResponse,
    refresh_token_fallback: Option<String>,
) -> HuggingFaceTokenData {
    HuggingFaceTokenData {
        access_token: response.access_token,
        refresh_token: response.refresh_token.or(refresh_token_fallback),
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

async fn refresh_access_token(
    client_id: &str,
    refresh_token: &str,
    token_url: &str,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
    ];

    let resp = client
        .post(token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Hugging Face token refresh failed ({}): {}",
            status,
            text
        ));
    }

    Ok(resp.json().await?)
}

async fn refreshed_or_usable_oauth_token_from_path(
    path: &Path,
    client_id: &str,
    token_url: &str,
) -> Result<Option<String>> {
    let Some(token) = load_oauth_token_from_path(path) else {
        return Ok(None);
    };

    if !token.is_expired() {
        return Ok(Some(token.access_token));
    }

    let Some(refresh_token) = token.refresh_token else {
        return Ok(None);
    };

    let refreshed = refresh_access_token(client_id, &refresh_token, token_url).await?;
    let refreshed =
        token_data_from_response_with_refresh_fallback(refreshed, Some(refresh_token.clone()));
    let access_token = refreshed.access_token.clone();
    save_oauth_token_to_path(path, &refreshed)?;
    Ok(Some(access_token))
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
    use wiremock::matchers::{body_string_contains, method, path as request_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
        assert!(url.contains("inference-api"));
    }

    #[test]
    fn oauth_client_id_defaults_to_cimd_metadata_url() {
        if option_env!("GOOSE_HUGGINGFACE_OAUTH_CLIENT_ID").is_none() {
            assert_eq!(oauth_client_id(), HUGGINGFACE_OAUTH_CLIENT_METADATA_URL);
        }
    }

    #[test]
    fn redirect_uri_matches_huggingface_cimd_metadata() {
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

    #[tokio::test]
    async fn expired_oauth_token_refreshes_with_cached_refresh_token() {
        let dir = TempDir::new().unwrap();
        let path = token_path(&dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            serde_json::to_string(&HuggingFaceTokenData {
                access_token: "expired".to_string(),
                refresh_token: Some("refresh".to_string()),
                expires_at: Some(Utc::now() - chrono::Duration::minutes(1)),
            })
            .unwrap(),
        )
        .unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(request_path("/"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=refresh"))
            .and(body_string_contains("client_id=client-fixture"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "refreshed",
                "expires_in": 60
            })))
            .mount(&server)
            .await;

        let token =
            refreshed_or_usable_oauth_token_from_path(&path, "client-fixture", &server.uri())
                .await
                .unwrap();

        assert_eq!(token.as_deref(), Some("refreshed"));
        let saved = load_oauth_token_from_path(&path).unwrap();
        assert_eq!(saved.access_token, "refreshed");
        assert_eq!(saved.refresh_token.as_deref(), Some("refresh"));
        assert!(saved.expires_at.unwrap() > Utc::now());
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
    fn has_usable_or_refreshable_oauth_token_accepts_unexpired_token() {
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

            assert!(has_usable_or_refreshable_oauth_token_from_path(&path));
        });
    }

    #[test]
    fn has_usable_or_refreshable_oauth_token_accepts_expired_refreshable_token() {
        with_token_path(|path| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                &path,
                serde_json::to_string(&HuggingFaceTokenData {
                    access_token: "expired".to_string(),
                    refresh_token: Some("refresh".to_string()),
                    expires_at: Some(Utc::now() - chrono::Duration::minutes(1)),
                })
                .unwrap(),
            )
            .unwrap();

            assert!(has_usable_or_refreshable_oauth_token_from_path(&path));
        });
    }

    #[test]
    fn has_usable_or_refreshable_oauth_token_rejects_expired_unrefreshable_token() {
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

            assert!(!has_usable_or_refreshable_oauth_token_from_path(&path));
        });
    }

    #[test]
    fn has_configured_token_accepts_oauth_without_secret_lookup() {
        let configured = has_configured_token_from_sources(true, || {
            panic!("secret store should not be queried when OAuth is configured")
        })
        .unwrap();

        assert!(configured);
    }

    #[test]
    fn has_configured_token_accepts_secret_fallback() {
        let configured =
            has_configured_token_from_sources(false, || Ok(Some("hf-token".to_string()))).unwrap();

        assert!(configured);
    }

    #[test]
    fn has_configured_token_rejects_missing_oauth_and_secret() {
        let configured = has_configured_token_from_sources(false, || Ok(None)).unwrap();

        assert!(!configured);
    }

    #[cfg(unix)]
    #[test]
    fn save_oauth_token_restricts_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        with_token_path(|path| {
            save_oauth_token_to_path(
                &path,
                &HuggingFaceTokenData {
                    access_token: "saved".to_string(),
                    refresh_token: None,
                    expires_at: None,
                },
            )
            .unwrap();

            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        });
    }

    #[test]
    fn resolver_prefers_provider_token_over_oauth() {
        let token = resolve_token_from_sources(
            Some("api-key".to_string()),
            Some("oauth".to_string()),
            || panic!("secret store should not be queried when provider token is usable"),
        )
        .unwrap();

        assert_eq!(token.as_deref(), Some("api-key"));
    }

    #[test]
    fn resolver_uses_oauth_before_secret_store() {
        let token = resolve_token_from_sources(None, Some("oauth".to_string()), || {
            panic!("secret store should not be queried when OAuth is usable")
        })
        .unwrap();

        assert_eq!(token.as_deref(), Some("oauth"));
    }

    #[test]
    fn resolver_uses_secret_store_when_no_provider_token_or_oauth_exists() {
        let token = resolve_token_from_sources(None, None, || Ok(Some("secret-store".to_string())))
            .unwrap();

        assert_eq!(token.as_deref(), Some("secret-store"));
    }

    #[tokio::test]
    async fn async_resolver_uses_secret_fallback_when_oauth_refresh_fails() {
        let token = resolve_token_async_from_sources(
            None,
            async { Err(anyhow::anyhow!("refresh token revoked")) },
            || Ok(Some("secret-store".to_string())),
        )
        .await
        .unwrap();

        assert_eq!(token.as_deref(), Some("secret-store"));
    }

    #[tokio::test]
    async fn async_resolver_reports_refresh_error_without_secret_fallback() {
        let error = resolve_token_async_from_sources(
            None,
            async { Err(anyhow::anyhow!("refresh token revoked")) },
            || Ok(None),
        )
        .await
        .unwrap_err();

        assert_eq!(error.to_string(), "refresh token revoked");
    }
}
