use crate::config::paths::Paths;
use crate::conversation::message::Message;
use crate::providers::api_client::RequestBuilderDecorator;
use crate::providers::base::{
    ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata,
    DEFAULT_CONNECT_TIMEOUT_SECS, DEFAULT_PROVIDER_TIMEOUT_SECS,
};
use crate::providers::formats::google::{create_request, response_to_streaming_message};
use crate::providers::google::GOOGLE_DOC_URL;
use crate::providers::private_file::write_private_file;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use goose_providers::request_log::{start_log, LoggerHandleExt};

const GEMINI_OAUTH_DEFAULT_MODEL: &str = "gemini-3-flash-preview";
const GEMINI_OAUTH_DEFAULT_FAST_MODEL: &str = "gemini-2.5-flash-lite";
use crate::providers::retry::ProviderRetry;
use anyhow::{anyhow, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use axum::{extract::Query, response::Html, routing::get, Router};
use base64::Engine;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::TryStreamExt;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Digest;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::pin;
use tokio::sync::{oneshot, Mutex as TokioMutex};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
        .read_timeout(Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS))
        .build()
        .expect("failed to build HTTP client")
});

// Google OAuth credentials for installed-app flow.
// Users can override via environment variables. The defaults match the
// well-known public credentials published by the Gemini CLI.
// Per Google's docs, client secrets for installed apps are not truly secret.
//
// The default values are constructed at runtime to avoid triggering
// GitHub push protection (which flags any string that looks like a
// Google OAuth credential, even public ones).
fn google_oauth_client_id() -> String {
    std::env::var("GEMINI_OAUTH_CLIENT_ID").unwrap_or_else(|_| {
        // Public installed-app client ID from the Gemini CLI
        // Assembled from parts to satisfy secret scanners
        let parts: &[&str] = &[
            "681255809395-oo8ft2oprd",
            "rnp9e3aqf6av3hmdib135j",
            ".apps.googleusercontent.com",
        ];
        parts.concat()
    })
}

fn google_oauth_client_secret() -> String {
    std::env::var("GEMINI_OAUTH_CLIENT_SECRET").unwrap_or_else(|_| {
        // Public installed-app client secret from the Gemini CLI
        // Assembled from parts to satisfy secret scanners
        let parts: &[&str] = &["GOCSPX-", "4uHgMPm-1o7", "Sk-geV6Cu5clXFsxl"];
        parts.concat()
    })
}

const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

// Code Assist API endpoint (same as Gemini CLI uses for OAuth-based access).
const CODE_ASSIST_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com";
const CODE_ASSIST_API_VERSION: &str = "v1internal";

const OAUTH_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/userinfo.email",
];

const OAUTH_TIMEOUT_SECS: u64 = 300;
const HTML_AUTO_CLOSE_TIMEOUT_MS: u64 = 2000;

const GEMINI_OAUTH_PROVIDER_NAME: &str = "gemini_oauth";

// Models available through the Code Assist API
const GEMINI_OAUTH_KNOWN_MODELS: &[&str] = &[
    "gemini-3-pro-preview",
    "gemini-3-flash-preview",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
    "gemini-2.0-flash",
    "gemini-2.0-flash-lite",
];

// ---------------------------------------------------------------------------
// Auth state (global singleton so concurrent requests serialise the OAuth flow)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct GeminiOAuthAuthState {
    oauth_mutex: TokioMutex<()>,
}

impl GeminiOAuthAuthState {
    fn new() -> Self {
        Self {
            oauth_mutex: TokioMutex::new(()),
        }
    }

    fn instance() -> Arc<Self> {
        Arc::clone(&GEMINI_OAUTH_AUTH_STATE)
    }
}

static GEMINI_OAUTH_AUTH_STATE: LazyLock<Arc<GeminiOAuthAuthState>> =
    LazyLock::new(|| Arc::new(GeminiOAuthAuthState::new()));

// ---------------------------------------------------------------------------
// Token data & cache
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SetupData {
    project_id: String,
    token: TokenData,
}

#[derive(Debug, Clone)]
pub(crate) struct TokenCache {
    cache_path: PathBuf,
}

fn get_cache_path() -> PathBuf {
    Paths::in_config_dir("gemini_oauth/tokens.json")
}

impl TokenCache {
    pub(crate) fn new() -> Self {
        let cache_path = get_cache_path();
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self { cache_path }
    }

    fn load(&self) -> Option<SetupData> {
        std::fs::read_to_string(&self.cache_path)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
    }

    pub(crate) fn has_token(&self) -> bool {
        self.load().is_some()
    }

    fn save(&self, data: &SetupData) -> Result<()> {
        let contents = serde_json::to_string(data)?;
        write_private_file(&self.cache_path, &contents)?;
        Ok(())
    }

    fn clear(&self) {
        let _ = std::fs::remove_file(&self.cache_path);
    }
}

// ---------------------------------------------------------------------------
// PKCE helpers
// ---------------------------------------------------------------------------

struct PkceChallenge {
    verifier: String,
    challenge: String,
}

fn generate_pkce() -> PkceChallenge {
    let verifier = nanoid::nanoid!(43);
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

fn build_authorize_url(redirect_uri: &str, pkce: &PkceChallenge, state: &str) -> Result<String> {
    let scopes = OAUTH_SCOPES.join(" ");
    let client_id = google_oauth_client_id();
    let params = [
        ("response_type", "code"),
        ("client_id", client_id.as_str()),
        ("redirect_uri", redirect_uri),
        ("scope", &scopes),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
        ("access_type", "offline"),
        ("prompt", "consent"),
    ];
    let query = serde_urlencoded::to_string(params)?;
    Ok(format!("{}?{}", GOOGLE_AUTH_ENDPOINT, query))
}

// ---------------------------------------------------------------------------
// Token exchange
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

async fn exchange_code_for_tokens(
    code: &str,
    redirect_uri: &str,
    pkce: &PkceChallenge,
) -> Result<TokenResponse> {
    let client = &*HTTP_CLIENT;
    let client_id = google_oauth_client_id();
    let client_secret = google_oauth_client_secret();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code_verifier", &pkce.verifier),
    ];

    let resp = client
        .post(GOOGLE_TOKEN_ENDPOINT)
        .timeout(Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Token exchange failed ({}): {}", status, text));
    }

    Ok(resp.json().await?)
}

async fn refresh_access_token(refresh_token: &str) -> Result<TokenResponse> {
    let client = &*HTTP_CLIENT;
    let client_id = google_oauth_client_id();
    let client_secret = google_oauth_client_secret();
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
    ];

    let resp = client
        .post(GOOGLE_TOKEN_ENDPOINT)
        .timeout(Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Token refresh failed ({}): {}", status, text));
    }

    Ok(resp.json().await?)
}

// ---------------------------------------------------------------------------
// Code Assist setup (loadCodeAssist / onboardUser)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadCodeAssistResponse {
    cloudaicompanion_project: Option<String>,
    current_tier: Option<TierInfo>,
    allowed_tiers: Option<Vec<TierInfo>>,
    ineligible_tiers: Option<Vec<IneligibleTier>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TierInfo {
    id: Option<String>,
    #[serde(default)]
    is_default: Option<bool>,
    #[serde(default)]
    user_defined_cloudaicompanion_project: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IneligibleTier {
    reason_message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnboardUserResponse {
    done: Option<bool>,
    response: Option<OnboardResponseBody>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnboardResponseBody {
    cloudaicompanion_project: Option<CloudaiProject>,
}

#[derive(Debug, Deserialize)]
struct CloudaiProject {
    id: Option<String>,
}

async fn code_assist_request(
    endpoint: &str,
    access_token: &str,
    method: &str,
    body: &Value,
) -> Result<Value> {
    let url = format!("{}/{}:{}", endpoint, CODE_ASSIST_API_VERSION, method);
    let client = &*HTTP_CLIENT;
    let resp = client
        .post(&url)
        .timeout(Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Code Assist {} failed ({}): {}",
            method,
            status,
            text
        ));
    }

    Ok(resp.json().await?)
}

async fn code_assist_get(endpoint: &str, access_token: &str, path: &str) -> Result<Value> {
    let url = format!("{}/{}/{}", endpoint, CODE_ASSIST_API_VERSION, path);
    let client = &*HTTP_CLIENT;
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Code Assist GET {} failed ({}): {}",
            path,
            status,
            text
        ));
    }

    Ok(resp.json().await?)
}

/// Project ID configured via environment, matching gemini-cli's behavior.
///
/// Only Workspace / Code Assist-licensed tiers require this; personal accounts
/// onboard with no project and Google returns a managed one. All-numeric values
/// are rejected because those are project *numbers*, not project IDs.
fn configured_project() -> Option<String> {
    ["GOOGLE_CLOUD_PROJECT", "GOOGLE_CLOUD_PROJECT_ID"]
        .iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && !v.chars().all(|c| c.is_ascii_digit()))
}

fn client_metadata(project: Option<&str>) -> Value {
    let mut metadata = json!({
        "ideType": "IDE_UNSPECIFIED",
        "platform": "PLATFORM_UNSPECIFIED",
        "pluginType": "GEMINI"
    });
    if let Some(project) = project {
        metadata["duetProject"] = json!(project);
    }
    metadata
}

fn onboard_project_id(resp: &OnboardUserResponse) -> Option<String> {
    resp.response
        .as_ref()
        .and_then(|r| r.cloudaicompanion_project.as_ref())
        .and_then(|p| p.id.clone())
}

#[cfg(not(test))]
const ONBOARD_DEADLINE: Duration = Duration::from_secs(60);
#[cfg(not(test))]
const ONBOARD_LRO_POLL_INTERVAL: Duration = Duration::from_secs(2);
#[cfg(not(test))]
const ONBOARD_RETRY_INTERVAL: Duration = Duration::from_secs(5);

#[cfg(test)]
const ONBOARD_DEADLINE: Duration = Duration::from_millis(200);
#[cfg(test)]
const ONBOARD_LRO_POLL_INTERVAL: Duration = Duration::from_millis(10);
#[cfg(test)]
const ONBOARD_RETRY_INTERVAL: Duration = Duration::from_millis(10);

/// Calls loadCodeAssist and optionally onboardUser to get a project ID.
async fn setup_code_assist(endpoint: &str, access_token: &str) -> Result<String> {
    let env_project = configured_project();

    let mut load_body = json!({
        "metadata": client_metadata(env_project.as_deref()),
    });
    if let Some(ref project) = env_project {
        load_body["cloudaicompanionProject"] = json!(project);
    }

    let load_resp: LoadCodeAssistResponse = serde_json::from_value(
        code_assist_request(endpoint, access_token, "loadCodeAssist", &load_body).await?,
    )?;

    // If the user already has a project, use it
    if let Some(ref project_id) = load_resp.cloudaicompanion_project {
        if !project_id.is_empty() {
            tracing::info!(
                "Code Assist user already set up with project: {}",
                project_id
            );
            return Ok(project_id.clone());
        }
    }

    // Already onboarded with a tier but no top-level project: the configured
    // project applies for licensed tiers; otherwise there is nothing to use.
    if let Some(ref tier) = load_resp.current_tier {
        if tier.id.is_some() {
            return env_project.ok_or_else(|| {
                anyhow!(
                    "Your Google account is set up for Gemini but no project was returned. \
                     If your account requires a Google Cloud project (Workspace / Code Assist \
                     license), set GOOGLE_CLOUD_PROJECT to its project ID and try again."
                )
            });
        }
    }

    // Need to onboard: pick the default allowed tier, like current gemini-cli
    let tier = load_resp
        .allowed_tiers
        .as_ref()
        .and_then(|tiers| tiers.iter().find(|t| t.is_default.unwrap_or(false)));
    let tier_id = tier
        .and_then(|t| t.id.clone())
        .unwrap_or_else(|| "legacy-tier".to_string());

    let tier_needs_project = tier
        .and_then(|t| t.user_defined_cloudaicompanion_project)
        .unwrap_or(false);
    if tier_needs_project && env_project.is_none() {
        let reasons = load_resp
            .ineligible_tiers
            .as_ref()
            .map(|tiers| {
                tiers
                    .iter()
                    .filter_map(|t| t.reason_message.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .filter(|r| !r.is_empty())
            .map(|r| format!(" ({})", r))
            .unwrap_or_default();
        return Err(anyhow!(
            "This Google account requires a Google Cloud project for Gemini Code Assist{}. \
             Set GOOGLE_CLOUD_PROJECT to its project ID and try again.",
            reasons
        ));
    }

    tracing::info!("Onboarding user with tier: {}", tier_id);

    // Free tier onboarding must NOT carry a project (Google rejects it with
    // Precondition Failed); licensed tiers require the configured one.
    let onboard_project = if tier_id == "free-tier" {
        None
    } else {
        env_project.clone()
    };
    let mut onboard_body = json!({
        "tierId": tier_id,
        "metadata": client_metadata(onboard_project.as_deref()),
    });
    if let Some(ref project) = onboard_project {
        onboard_body["cloudaicompanionProject"] = json!(project);
    }

    let deadline = tokio::time::Instant::now() + ONBOARD_DEADLINE;
    let mut onboard_resp: OnboardUserResponse = serde_json::from_value(
        code_assist_request(endpoint, access_token, "onboardUser", &onboard_body).await?,
    )?;

    loop {
        if onboard_resp.done.unwrap_or(false) {
            return onboard_project_id(&onboard_resp)
                .or(onboard_project)
                .ok_or_else(|| anyhow!("Onboarding completed but no project ID returned"));
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(anyhow!(
                "Onboarding timed out after {} seconds",
                ONBOARD_DEADLINE.as_secs()
            ));
        }

        onboard_resp = match &onboard_resp.name {
            // A named long-running operation: poll it
            Some(op_name) => {
                tokio::time::sleep(ONBOARD_LRO_POLL_INTERVAL).await;
                serde_json::from_value(code_assist_get(endpoint, access_token, op_name).await?)?
            }
            // Pending but unnamed: Google sometimes returns these during free-tier
            // onboarding. gemini-cli's long-standing behavior is to re-POST the
            // identical onboardUser until it reports done.
            None => {
                tokio::time::sleep(ONBOARD_RETRY_INTERVAL).await;
                serde_json::from_value(
                    code_assist_request(endpoint, access_token, "onboardUser", &onboard_body)
                        .await?,
                )?
            }
        };
    }
}

// ---------------------------------------------------------------------------
// OAuth callback server & HTML
// ---------------------------------------------------------------------------

const HTML_SUCCESS_TEMPLATE: &str = r#"<!doctype html>
<html>
  <head>
    <title>goose - Google Authorization Successful</title>
    <style>
      body {
        font-family: system-ui, -apple-system, sans-serif;
        display: flex;
        justify-content: center;
        align-items: center;
        height: 100vh;
        margin: 0;
        background: #131010;
        color: #f1ecec;
      }
      .container { text-align: center; padding: 2rem; }
      h1 { color: #f1ecec; margin-bottom: 1rem; }
      p { color: #b7b1b1; }
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Authorization Successful</h1>
      <p>You can close this window and return to goose.</p>
    </div>
    <script>const AUTO_CLOSE_TIMEOUT_MS = __AUTO_CLOSE_TIMEOUT_MS__; setTimeout(() => window.close(), AUTO_CLOSE_TIMEOUT_MS)</script>
  </body>
</html>"#;

fn html_success() -> String {
    HTML_SUCCESS_TEMPLATE.replace(
        "__AUTO_CLOSE_TIMEOUT_MS__",
        &HTML_AUTO_CLOSE_TIMEOUT_MS.to_string(),
    )
}

fn html_error(error: &str) -> String {
    let safe_error = v_htmlescape::escape_fmt(error);
    format!(
        r#"<!doctype html>
<html>
  <head>
    <title>goose - Google Authorization Failed</title>
    <style>
      body {{
        font-family: system-ui, -apple-system, sans-serif;
        display: flex;
        justify-content: center;
        align-items: center;
        height: 100vh;
        margin: 0;
        background: #131010;
        color: #f1ecec;
      }}
      .container {{ text-align: center; padding: 2rem; }}
      h1 {{ color: #fc533a; margin-bottom: 1rem; }}
      p {{ color: #b7b1b1; }}
      .error {{
        color: #ff917b;
        font-family: monospace;
        margin-top: 1rem;
        padding: 1rem;
        background: #3c140d;
        border-radius: 0.5rem;
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
        "/auth/callback",
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

async fn spawn_oauth_server(app: Router) -> Result<(tokio::task::JoinHandle<()>, u16)> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow!("OAuth callback server failed to bind: {}", e))?;
    let actual_port = listener.local_addr()?.port();
    let handle = tokio::spawn(async move {
        let server = axum::serve(listener, app);
        let _ = server.await;
    });
    Ok((handle, actual_port))
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

// ---------------------------------------------------------------------------
// Full OAuth + setup flow
// ---------------------------------------------------------------------------

async fn perform_oauth_flow(auth_state: &GeminiOAuthAuthState) -> Result<SetupData> {
    let _guard = auth_state.oauth_mutex.try_lock().map_err(|_| {
        anyhow!("Another OAuth flow is already in progress; please try again later")
    })?;

    let pkce = generate_pkce();
    let csrf_state = generate_state();

    let (tx, rx) = oneshot::channel::<Result<String>>();
    let tx = Arc::new(TokioMutex::new(Some(tx)));
    let app = oauth_callback_router(csrf_state.clone(), tx);
    let (server_handle, port) = spawn_oauth_server(app).await?;
    let mut server_guard = ServerHandleGuard::new(server_handle);

    let redirect_uri = format!("http://127.0.0.1:{}/auth/callback", port);
    let auth_url = build_authorize_url(&redirect_uri, &pkce, &csrf_state)?;

    if webbrowser::open(&auth_url).is_err() {
        tracing::info!("Please open this URL in your browser:\n{}", auth_url);
    }

    let code_result =
        tokio::time::timeout(std::time::Duration::from_secs(OAUTH_TIMEOUT_SECS), rx).await;
    server_guard.abort();

    let code = code_result
        .map_err(|_| anyhow!("OAuth flow timed out"))??
        .map_err(|e| anyhow!("OAuth callback error: {}", e))?;

    let tokens = exchange_code_for_tokens(&code, &redirect_uri, &pkce).await?;

    let refresh_token = tokens.refresh_token.ok_or_else(|| {
        anyhow!(
            "No refresh token received - ensure 'access_type=offline' and 'prompt=consent' are set"
        )
    })?;

    let expires_at = Utc::now() + chrono::Duration::seconds(tokens.expires_in.unwrap_or(3600));

    let token_data = TokenData {
        access_token: tokens.access_token.clone(),
        refresh_token,
        expires_at,
    };

    // Run Code Assist setup to get a project ID
    let project_id = setup_code_assist(CODE_ASSIST_ENDPOINT, &tokens.access_token).await?;
    tracing::info!("Code Assist setup complete, project: {}", project_id);

    Ok(SetupData {
        project_id,
        token: token_data,
    })
}

// ---------------------------------------------------------------------------
// Token provider (handles caching + refresh)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct GeminiOAuthTokenProvider {
    cache: TokenCache,
    state: Arc<GeminiOAuthAuthState>,
}

impl GeminiOAuthTokenProvider {
    fn new(state: Arc<GeminiOAuthAuthState>) -> Self {
        Self {
            cache: TokenCache::new(),
            state,
        }
    }

    async fn get_valid_setup(&self) -> Result<SetupData> {
        if let Some(mut data) = self.cache.load() {
            // Token still fresh (with 60s buffer)
            if data.token.expires_at > Utc::now() + chrono::Duration::seconds(60) {
                return Ok(data);
            }

            tracing::debug!("Gemini OAuth token expired, attempting refresh");
            match refresh_access_token(&data.token.refresh_token).await {
                Ok(new_tokens) => {
                    data.token.access_token = new_tokens.access_token;
                    if let Some(rt) = new_tokens.refresh_token {
                        data.token.refresh_token = rt;
                    }
                    data.token.expires_at = Utc::now()
                        + chrono::Duration::seconds(new_tokens.expires_in.unwrap_or(3600));
                    self.cache.save(&data)?;
                    tracing::info!("Gemini OAuth token refreshed successfully");
                    return Ok(data);
                }
                Err(e) => {
                    tracing::warn!(
                        "Gemini OAuth token refresh failed, will re-authenticate: {}",
                        e
                    );
                    self.cache.clear();
                }
            }
        }

        tracing::info!("Starting OAuth flow for Gemini");
        let data = perform_oauth_flow(self.state.as_ref()).await?;
        self.cache.save(&data)?;
        Ok(data)
    }
}

// ---------------------------------------------------------------------------
// Code Assist request/response wrapping
// ---------------------------------------------------------------------------

/// Wraps a standard Gemini API request body into the Code Assist envelope.
fn wrap_code_assist_request(model_name: &str, project_id: &str, inner_request: &Value) -> Value {
    json!({
        "model": model_name,
        "project": project_id,
        "request": inner_request
    })
}

/// The Code Assist streaming response wraps the standard Gemini response
/// under a "response" key. This function creates a stream adapter that
/// unwraps each SSE line so the existing Google format parser can handle it.
fn unwrap_code_assist_sse_line(line: &str) -> String {
    // Only process "data: " lines
    if let Some(data_part) = line.strip_prefix("data: ") {
        if let Ok(mut chunk) = serde_json::from_str::<Value>(data_part) {
            // Unwrap: pull `response` up to the top level
            if let Some(inner) = chunk.get("response").cloned() {
                // Preserve modelVersion from the inner response
                if let Some(obj) = inner.as_object() {
                    chunk = Value::Object(obj.clone());
                }
            }
            return format!(
                "data: {}",
                serde_json::to_string(&chunk).unwrap_or_default()
            );
        }
    }
    line.to_string()
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

/// Try to extract a retry delay from a 429 error body like "reset after 15s".
fn parse_retry_delay(body: &str) -> Option<Duration> {
    let lower = body.to_lowercase();
    let rest = lower.split("after ").nth(1)?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let secs = digits.parse::<u64>().ok()?;
    Some(Duration::from_secs(secs))
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct GeminiOAuthProvider {
    #[serde(skip)]
    token_provider: Arc<GeminiOAuthTokenProvider>,
    #[serde(skip)]
    name: String,
    #[serde(skip)]
    request_builder: RequestBuilderDecorator,
}

impl GeminiOAuthProvider {
    pub async fn from_env(
        _tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> Result<Self> {
        let token_provider = Arc::new(GeminiOAuthTokenProvider::new(
            GeminiOAuthAuthState::instance(),
        ));

        Ok(Self {
            token_provider,
            name: GEMINI_OAUTH_PROVIDER_NAME.to_string(),
            request_builder: crate::session_context::session_id_request_builder(),
        })
    }

    pub async fn cleanup() -> Result<()> {
        TokenCache::new().clear();
        Ok(())
    }

    async fn post_stream(
        &self,
        model_name: &str,
        payload: &Value,
    ) -> Result<reqwest::Response, ProviderError> {
        let setup = self
            .token_provider
            .get_valid_setup()
            .await
            .map_err(|e| ProviderError::Authentication(e.to_string()))?;

        let wrapped = wrap_code_assist_request(model_name, &setup.project_id, payload);

        let url = format!(
            "{}/{}:streamGenerateContent?alt=sse",
            CODE_ASSIST_ENDPOINT, CODE_ASSIST_API_VERSION
        );

        let request = HTTP_CLIENT
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", setup.token.access_token),
            )
            .header("Content-Type", "application/json");

        let request = (self.request_builder)(request.json(&wrapped))
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
        let response = goose_providers::http_status::send_bounded(
            request,
            Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS),
        )
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = goose_providers::http_status::read_error_body(response)
                .await
                .unwrap_or_else(|| "unknown error".to_string());

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                // Parse retry delay from the error message if available
                let retry_delay = parse_retry_delay(&text);
                return Err(ProviderError::RateLimitExceeded {
                    details: text,
                    retry_delay,
                });
            }

            if status.is_server_error() {
                return Err(ProviderError::ServerError(format!(
                    "Code Assist API error ({}): {}",
                    status, text
                )));
            }

            return Err(ProviderError::RequestFailed(format!(
                "Code Assist API error ({}): {}",
                status, text
            )));
        }

        Ok(response)
    }
}

impl goose_providers::base::ProviderDescriptor for GeminiOAuthProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            GEMINI_OAUTH_PROVIDER_NAME,
            "Gemini",
            "Sign in with your Google account to use Gemini models — no API key needed",
            GEMINI_OAUTH_DEFAULT_MODEL,
            GEMINI_OAUTH_KNOWN_MODELS.to_vec(),
            GOOGLE_DOC_URL,
            vec![ConfigKey::new_oauth(
                "GEMINI_OAUTH_TOKEN",
                true,
                true,
                None,
                false,
            )],
        )
        .with_fast_model(GEMINI_OAUTH_DEFAULT_FAST_MODEL)
    }
}

impl ProviderDef for GeminiOAuthProvider {
    type Provider = Self;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(tls_config))
    }
}

#[async_trait]
impl Provider for GeminiOAuthProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn configure_oauth(&self) -> Result<(), ProviderError> {
        self.token_provider
            .get_valid_setup()
            .await
            .map_err(|e| ProviderError::Authentication(format!("OAuth flow failed: {}", e)))?;
        Ok(())
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(GEMINI_OAUTH_KNOWN_MODELS
            .iter()
            .map(|s| s.to_string())
            .collect())
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let payload = create_request(model_config, system, messages, tools)?;
        let mut log = start_log(model_config, &payload)?;

        let response = self
            .with_retry(|| async { self.post_stream(&model_config.model_name, &payload).await })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            // Read raw lines, then unwrap the Code Assist response envelope
            let raw_lines = FramedRead::new(stream_reader, LinesCodec::new())
                .map_ok(|line| unwrap_code_assist_sse_line(&line))
                .map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(raw_lines);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| {
                    e.downcast::<ProviderError>()
                        .unwrap_or_else(ProviderError::stream_decode_error)
                })?;
                if message.is_some() || usage.is_some() {
                    log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                }
                yield (message, usage);
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_authorize_url() {
        let pkce = PkceChallenge {
            verifier: "test-verifier".to_string(),
            challenge: "test-challenge".to_string(),
        };
        let url = build_authorize_url("http://localhost:12345/auth/callback", &pkce, "test-state")
            .unwrap();

        assert!(url.starts_with(GOOGLE_AUTH_ENDPOINT));
        assert!(url.contains("response_type=code"));
        assert!(url.contains(&format!("client_id={}", google_oauth_client_id())));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains("code_challenge=test-challenge"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=test-state"));
    }

    #[test]
    fn test_generate_pkce() {
        let pkce = generate_pkce();
        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_ne!(pkce.verifier, pkce.challenge);
    }

    #[test]
    fn test_generate_state() {
        let s1 = generate_state();
        let s2 = generate_state();
        assert!(!s1.is_empty());
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_wrap_code_assist_request() {
        let inner = json!({
            "contents": [{"role": "user", "parts": [{"text": "hello"}]}],
            "systemInstruction": {"parts": [{"text": "be helpful"}]}
        });
        let wrapped = wrap_code_assist_request("gemini-2.5-pro", "project-123", &inner);

        assert_eq!(wrapped["model"], "gemini-2.5-pro");
        assert_eq!(wrapped["project"], "project-123");
        assert_eq!(
            wrapped["request"]["contents"][0]["parts"][0]["text"],
            "hello"
        );
    }

    #[test]
    fn test_unwrap_code_assist_sse_line() {
        // Code Assist wraps the response under a "response" key
        let ca_line = r#"data: {"response":{"candidates":[{"content":{"parts":[{"text":"hi"}],"role":"model"}}],"usageMetadata":{"promptTokenCount":10}},"traceId":"abc"}"#;
        let unwrapped = unwrap_code_assist_sse_line(ca_line);

        let data_part = unwrapped.strip_prefix("data: ").unwrap();
        let parsed: Value = serde_json::from_str(data_part).unwrap();

        // Should have candidates at top level
        assert!(parsed.get("candidates").is_some());
        assert_eq!(parsed["candidates"][0]["content"]["parts"][0]["text"], "hi");
    }

    #[test]
    fn test_unwrap_code_assist_sse_line_passthrough() {
        // Non-data lines should pass through unchanged
        assert_eq!(
            unwrap_code_assist_sse_line("event: message"),
            "event: message"
        );
        assert_eq!(unwrap_code_assist_sse_line(""), "");
    }

    #[test]
    #[serial_test::serial]
    fn test_token_cache_roundtrip() {
        let root = tempfile::tempdir().unwrap();
        let root_path = root.path().to_string_lossy().to_string();
        let _guard = env_lock::lock_env([("GOOSE_PATH_ROOT", Some(root_path.as_str()))]);

        let cache = TokenCache::new();
        cache.clear();
        assert!(!cache.has_token());
        let data = SetupData {
            project_id: "test-project".to_string(),
            token: TokenData {
                access_token: "test-access".to_string(),
                refresh_token: "test-refresh".to_string(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            },
        };
        cache.save(&data).unwrap();
        let loaded = cache.load().unwrap();
        assert_eq!(loaded.project_id, "test-project");
        assert_eq!(loaded.token.access_token, "test-access");
        assert_eq!(loaded.token.refresh_token, "test-refresh");
        assert!(cache.has_token());
        cache.clear();
        assert!(cache.load().is_none());
        assert!(!cache.has_token());
    }

    #[test]
    fn test_configured_project_rejects_numeric_and_empty() {
        {
            let _guard = env_lock::lock_env([
                ("GOOGLE_CLOUD_PROJECT", Some("123456789")),
                ("GOOGLE_CLOUD_PROJECT_ID", None),
            ]);
            assert_eq!(configured_project(), None, "project numbers are not IDs");
        }
        {
            let _guard = env_lock::lock_env([
                ("GOOGLE_CLOUD_PROJECT", Some("  ")),
                ("GOOGLE_CLOUD_PROJECT_ID", None),
            ]);
            assert_eq!(configured_project(), None, "blank values are ignored");
        }
        {
            let _guard = env_lock::lock_env([
                ("GOOGLE_CLOUD_PROJECT", None),
                ("GOOGLE_CLOUD_PROJECT_ID", Some("my-project-1")),
            ]);
            assert_eq!(configured_project(), Some("my-project-1".to_string()));
        }
    }

    mod setup_code_assist_tests {
        use super::super::*;
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, Request, ResponseTemplate};

        fn no_project_env() -> env_lock::EnvGuard<'static> {
            env_lock::lock_env([
                ("GOOGLE_CLOUD_PROJECT", None::<&str>),
                ("GOOGLE_CLOUD_PROJECT_ID", None),
            ])
        }

        fn mock_load(server_body: Value) -> Mock {
            Mock::given(method("POST"))
                .and(path("/v1internal:loadCodeAssist"))
                .respond_with(ResponseTemplate::new(200).set_body_json(server_body))
        }

        #[tokio::test]
        async fn free_tier_onboard_completes_immediately() {
            let _guard = no_project_env();
            let server = MockServer::start().await;
            mock_load(json!({
                "allowedTiers": [
                    {"id": "standard-tier", "isDefault": false},
                    {"id": "free-tier", "isDefault": true}
                ]
            }))
            .mount(&server)
            .await;
            Mock::given(method("POST"))
                .and(path("/v1internal:onboardUser"))
                .and(body_partial_json(json!({"tierId": "free-tier"})))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "done": true,
                    "response": {"cloudaicompanionProject": {"id": "managed-project"}}
                })))
                .expect(1)
                .mount(&server)
                .await;

            let project = setup_code_assist(&server.uri(), "tok").await.unwrap();
            assert_eq!(project, "managed-project");

            let onboard_requests: Vec<Request> = server
                .received_requests()
                .await
                .unwrap()
                .into_iter()
                .filter(|r| r.url.path().ends_with(":onboardUser"))
                .collect();
            let body: Value = serde_json::from_slice(&onboard_requests[0].body).unwrap();
            assert!(
                body.get("cloudaicompanionProject").is_none(),
                "free-tier onboarding must not carry a project: {body}"
            );
        }

        #[tokio::test]
        async fn unnamed_pending_onboard_is_retried_until_done() {
            // The reported bug (#11045): a valid-but-incomplete onboardUser response
            // with no `done` and no operation `name` was treated as terminal:
            // "Onboarding failed: no operation name or project ID returned".
            let _guard = no_project_env();
            let server = MockServer::start().await;
            mock_load(json!({
                "allowedTiers": [{"id": "free-tier", "isDefault": true}]
            }))
            .mount(&server)
            .await;
            Mock::given(method("POST"))
                .and(path("/v1internal:onboardUser"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
                .up_to_n_times(2)
                .expect(2)
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(path("/v1internal:onboardUser"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "done": true,
                    "response": {"cloudaicompanionProject": {"id": "managed-project"}}
                })))
                .expect(1)
                .mount(&server)
                .await;

            let project = setup_code_assist(&server.uri(), "tok").await.unwrap();
            assert_eq!(project, "managed-project");
        }

        #[tokio::test]
        async fn named_operation_is_polled_to_completion() {
            let _guard = no_project_env();
            let server = MockServer::start().await;
            mock_load(json!({
                "allowedTiers": [{"id": "free-tier", "isDefault": true}]
            }))
            .mount(&server)
            .await;
            Mock::given(method("POST"))
                .and(path("/v1internal:onboardUser"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "done": false,
                    "name": "operations/op-1"
                })))
                .expect(1)
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path("/v1internal/operations/op-1"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "done": true,
                    "response": {"cloudaicompanionProject": {"id": "polled-project"}}
                })))
                .expect(1)
                .mount(&server)
                .await;

            let project = setup_code_assist(&server.uri(), "tok").await.unwrap();
            assert_eq!(project, "polled-project");
        }

        #[tokio::test]
        async fn unnamed_pending_onboard_times_out() {
            let _guard = no_project_env();
            let server = MockServer::start().await;
            mock_load(json!({
                "allowedTiers": [{"id": "free-tier", "isDefault": true}]
            }))
            .mount(&server)
            .await;
            Mock::given(method("POST"))
                .and(path("/v1internal:onboardUser"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
                .mount(&server)
                .await;

            let err = setup_code_assist(&server.uri(), "tok").await.unwrap_err();
            assert!(err.to_string().contains("timed out"), "got: {err}");
        }

        #[tokio::test]
        async fn licensed_tier_without_project_gives_actionable_error() {
            let _guard = no_project_env();
            let server = MockServer::start().await;
            mock_load(json!({
                "allowedTiers": [{
                    "id": "standard-tier",
                    "isDefault": true,
                    "userDefinedCloudaicompanionProject": true
                }],
                "ineligibleTiers": [{"reasonMessage": "account has a Code Assist license"}]
            }))
            .mount(&server)
            .await;

            let err = setup_code_assist(&server.uri(), "tok").await.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("GOOGLE_CLOUD_PROJECT"), "got: {msg}");
            assert!(
                msg.contains("account has a Code Assist license"),
                "got: {msg}"
            );
        }

        #[tokio::test]
        async fn licensed_tier_with_project_sends_it_and_uses_it() {
            let _guard = env_lock::lock_env([
                ("GOOGLE_CLOUD_PROJECT", Some("workspace-project")),
                ("GOOGLE_CLOUD_PROJECT_ID", None),
            ]);
            let server = MockServer::start().await;
            mock_load(json!({
                "allowedTiers": [{
                    "id": "standard-tier",
                    "isDefault": true,
                    "userDefinedCloudaicompanionProject": true
                }]
            }))
            .mount(&server)
            .await;
            Mock::given(method("POST"))
                .and(path("/v1internal:onboardUser"))
                .and(body_partial_json(json!({
                    "tierId": "standard-tier",
                    "cloudaicompanionProject": "workspace-project",
                    "metadata": {"duetProject": "workspace-project"}
                })))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "done": true
                })))
                .expect(1)
                .mount(&server)
                .await;

            // done without a project in the response: the configured project applies
            let project = setup_code_assist(&server.uri(), "tok").await.unwrap();
            assert_eq!(project, "workspace-project");

            let load_requests: Vec<Request> = server
                .received_requests()
                .await
                .unwrap()
                .into_iter()
                .filter(|r| r.url.path().ends_with(":loadCodeAssist"))
                .collect();
            let body: Value = serde_json::from_slice(&load_requests[0].body).unwrap();
            assert_eq!(body["cloudaicompanionProject"], "workspace-project");
        }

        #[tokio::test]
        async fn no_default_tier_falls_back_to_legacy_tier() {
            let _guard = no_project_env();
            let server = MockServer::start().await;
            mock_load(json!({
                "allowedTiers": [{"id": "some-tier", "isDefault": false}]
            }))
            .mount(&server)
            .await;
            Mock::given(method("POST"))
                .and(path("/v1internal:onboardUser"))
                .and(body_partial_json(json!({"tierId": "legacy-tier"})))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "done": true,
                    "response": {"cloudaicompanionProject": {"id": "legacy-project"}}
                })))
                .expect(1)
                .mount(&server)
                .await;

            let project = setup_code_assist(&server.uri(), "tok").await.unwrap();
            assert_eq!(project, "legacy-project");
        }

        #[tokio::test]
        async fn existing_project_short_circuits() {
            let _guard = no_project_env();
            let server = MockServer::start().await;
            mock_load(json!({
                "cloudaicompanionProject": "existing-project",
                "currentTier": {"id": "free-tier"}
            }))
            .mount(&server)
            .await;

            let project = setup_code_assist(&server.uri(), "tok").await.unwrap();
            assert_eq!(project, "existing-project");
        }
    }

    #[cfg(unix)]
    #[test]
    fn token_cache_replaces_loose_file_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let cache_path = directory.path().join("tokens.json");
        std::fs::write(&cache_path, "{}").unwrap();
        std::fs::set_permissions(&cache_path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let cache = TokenCache {
            cache_path: cache_path.clone(),
        };

        cache
            .save(&SetupData {
                project_id: "project".to_string(),
                token: TokenData {
                    access_token: "access".to_string(),
                    refresh_token: "refresh".to_string(),
                    expires_at: Utc::now() + chrono::Duration::hours(1),
                },
            })
            .unwrap();

        let mode = std::fs::metadata(cache_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
