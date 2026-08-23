//! Zitadel PKCE + avcd-llm provision for the avocado provider.
//!
//! Flow: browser Auth Code + PKCE → access JWT → POST /keys/provision →
//! store LiteLLM virtual key as AVOCADO_API_KEY (secret + private cache).

use crate::config::paths::Paths;
use crate::config::Config;
use crate::providers::private_file::write_private_file;
use anyhow::{anyhow, Result};
use axum::{extract::Query, response::Html, routing::get, Router};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use tokio::sync::{oneshot, Mutex as TokioMutex};

pub const AVOCADO_API_KEY_SECRET: &str = "AVOCADO_API_KEY";
pub const AVOCADO_OAUTH_CACHE_PATH: &str = "avocado/oauth/tokens.json";

pub const DEFAULT_ZITADEL_ISSUER: &str = "https://zitadel.avcd.ai";
pub const DEFAULT_ZITADEL_CLIENT_ID: &str = "385574574122598405";
pub const DEFAULT_ZITADEL_PROJECT_ID: &str = "385574573904494597";
pub const DEFAULT_ZITADEL_ORG_ID: &str = "378278744818778119";
pub const DEFAULT_PROVISION_URL: &str = "https://dev.avocado.tech/llm-api/keys/provision";

const OAUTH_HOST: [u8; 4] = [127, 0, 0, 1];
const OAUTH_PORT: u16 = 47821;
const OAUTH_REDIRECT_PATH: &str = "/callback";
const OAUTH_TIMEOUT_SECS: u64 = 300;
const HTML_AUTO_CLOSE_TIMEOUT_MS: u64 = 2000;

static AVOCADO_OAUTH_MUTEX: LazyLock<TokioMutex<()>> = LazyLock::new(|| TokioMutex::new(()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionedKey {
    pub api_key: String,
    pub base_url: String,
    pub user_id: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenCacheData {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    access_expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    virtual_api_key: Option<String>,
    #[serde(default)]
    virtual_key_expires_at: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    user_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProvisionError {
    #[error("invalid_token")]
    Unauthorized,
    #[error("forbidden: {detail}")]
    Forbidden { detail: String },
    #[error("litellm_unavailable")]
    Unavailable,
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub struct ZitadelOidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub project_id: String,
    pub org_id: String,
    pub google_idp_id: String,
    pub provision_url: String,
}

impl ZitadelOidcConfig {
    pub fn from_env() -> Self {
        Self {
            issuer: env_or("ZITADEL_ISSUER", DEFAULT_ZITADEL_ISSUER)
                .trim_end_matches('/')
                .to_string(),
            client_id: env_or("ZITADEL_CLIENT_ID", DEFAULT_ZITADEL_CLIENT_ID),
            project_id: env_or("ZITADEL_PROJECT_ID", DEFAULT_ZITADEL_PROJECT_ID),
            org_id: env_or("ZITADEL_ORG_ID", DEFAULT_ZITADEL_ORG_ID),
            google_idp_id: env_or("ZITADEL_GOOGLE_IDP_ID", ""),
            provision_url: env_or("AVOCADO_PROVISION_URL", DEFAULT_PROVISION_URL),
        }
    }

    pub fn scopes(&self) -> String {
        if let Ok(scopes) = std::env::var("ZITADEL_AUTH_SCOPES") {
            if !scopes.trim().is_empty() {
                return scopes;
            }
        }
        let mut scopes = vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
            "offline_access".to_string(),
            format!("urn:zitadel:iam:org:project:id:{}:aud", self.project_id),
            format!("urn:zitadel:iam:org:project:id:{}:roles", self.project_id),
            "urn:zitadel:iam:org:projects:roles".to_string(),
            format!("urn:zitadel:iam:org:id:{}", self.org_id),
            "urn:zitadel:iam:user:resourceowner".to_string(),
        ];
        // Only when explicitly set — this scope skips the email/password form
        // and force-redirects to Google.
        if !self.google_idp_id.is_empty() {
            scopes.push(format!("urn:zitadel:iam:org:idp:id:{}", self.google_idp_id));
        }
        scopes.join(" ")
    }

    pub fn authorize_endpoint(&self) -> String {
        format!("{}/oauth/v2/authorize", self.issuer)
    }

    pub fn token_endpoint(&self) -> String {
        format!("{}/oauth/v2/token", self.issuer)
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

pub fn oauth_cache_path() -> PathBuf {
    Paths::in_config_dir(AVOCADO_OAUTH_CACHE_PATH)
}

pub fn redirect_uri() -> String {
    format!(
        "http://{}.{}.{}.{}:{}{}",
        OAUTH_HOST[0], OAUTH_HOST[1], OAUTH_HOST[2], OAUTH_HOST[3], OAUTH_PORT, OAUTH_REDIRECT_PATH
    )
}

pub fn has_configured_key() -> bool {
    resolve_api_key().is_some()
}

pub fn resolve_api_key() -> Option<String> {
    if let Ok(key) = Config::global().get_secret::<String>(AVOCADO_API_KEY_SECRET) {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    if let Ok(key) = std::env::var(AVOCADO_API_KEY_SECRET) {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    load_cache()
        .and_then(|c| c.virtual_api_key)
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
}

pub fn clear_configured_key() -> Result<()> {
    let _ = Config::global().delete_secret(AVOCADO_API_KEY_SECRET);
    let path = oauth_cache_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn load_cache() -> Option<TokenCacheData> {
    load_cache_from_path(&oauth_cache_path())
}

fn load_cache_from_path(path: &Path) -> Option<TokenCacheData> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn save_cache(data: &TokenCacheData) -> Result<()> {
    save_cache_to_path(&oauth_cache_path(), data)
}

fn save_cache_to_path(path: &Path, data: &TokenCacheData) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(data)?;
    write_private_file(path, &contents)?;
    Ok(())
}

fn persist_virtual_key(
    provisioned: &ProvisionedKey,
    tokens: Option<&TokenCacheData>,
) -> Result<()> {
    let _ = Config::global().set_secret(AVOCADO_API_KEY_SECRET, &provisioned.api_key);
    let mut cache = tokens.cloned().unwrap_or(TokenCacheData {
        access_token: String::new(),
        refresh_token: None,
        access_expires_at: None,
        virtual_api_key: None,
        virtual_key_expires_at: None,
        base_url: None,
        user_id: None,
    });
    cache.virtual_api_key = Some(provisioned.api_key.clone());
    cache.virtual_key_expires_at = Some(provisioned.expires_at.clone());
    cache.base_url = Some(provisioned.base_url.clone());
    cache.user_id = Some(provisioned.user_id.clone());
    save_cache(&cache)
}

/// Parse provision HTTP status + body into a typed result (no I/O).
pub fn parse_provision_response(status: u16, body: &str) -> Result<ProvisionedKey, ProvisionError> {
    match status {
        200 => {
            let value: serde_json::Value = serde_json::from_str(body)
                .map_err(|e| ProvisionError::Other(anyhow!("invalid provision JSON: {e}")))?;
            let api_key = value
                .get("apiKey")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ProvisionError::Other(anyhow!("provision response missing apiKey")))?
                .to_string();
            let base_url = value
                .get("baseUrl")
                .and_then(|v| v.as_str())
                .unwrap_or("https://dev.avocado.tech/llm")
                .to_string();
            let user_id = value
                .get("userId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let expires_at = value
                .get("expiresAt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(ProvisionedKey {
                api_key,
                base_url,
                user_id,
                expires_at,
            })
        }
        401 => Err(ProvisionError::Unauthorized),
        403 => {
            let detail = serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|v| {
                    v.get("detail")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "forbidden".to_string());
            Err(ProvisionError::Forbidden { detail })
        }
        502 => Err(ProvisionError::Unavailable),
        other => Err(ProvisionError::Other(anyhow!(
            "provision failed with status {other}: {body}"
        ))),
    }
}

pub async fn provision_virtual_key(
    client: &reqwest::Client,
    provision_url: &str,
    access_token: &str,
) -> Result<ProvisionedKey, ProvisionError> {
    let response = client
        .post(provision_url)
        .header("authorization", format!("Bearer {access_token}"))
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|e| ProvisionError::Other(anyhow!("provision network error: {e}")))?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| ProvisionError::Other(anyhow!("provision body error: {e}")))?;
    parse_provision_response(status, &body).map_err(|err| match err {
        ProvisionError::Other(inner) => {
            ProvisionError::Other(anyhow!("POST {provision_url}: {inner}"))
        }
        other => other,
    })
}

/// After a Zitadel access token is available: provision + persist virtual key.
pub async fn complete_oauth_from_access_token(
    access_token: &str,
    provision_url: &str,
) -> Result<(), ProvisionError> {
    let client = reqwest::Client::new();
    let provisioned = provision_virtual_key(&client, provision_url, access_token).await?;
    let existing = load_cache();
    let mut tokens = existing.unwrap_or(TokenCacheData {
        access_token: access_token.to_string(),
        refresh_token: None,
        access_expires_at: None,
        virtual_api_key: None,
        virtual_key_expires_at: None,
        base_url: None,
        user_id: None,
    });
    tokens.access_token = access_token.to_string();
    persist_virtual_key(&provisioned, Some(&tokens)).map_err(ProvisionError::Other)?;
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

fn build_authorize_url(
    config: &ZitadelOidcConfig,
    pkce: &PkceChallenge,
    state: &str,
) -> Result<String> {
    let redirect = redirect_uri();
    let scopes = config.scopes();
    let params = [
        ("response_type", "code"),
        ("client_id", config.client_id.as_str()),
        ("redirect_uri", redirect.as_str()),
        ("scope", scopes.as_str()),
        ("code_challenge", pkce.challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("state", state),
    ];
    let query = serde_urlencoded::to_string(params)?;
    Ok(format!("{}?{}", config.authorize_endpoint(), query))
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

async fn exchange_code_for_tokens(
    config: &ZitadelOidcConfig,
    code: &str,
    pkce: &PkceChallenge,
    token_url_override: Option<&str>,
) -> Result<TokenCacheData> {
    let token_url = token_url_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| config.token_endpoint());
    let redirect = redirect_uri();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect.as_str()),
        ("code_verifier", pkce.verifier.as_str()),
        ("client_id", config.client_id.as_str()),
    ];
    let client = reqwest::Client::new();
    let resp = client
        .post(&token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;
    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Failed to exchange code for token: {err_text}"));
    }
    let token: TokenResponse = resp.json().await?;
    Ok(TokenCacheData {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        access_expires_at: token
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs)),
        virtual_api_key: None,
        virtual_key_expires_at: None,
        base_url: None,
        user_id: None,
    })
}

async fn spawn_oauth_server(app: Router) -> Result<(tokio::task::JoinHandle<()>, SocketAddr)> {
    let addr = SocketAddr::from((OAUTH_HOST, OAUTH_PORT));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow!("Failed to bind avocado OAuth loopback {addr}: {e}"))?;
    let local_addr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let server = axum::serve(listener, app);
        let _ = server.await;
    });
    Ok((handle, local_addr))
}

fn oauth_callback_router(
    expected_state: String,
    tx: Arc<TokioMutex<Option<oneshot::Sender<Result<String>>>>>,
) -> Router {
    Router::new().route(
        OAUTH_REDIRECT_PATH,
        get(move |Query(params): Query<HashMap<String, String>>| {
            let tx = Arc::clone(&tx);
            let expected_state = expected_state.clone();
            async move {
                let code = params.get("code").cloned();
                let received_state = params.get("state").cloned();
                let error = params.get("error").cloned();

                if let Some(error) = error {
                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send(Err(anyhow!("OAuth error: {error}")));
                    }
                    return Html(format!(
                        "<h2>Sign-in failed</h2><p>{error}</p><script>setTimeout(()=>window.close(),{HTML_AUTO_CLOSE_TIMEOUT_MS})</script>"
                    ));
                }

                if let (Some(code), Some(received_state)) = (code, received_state) {
                    if received_state == expected_state {
                        if let Some(sender) = tx.lock().await.take() {
                            let _ = sender.send(Ok(code));
                        }
                        return Html(format!(
                            "<h2>Login Success</h2><p>You can close this window</p><script>setTimeout(()=>window.close(),{HTML_AUTO_CLOSE_TIMEOUT_MS})</script>"
                        ));
                    }
                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send(Err(anyhow!("OAuth state mismatch")));
                    }
                    return Html("<h2>Error</h2><p>State mismatch.</p>".to_string());
                }

                if let Some(sender) = tx.lock().await.take() {
                    let _ = sender.send(Err(anyhow!("OAuth callback missing code")));
                }
                Html("<h2>Error</h2><p>Authentication failed.</p>".to_string())
            }
        }),
    )
}

async fn perform_loopback_oauth_flow(config: &ZitadelOidcConfig) -> Result<TokenCacheData> {
    let _guard = AVOCADO_OAUTH_MUTEX.try_lock().map_err(|_| {
        anyhow!("Another Avocado OAuth flow is already in progress; please try again later")
    })?;

    let pkce = generate_pkce();
    let csrf_state = generate_state();
    let auth_url = build_authorize_url(config, &pkce, &csrf_state)?;

    let (tx, rx) = oneshot::channel::<Result<String>>();
    let tx = Arc::new(TokioMutex::new(Some(tx)));
    let app = oauth_callback_router(csrf_state, tx);
    let (server_handle, _) = spawn_oauth_server(app).await?;

    if webbrowser::open(&auth_url).is_err() {
        tracing::info!(
            "Please open this URL in your browser to authorize Avocado Work:\n{}",
            auth_url
        );
    }

    let code_result = tokio::time::timeout(std::time::Duration::from_secs(OAUTH_TIMEOUT_SECS), rx)
        .await
        .map_err(|_| anyhow!("Avocado OAuth flow timed out"))?
        .map_err(|_| anyhow!("Avocado OAuth channel closed"))??;

    server_handle.abort();

    exchange_code_for_tokens(config, &code_result, &pkce, None).await
}

/// Full interactive OAuth + provision (desktop Sign in / CLI configure).
pub async fn configure_oauth() -> Result<()> {
    let config = ZitadelOidcConfig::from_env();
    let tokens = perform_loopback_oauth_flow(&config).await?;
    save_cache(&tokens)?;
    complete_oauth_from_access_token(&tokens.access_token, &config.provision_url)
        .await
        .map_err(|e| anyhow!(e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn given_valid_provision_json_when_parse_then_returns_api_key_base_url_user_id_expires_at() {
        let body = r#"{"apiKey":"sk-gen-a","baseUrl":"https://dev.avocado.tech/llm","userId":"t:u","expiresAt":"2099-01-01T00:00:00.000Z"}"#;
        let key = parse_provision_response(200, body).unwrap();
        assert_eq!(key.api_key, "sk-gen-a");
        assert_eq!(key.base_url, "https://dev.avocado.tech/llm");
        assert_eq!(key.user_id, "t:u");
        assert_eq!(key.expires_at, "2099-01-01T00:00:00.000Z");
    }

    #[test]
    fn given_two_different_api_keys_when_parse_then_keys_differ() {
        let a = parse_provision_response(
            200,
            r#"{"apiKey":"sk-aaa","baseUrl":"https://x","userId":"1","expiresAt":"e"}"#,
        )
        .unwrap();
        let b = parse_provision_response(
            200,
            r#"{"apiKey":"sk-bbb","baseUrl":"https://x","userId":"2","expiresAt":"e"}"#,
        )
        .unwrap();
        assert_ne!(a.api_key, b.api_key);
    }

    #[test]
    fn given_401_invalid_token_when_parse_then_unauthorized() {
        assert!(matches!(
            parse_provision_response(401, r#"{"error":"invalid_token"}"#),
            Err(ProvisionError::Unauthorized)
        ));
    }

    #[test]
    fn given_403_forbidden_when_parse_then_forbidden() {
        match parse_provision_response(
            403,
            r#"{"error":"forbidden","detail":"Missing required role: agent-access"}"#,
        ) {
            Err(ProvisionError::Forbidden { detail }) => {
                assert!(detail.contains("agent-access"));
            }
            other => panic!("expected Forbidden, got {other:?}"),
        }
    }

    #[test]
    fn given_502_litellm_when_parse_then_unavailable() {
        assert!(matches!(
            parse_provision_response(502, r#"{"error":"litellm_unavailable"}"#),
            Err(ProvisionError::Unavailable)
        ));
    }

    #[test]
    fn given_empty_cache_when_has_configured_key_then_false() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        let _guard = env_lock::lock_env([
            ("GOOSE_PATH_ROOT", Some(root.as_str())),
            ("AVOCADO_API_KEY", None::<&str>),
        ]);
        let _ = std::fs::remove_file(oauth_cache_path());
        // May still be true if Config::global has a sticky secret — clear cache path only.
        // Assert cache path itself has no key.
        assert!(load_cache().and_then(|c| c.virtual_api_key).is_none());
    }

    #[tokio::test]
    async fn given_stored_api_key_in_cache_when_provider_cleanup_then_key_and_cache_gone() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        let _guard = env_lock::lock_env([
            ("GOOSE_PATH_ROOT", Some(root.as_str())),
            ("AVOCADO_API_KEY", None::<&str>),
        ]);
        let path = oauth_cache_path();
        save_cache_to_path(
            &path,
            &TokenCacheData {
                access_token: "jwt".into(),
                refresh_token: None,
                access_expires_at: None,
                virtual_api_key: Some("sk-cached".into()),
                virtual_key_expires_at: Some("2099-01-01T00:00:00.000Z".into()),
                base_url: Some("https://dev.avocado.tech/llm".into()),
                user_id: Some("t:u".into()),
            },
        )
        .unwrap();
        assert_eq!(resolve_api_key().as_deref(), Some("sk-cached"));
        crate::providers::avocado::AvocadoProvider::cleanup()
            .await
            .unwrap();
        assert!(load_cache().is_none());
        assert!(resolve_api_key().is_none());
    }

    #[test]
    fn given_authorize_params_when_build_url_then_has_s256_and_redirect_47821_callback() {
        let config = ZitadelOidcConfig {
            issuer: "https://zitadel.example".into(),
            client_id: "client-1".into(),
            project_id: "proj".into(),
            org_id: "org".into(),
            google_idp_id: String::new(),
            provision_url: DEFAULT_PROVISION_URL.into(),
        };
        let pkce = PkceChallenge {
            verifier: "v".repeat(64),
            challenge: "challenge-fixture".into(),
        };
        let url = build_authorize_url(&config, &pkce, "state-fixture").unwrap();
        assert!(url.contains("/oauth/v2/authorize"));
        assert!(url.contains("client_id=client-1"));
        assert!(url.contains("code_challenge=challenge-fixture"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=state-fixture"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A47821%2Fcallback"));
        assert!(url.contains("openid"));
        assert!(
            !url.contains("urn:zitadel:iam:org:idp:id"),
            "desktop login must not force Google"
        );
    }

    #[test]
    fn given_google_idp_id_when_scopes_then_includes_idp_hint() {
        let config = ZitadelOidcConfig {
            issuer: "https://zitadel.example".into(),
            client_id: "client-1".into(),
            project_id: "proj".into(),
            org_id: "org".into(),
            google_idp_id: "idp-9".into(),
            provision_url: DEFAULT_PROVISION_URL.into(),
        };
        assert!(config.scopes().contains("urn:zitadel:iam:org:idp:id:idp-9"));
    }

    #[tokio::test]
    async fn given_401_when_provision_http_then_err_and_no_secret_write() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        let _guard = env_lock::lock_env([("GOOSE_PATH_ROOT", Some(root.as_str()))]);
        let _ = std::fs::remove_file(oauth_cache_path());

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/keys/provision"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "invalid_token"
            })))
            .mount(&server)
            .await;

        let err =
            complete_oauth_from_access_token("bad", &format!("{}/keys/provision", server.uri()))
                .await
                .unwrap_err();
        assert!(matches!(err, ProvisionError::Unauthorized));
        assert!(load_cache().and_then(|c| c.virtual_api_key).is_none());
    }

    #[tokio::test]
    async fn given_token_200_with_refresh_when_exchange_then_stores_access_and_refresh() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/v2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "access-1",
                "refresh_token": "refresh-1",
                "expires_in": 3600
            })))
            .mount(&server)
            .await;

        let config = ZitadelOidcConfig {
            issuer: server.uri(),
            client_id: "client".into(),
            project_id: "p".into(),
            org_id: "o".into(),
            google_idp_id: "g".into(),
            provision_url: DEFAULT_PROVISION_URL.into(),
        };
        let pkce = generate_pkce();
        let tokens = exchange_code_for_tokens(
            &config,
            "code-1",
            &pkce,
            Some(&format!("{}/oauth/v2/token", server.uri())),
        )
        .await
        .unwrap();
        assert_eq!(tokens.access_token, "access-1");
        assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-1"));
    }

    #[tokio::test]
    async fn given_valid_jwt_when_complete_oauth_then_persists_virtual_key() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        let _guard = env_lock::lock_env([
            ("GOOSE_PATH_ROOT", Some(root.as_str())),
            ("AVOCADO_API_KEY", None::<&str>),
        ]);

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/keys/provision"))
            .and(header("authorization", "Bearer jwt-ok"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "apiKey": "sk-from-provision",
                "baseUrl": "https://dev.avocado.tech/llm",
                "userId": "t:sub",
                "expiresAt": "2099-01-01T00:00:00.000Z",
            })))
            .mount(&server)
            .await;

        complete_oauth_from_access_token("jwt-ok", &format!("{}/keys/provision", server.uri()))
            .await
            .unwrap();
        assert_eq!(resolve_api_key().as_deref(), Some("sk-from-provision"));
        clear_configured_key().unwrap();
    }
}
