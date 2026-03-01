use anyhow::{anyhow, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use axum::{response::Html, routing::get, Router};
use base64::Engine;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tokio::pin;
use tokio::sync::{oneshot, Mutex as TokioMutex};
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::errors::ProviderError;
use super::formats::anthropic::{
    create_request, response_to_streaming_message, thinking_type, ThinkingType,
};
use super::openai_compatible::handle_status_openai_compat;
use super::retry::ProviderRetry;
use super::utils::RequestLog;
use crate::config::paths::Paths;
use crate::config::ExtensionConfig;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PROVIDER_NAME: &str = "claude_sub";
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-5";
const DEFAULT_FAST_MODEL: &str = "claude-haiku-4-5";

const KNOWN_MODELS: &[&str] = &[
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-sonnet-4-5",
    "claude-sonnet-4-5-20250929",
    "claude-haiku-4-5",
    "claude-haiku-4-5-20251001",
    "claude-opus-4-5",
    "claude-opus-4-5-20251101",
    "claude-sonnet-4-0",
    "claude-sonnet-4-20250514",
    "claude-opus-4-0",
    "claude-opus-4-20250514",
];

const DOC_URL: &str = "https://docs.anthropic.com/en/docs/about-claude/models";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const ANTHROPIC_API_HOST: &str = "https://api.anthropic.com";

// OAuth constants (from Claude Code / pi implementation)
const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const OAUTH_AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
const OAUTH_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
const OAUTH_REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";
const OAUTH_SCOPES: &str = "org:create_api_key user:profile user:inference";

// Claude Code stealth identity
const CLAUDE_CODE_VERSION: &str = "2.1.62";
const CLAUDE_CODE_SYSTEM_PREFIX: &str =
    "You are Claude Code, Anthropic's official CLI for Claude.";

// Claude Code canonical tool names for remapping
const CLAUDE_CODE_TOOLS: &[&str] = &[
    "Read",
    "Write",
    "Edit",
    "Bash",
    "Grep",
    "Glob",
    "AskUserQuestion",
    "EnterPlanMode",
    "ExitPlanMode",
    "KillShell",
    "NotebookEdit",
    "Skill",
    "Task",
    "TaskOutput",
    "TodoWrite",
    "WebFetch",
    "WebSearch",
];

// OAuth callback port
const OAUTH_PORT: u16 = 0; // OS-assigned
const OAUTH_TIMEOUT_SECS: u64 = 300;

// ---------------------------------------------------------------------------
// Auth state (singleton)
// ---------------------------------------------------------------------------

static AUTH_STATE: LazyLock<Arc<TokioMutex<()>>> = LazyLock::new(|| Arc::new(TokioMutex::new(())));

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct TokenCache {
    cache_path: PathBuf,
}

fn get_cache_path() -> PathBuf {
    Paths::in_config_dir("claude_sub/tokens.json")
}

impl TokenCache {
    fn new() -> Self {
        let cache_path = get_cache_path();
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self { cache_path }
    }

    fn load(&self) -> Option<TokenData> {
        let contents = std::fs::read_to_string(&self.cache_path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    fn save(&self, token_data: &TokenData) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string(token_data)?;
        std::fs::write(&self.cache_path, contents)?;
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

// ---------------------------------------------------------------------------
// OAuth flow
// ---------------------------------------------------------------------------

fn build_authorize_url(pkce: &PkceChallenge, state: &str) -> String {
    let params = [
        ("code", "true"),
        ("client_id", OAUTH_CLIENT_ID),
        ("response_type", "code"),
        ("redirect_uri", OAUTH_REDIRECT_URI),
        ("scope", OAUTH_SCOPES),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
    ];
    let query = serde_urlencoded::to_string(params).expect("valid params");
    format!("{}?{}", OAUTH_AUTHORIZE_URL, query)
}

/// The Anthropic OAuth flow uses a fixed redirect URI that points to
/// console.anthropic.com.  The browser visits that page after the user
/// authorises, and Anthropic's page displays `code#state` for the user
/// to copy-paste.  We therefore start a tiny local web page that simply
/// asks the user to paste the code.  An alternative design would be to
/// embed the code into a localhost redirect (like Codex/Copilot), but
/// that would require Anthropic to register our localhost callback.
///
/// This matches how pi does it: open the auth URL, then prompt for the
/// code string that contains `code#state`.
async fn perform_oauth_flow() -> Result<TokenData> {
    let _guard = AUTH_STATE
        .try_lock()
        .map_err(|_| anyhow!("Another OAuth flow is already in progress"))?;

    let pkce = generate_pkce();
    let state = nanoid::nanoid!(32);
    let auth_url = build_authorize_url(&pkce, &state);

    // --- Local server to collect the code#state paste ---
    let (tx, rx) = oneshot::channel::<Result<String>>();
    let tx = Arc::new(TokioMutex::new(Some(tx)));
    let tx_clone = Arc::clone(&tx);

    let app = Router::new()
        .route(
            "/",
            get(|| async {
                Html(
                    r#"<!doctype html>
<html><head><title>goose - Claude Authorization</title>
<style>
body{font-family:system-ui;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;background:#131010;color:#f1ecec}
.c{text-align:center;padding:2rem;max-width:500px}
input{width:100%;padding:0.5rem;margin:0.5rem 0;font-size:1rem;border-radius:4px;border:1px solid #444;background:#1a1a1a;color:#f1ecec}
button{padding:0.5rem 1.5rem;font-size:1rem;border-radius:4px;border:none;background:#7c3aed;color:white;cursor:pointer}
button:hover{background:#6d28d9}
p{color:#b7b1b1}
</style></head><body>
<div class="c">
<h2>Paste the authorization code</h2>
<p>After authorizing in the browser, Anthropic will show a code. Paste it here:</p>
<form method="POST" action="/submit">
<input name="code" placeholder="code#state" autofocus/>
<br/><button type="submit">Submit</button>
</form>
</div></body></html>"#,
                )
            }),
        )
        .route(
            "/submit",
            axum::routing::post(
                move |form: axum::extract::Form<HashMap<String, String>>| {
                    let tx = Arc::clone(&tx_clone);
                    async move {
                        let code_str = form
                            .get("code")
                            .cloned()
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        if code_str.is_empty() {
                            return Html("<h2>Error</h2><p>No code provided</p>".to_string());
                        }
                        if let Some(sender) = tx.lock().await.take() {
                            let _ = sender.send(Ok(code_str));
                        }
                        Html(
                            "<h2>Success</h2><p>You can close this window and return to goose.</p>"
                                .to_string(),
                        )
                    }
                },
            ),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], OAUTH_PORT));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_port = listener.local_addr()?.port();
    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    // Open browser to auth URL
    if webbrowser::open(&auth_url).is_err() {
        tracing::info!("Please open this URL in your browser:\n{}", auth_url);
    }

    // Also print instructions to the local paste page
    tracing::info!(
        "After authorizing, paste the code at http://localhost:{}",
        actual_port
    );

    // Wait for the code
    let code_result = tokio::time::timeout(
        std::time::Duration::from_secs(OAUTH_TIMEOUT_SECS),
        rx,
    )
    .await
    .map_err(|_| anyhow!("OAuth flow timed out"))??;

    server_handle.abort();
    let code_str = code_result?;

    // Parse code#state
    let (code, returned_state) = code_str
        .split_once('#')
        .ok_or_else(|| anyhow!("Invalid code format, expected code#state"))?;

    // Exchange code for tokens
    let client = reqwest::Client::new();
    let resp = client
        .post(OAUTH_TOKEN_URL)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": OAUTH_CLIENT_ID,
            "code": code,
            "state": returned_state,
            "redirect_uri": OAUTH_REDIRECT_URI,
            "code_verifier": pkce.verifier,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Token exchange failed ({}): {}", status, text));
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: String,
        expires_in: u64,
    }

    let tokens: TokenResponse = resp.json().await?;
    // 5-minute buffer before expiry
    let expires_at = Utc::now() + chrono::Duration::seconds(tokens.expires_in as i64 - 300);

    Ok(TokenData {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at,
    })
}

async fn refresh_token(refresh_token: &str) -> Result<TokenData> {
    let client = reqwest::Client::new();
    let resp = client
        .post(OAUTH_TOKEN_URL)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": OAUTH_CLIENT_ID,
            "refresh_token": refresh_token,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Token refresh failed ({}): {}", status, text));
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: String,
        expires_in: u64,
    }

    let tokens: TokenResponse = resp.json().await?;
    let expires_at = Utc::now() + chrono::Duration::seconds(tokens.expires_in as i64 - 300);

    Ok(TokenData {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at,
    })
}

async fn get_valid_token(cache: &TokenCache) -> Result<TokenData> {
    if let Some(token_data) = cache.load() {
        // Still valid? Use it.
        if token_data.expires_at > Utc::now() + chrono::Duration::seconds(60) {
            return Ok(token_data);
        }

        // Try refresh
        tracing::debug!("Claude subscription token expired, attempting refresh");
        match refresh_token(&token_data.refresh_token).await {
            Ok(new_token) => {
                cache.save(&new_token)?;
                tracing::info!("Claude subscription token refreshed successfully");
                return Ok(new_token);
            }
            Err(e) => {
                tracing::warn!(
                    "Token refresh failed, will re-authenticate: {}",
                    e
                );
                cache.clear();
            }
        }
    }

    tracing::info!("Starting OAuth flow for Claude subscription");
    let token_data = perform_oauth_flow().await?;
    cache.save(&token_data)?;
    Ok(token_data)
}

// ---------------------------------------------------------------------------
// Tool name remapping
// ---------------------------------------------------------------------------

fn to_claude_code_name(name: &str) -> String {
    let lower = name.to_lowercase();
    for &cc_name in CLAUDE_CODE_TOOLS {
        if cc_name.to_lowercase() == lower {
            return cc_name.to_string();
        }
    }
    name.to_string()
}

fn from_claude_code_name<'a>(name: &str, tools: &'a [Tool]) -> String {
    let lower = name.to_lowercase();
    for tool in tools {
        if tool.name.to_lowercase() == lower {
            return tool.name.to_string();
        }
    }
    name.to_string()
}

// ---------------------------------------------------------------------------
// System prompt injection
// ---------------------------------------------------------------------------

fn build_stealth_system(system: &str) -> String {
    if system.is_empty() {
        CLAUDE_CODE_SYSTEM_PREFIX.to_string()
    } else {
        format!("{}\n\n{}", CLAUDE_CODE_SYSTEM_PREFIX, system)
    }
}

// ---------------------------------------------------------------------------
// Request / response modifications for stealth mode
// ---------------------------------------------------------------------------

/// Remap tool names in the request payload from the caller's names to Claude
/// Code canonical names.
fn remap_tools_in_payload(payload: &mut Value) {
    if let Some(tools) = payload.get_mut("tools").and_then(|t| t.as_array_mut()) {
        for tool in tools.iter_mut() {
            if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                let cc_name = to_claude_code_name(name);
                tool.as_object_mut()
                    .unwrap()
                    .insert("name".to_string(), serde_json::json!(cc_name));
            }
        }
    }
}

/// Remap tool names in a streaming response message back from Claude Code
/// canonical names to the caller's original names.
fn remap_tool_names_in_message(message: &mut Option<Message>, tools: &[Tool]) {
    if let Some(msg) = message.as_mut() {
        for content in &mut msg.content {
            if let crate::conversation::message::MessageContent::ToolRequest(ref mut req) = content
            {
                if let Ok(ref mut call) = req.tool_call {
                    let original = from_claude_code_name(&call.name, tools);
                    call.name = original.into();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct ClaudeSubProvider {
    model: ModelConfig,
    #[serde(skip)]
    token_cache: TokenCache,
    #[serde(skip)]
    name: String,
}

impl ClaudeSubProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let model = model.with_fast(DEFAULT_FAST_MODEL, PROVIDER_NAME)?;
        Ok(Self {
            model,
            token_cache: TokenCache::new(),
            name: PROVIDER_NAME.to_string(),
        })
    }

    fn get_conditional_headers(&self) -> Vec<(&str, &str)> {
        let mut headers = Vec::new();
        if self.model.model_name.starts_with("claude-3-7-sonnet-") {
            if thinking_type(&self.model) == ThinkingType::Enabled {
                headers.push(("anthropic-beta", "output-128k-2025-02-19"));
            }
            headers.push(("anthropic-beta", "token-efficient-tools-2025-02-19"));
        }
        headers
    }

    async fn post_streaming(
        &self,
        session_id: Option<&str>,
        payload: &Value,
    ) -> Result<reqwest::Response, ProviderError> {
        let token_data = get_valid_token(&self.token_cache)
            .await
            .map_err(|e| ProviderError::Authentication(e.to_string()))?;

        let mut headers = reqwest::header::HeaderMap::new();
        // Anthropic API version
        headers.insert(
            reqwest::header::HeaderName::from_static("anthropic-version"),
            reqwest::header::HeaderValue::from_static(ANTHROPIC_API_VERSION),
        );
        // Claude Code stealth headers
        headers.insert(
            reqwest::header::HeaderName::from_static("anthropic-beta"),
            reqwest::header::HeaderValue::from_static(
                "claude-code-20250219,oauth-2025-04-20,fine-grained-tool-streaming-2025-05-14",
            ),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("user-agent"),
            reqwest::header::HeaderValue::from_str(&format!("claude-cli/{}", CLAUDE_CODE_VERSION))
                .unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("x-app"),
            reqwest::header::HeaderValue::from_static("cli"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        // Session ID
        if let Some(sid) = session_id.filter(|id| !id.is_empty()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(sid) {
                headers.insert(
                    reqwest::header::HeaderName::from_static(
                        crate::session_context::SESSION_ID_HEADER,
                    ),
                    val,
                );
            }
        }

        // Conditional headers for specific models
        for (key, value) in self.get_conditional_headers() {
            // Append to anthropic-beta if it's the same header
            if key == "anthropic-beta" {
                if let Some(existing) = headers.get("anthropic-beta") {
                    let existing_str = existing.to_str().unwrap_or("");
                    let combined = format!("{},{}", existing_str, value);
                    headers.insert(
                        reqwest::header::HeaderName::from_static("anthropic-beta"),
                        reqwest::header::HeaderValue::from_str(&combined).unwrap(),
                    );
                }
            } else if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/v1/messages", ANTHROPIC_API_HOST))
            .header(
                "Authorization",
                format!("Bearer {}", token_data.access_token),
            )
            .header("Content-Type", "application/json")
            .headers(headers)
            .json(payload)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        handle_status_openai_compat(response).await
    }
}

impl ProviderDef for ClaudeSubProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            PROVIDER_NAME,
            "Claude Subscription",
            "Use your Claude Pro/Max subscription via OAuth (no API key required)",
            DEFAULT_MODEL,
            KNOWN_MODELS.to_vec(),
            DOC_URL,
            vec![ConfigKey::new_oauth(
                "CLAUDE_SUB_TOKEN",
                true,
                true,
                None,
                false,
            )],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for ClaudeSubProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        // Build standard Anthropic request payload
        let stealth_system = build_stealth_system(system);
        let mut payload = create_request(model_config, &stealth_system, messages, tools)
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;

        // Enable streaming
        payload
            .as_object_mut()
            .unwrap()
            .insert("stream".to_string(), serde_json::json!(true));

        // Remap tool names to Claude Code canonical casing
        remap_tools_in_payload(&mut payload);

        let mut log = RequestLog::start(model_config, &payload)
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;

        // Clone tools for use in the stream closure
        let tools_owned: Vec<Tool> = tools.to_vec();

        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post_streaming(Some(session_id), &payload_clone).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new())
                .map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = futures::StreamExt::next(&mut message_stream).await {
                let (mut message, usage) = message.map_err(|e| {
                    ProviderError::RequestFailed(format!("Stream decode error: {}", e))
                })?;
                // Remap tool names back from Claude Code canonical casing
                remap_tool_names_in_message(&mut message, &tools_owned);
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())
                    .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
                yield (message, usage);
            }
        }))
    }

    async fn configure_oauth(&self) -> Result<(), ProviderError> {
        get_valid_token(&self.token_cache)
            .await
            .map_err(|e| ProviderError::Authentication(format!("OAuth flow failed: {}", e)))?;
        Ok(())
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(KNOWN_MODELS.iter().map(|s| s.to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Tool;
    use rmcp::object;
    use serde_json::json;

    #[test]
    fn test_to_claude_code_name() {
        assert_eq!(to_claude_code_name("read"), "Read");
        assert_eq!(to_claude_code_name("READ"), "Read");
        assert_eq!(to_claude_code_name("bash"), "Bash");
        assert_eq!(to_claude_code_name("Bash"), "Bash");
        assert_eq!(to_claude_code_name("my_custom_tool"), "my_custom_tool");
    }

    #[test]
    fn test_from_claude_code_name() {
        let tools = vec![
            Tool::new("read", "Read files", object!({"type": "object"})),
            Tool::new("bash", "Run commands", object!({"type": "object"})),
        ];
        assert_eq!(from_claude_code_name("Read", &tools), "read");
        assert_eq!(from_claude_code_name("Bash", &tools), "bash");
        assert_eq!(from_claude_code_name("Unknown", &tools), "Unknown");
    }

    #[test]
    fn test_build_stealth_system_empty() {
        let result = build_stealth_system("");
        assert_eq!(result, CLAUDE_CODE_SYSTEM_PREFIX);
    }

    #[test]
    fn test_build_stealth_system_with_content() {
        let result = build_stealth_system("You are a helpful assistant.");
        assert!(result.starts_with(CLAUDE_CODE_SYSTEM_PREFIX));
        assert!(result.contains("You are a helpful assistant."));
    }

    #[test]
    fn test_remap_tools_in_payload() {
        let mut payload = json!({
            "tools": [
                {"name": "read", "description": "Read files", "input_schema": {}},
                {"name": "bash", "description": "Run commands", "input_schema": {}},
                {"name": "my_custom_tool", "description": "Custom", "input_schema": {}},
            ]
        });
        remap_tools_in_payload(&mut payload);

        let tools = payload["tools"].as_array().unwrap();
        assert_eq!(tools[0]["name"], "Read");
        assert_eq!(tools[1]["name"], "Bash");
        assert_eq!(tools[2]["name"], "my_custom_tool");
    }

    #[test]
    fn test_build_authorize_url() {
        let pkce = PkceChallenge {
            verifier: "test-verifier".to_string(),
            challenge: "test-challenge".to_string(),
        };
        let url = build_authorize_url(&pkce, "test-state");
        assert!(url.starts_with(OAUTH_AUTHORIZE_URL));
        assert!(url.contains("client_id="));
        assert!(url.contains("code_challenge=test-challenge"));
        assert!(url.contains("state=test-state"));
    }

    #[test]
    fn test_token_cache_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = TokenCache {
            cache_path: dir.path().join("tokens.json"),
        };

        assert!(cache.load().is_none());

        let token_data = TokenData {
            access_token: "sk-ant-oat-test-access".to_string(),
            refresh_token: "test-refresh".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        cache.save(&token_data).unwrap();
        let loaded = cache.load().unwrap();
        assert_eq!(loaded.access_token, "sk-ant-oat-test-access");
        assert_eq!(loaded.refresh_token, "test-refresh");

        cache.clear();
        assert!(cache.load().is_none());
    }
}
