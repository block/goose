use crate::config::paths::Paths;
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::api_client::AuthProvider;
use crate::providers::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage};
use crate::providers::errors::ProviderError;
use crate::providers::formats::openai_responses::responses_api_to_streaming_message;
use crate::providers::retry::ProviderRetry;
use crate::providers::utils::handle_status_openai_compat;
use anyhow::{anyhow, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use axum::{extract::Query, response::Html, routing::get, Router};
use base64::Engine;
use chrono::{DateTime, Utc};
use futures::{StreamExt, TryStreamExt};
use once_cell::sync::Lazy;
use rmcp::model::{RawContent, Role, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Digest;
use std::io;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::pin;
use tokio::sync::{oneshot, Mutex as TokioMutex};
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const ISSUER: &str = "https://auth.openai.com";
const CODEX_API_ENDPOINT: &str = "https://chatgpt.com/backend-api/codex";
const OAUTH_SCOPES: &[&str] = &["openid", "profile", "email", "offline_access"];
const OAUTH_PORT: u16 = 1455;

pub const CHATGPT_CODEX_DEFAULT_MODEL: &str = "gpt-5.1-codex";
pub const CHATGPT_CODEX_KNOWN_MODELS: &[&str] = &[
    "gpt-5.2-codex",
    "gpt-5.1-codex",
    "gpt-5.1-codex-mini",
    "gpt-5.1-codex-max",
];

const CHATGPT_CODEX_DOC_URL: &str = "https://openai.com/chatgpt";

static OAUTH_MUTEX: Lazy<TokioMutex<()>> = Lazy::new(|| TokioMutex::new(()));

fn create_codex_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let mut input_items = Vec::new();

    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        let has_only_tool_content = message.content.iter().all(|c| {
            matches!(
                c,
                MessageContent::ToolRequest(_) | MessageContent::ToolResponse(_)
            )
        });

        if has_only_tool_content {
            continue;
        }

        if message.role != Role::User && message.role != Role::Assistant {
            continue;
        }

        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let mut content_items = Vec::new();
        for content in &message.content {
            if let MessageContent::Text(text) = content {
                if !text.text.is_empty() {
                    let content_type = if message.role == Role::Assistant {
                        "output_text"
                    } else {
                        "input_text"
                    };
                    content_items.push(json!({
                        "type": content_type,
                        "text": text.text
                    }));
                }
            }
        }

        if !content_items.is_empty() {
            input_items.push(json!({
                "role": role,
                "content": content_items
            }));
        }
    }

    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        if message.role == Role::Assistant {
            for content in &message.content {
                if let MessageContent::ToolRequest(request) = content {
                    if let Ok(tool_call) = &request.tool_call {
                        let arguments_str = tool_call
                            .arguments
                            .as_ref()
                            .map(|args| {
                                serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
                            })
                            .unwrap_or_else(|| "{}".to_string());

                        input_items.push(json!({
                            "type": "function_call",
                            "call_id": request.id,
                            "name": tool_call.name,
                            "arguments": arguments_str
                        }));
                    }
                }
            }
        }
    }

    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        for content in &message.content {
            if let MessageContent::ToolResponse(response) = content {
                match &response.tool_result {
                    Ok(contents) => {
                        let text_content: Vec<String> = contents
                            .content
                            .iter()
                            .filter_map(|c| {
                                if let RawContent::Text(t) = c.deref() {
                                    Some(t.text.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if !text_content.is_empty() {
                            input_items.push(json!({
                                "type": "function_call_output",
                                "call_id": response.id,
                                "output": text_content.join("\n")
                            }));
                        }
                    }
                    Err(error_data) => {
                        input_items.push(json!({
                            "type": "function_call_output",
                            "call_id": response.id,
                            "output": format!("Error: {}", error_data.message)
                        }));
                    }
                }
            }
        }
    }

    let mut payload = json!({
        "model": model_config.model_name,
        "input": input_items,
        "store": false,
        "instructions": system,
    });

    if !tools.is_empty() {
        let tools_spec: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                })
            })
            .collect();

        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tools_spec));
    }

    if let Some(temp) = model_config.temperature {
        payload
            .as_object_mut()
            .unwrap()
            .insert("temperature".to_string(), json!(temp));
    }

    // Note: ChatGPT Codex API does not support max_output_tokens parameter

    Ok(payload)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
    id_token: Option<String>,
    expires_at: DateTime<Utc>,
    account_id: Option<String>,
}

#[derive(Debug, Clone)]
struct TokenCache {
    cache_path: PathBuf,
}

fn get_cache_path() -> PathBuf {
    Paths::in_config_dir("chatgpt_codex/tokens.json")
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
        if let Ok(contents) = std::fs::read_to_string(&self.cache_path) {
            serde_json::from_str(&contents).ok()
        } else {
            None
        }
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

#[derive(Debug, Deserialize)]
struct JwtClaims {
    chatgpt_account_id: Option<String>,
    #[serde(rename = "https://api.openai.com/auth")]
    auth_claims: Option<AuthClaims>,
    organizations: Option<Vec<OrgInfo>>,
}

#[derive(Debug, Deserialize)]
struct AuthClaims {
    chatgpt_account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrgInfo {
    id: String,
}

fn parse_jwt_claims(token: &str) -> Option<JwtClaims> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    serde_json::from_slice(&payload).ok()
}

fn extract_account_id(token_data: &TokenData) -> Option<String> {
    if let Some(ref id_token) = token_data.id_token {
        if let Some(claims) = parse_jwt_claims(id_token) {
            if let Some(id) = claims.chatgpt_account_id {
                return Some(id);
            }
            if let Some(auth) = claims.auth_claims {
                if let Some(id) = auth.chatgpt_account_id {
                    return Some(id);
                }
            }
            if let Some(orgs) = claims.organizations {
                if let Some(org) = orgs.first() {
                    return Some(org.id.clone());
                }
            }
        }
    }
    if let Some(claims) = parse_jwt_claims(&token_data.access_token) {
        if let Some(id) = claims.chatgpt_account_id {
            return Some(id);
        }
        if let Some(auth) = claims.auth_claims {
            if let Some(id) = auth.chatgpt_account_id {
                return Some(id);
            }
        }
        if let Some(orgs) = claims.organizations {
            if let Some(org) = orgs.first() {
                return Some(org.id.clone());
            }
        }
    }
    None
}

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

fn build_authorize_url(redirect_uri: &str, pkce: &PkceChallenge, state: &str) -> String {
    let scopes = OAUTH_SCOPES.join(" ");
    let params = [
        ("response_type", "code"),
        ("client_id", CLIENT_ID),
        ("redirect_uri", redirect_uri),
        ("scope", &scopes),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", "S256"),
        ("id_token_add_organizations", "true"),
        ("codex_cli_simplified_flow", "true"),
        ("state", state),
        ("originator", "goose"),
    ];
    format!(
        "{}/oauth/authorize?{}",
        ISSUER,
        serde_urlencoded::to_string(params).unwrap()
    )
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    id_token: Option<String>,
    expires_in: Option<i64>,
}

async fn exchange_code_for_tokens(
    code: &str,
    redirect_uri: &str,
    pkce: &PkceChallenge,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", CLIENT_ID),
        ("code_verifier", &pkce.verifier),
    ];

    let resp = client
        .post(format!("{}/oauth/token", ISSUER))
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
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", CLIENT_ID),
    ];

    let resp = client
        .post(format!("{}/oauth/token", ISSUER))
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

const HTML_SUCCESS: &str = r#"<!doctype html>
<html>
  <head>
    <title>Goose - ChatGPT Authorization Successful</title>
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
      <p>You can close this window and return to Goose.</p>
    </div>
    <script>setTimeout(() => window.close(), 2000)</script>
  </body>
</html>"#;

fn html_error(error: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
  <head>
    <title>Goose - ChatGPT Authorization Failed</title>
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
        error
    )
}

async fn perform_oauth_flow() -> Result<TokenData> {
    let _guard = OAUTH_MUTEX.lock().await;

    let pkce = generate_pkce();
    let state = generate_state();
    let redirect_uri = format!("http://localhost:{}/auth/callback", OAUTH_PORT);

    let (tx, rx) = oneshot::channel::<Result<String>>();
    let tx = Arc::new(TokioMutex::new(Some(tx)));
    let expected_state = state.clone();
    let pkce_for_handler = Arc::new(pkce.verifier.clone());

    #[derive(Deserialize)]
    struct CallbackParams {
        code: Option<String>,
        state: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
    }

    let tx_clone = tx.clone();
    let app = Router::new().route(
        "/auth/callback",
        get(move |Query(params): Query<CallbackParams>| {
            let tx = tx_clone.clone();
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
                Html(HTML_SUCCESS.to_string())
            }
        }),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], OAUTH_PORT));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server_handle = tokio::spawn(async move {
        let server = axum::serve(listener, app);
        let _ = server.await;
    });

    let auth_url = build_authorize_url(&redirect_uri, &pkce, &state);
    if webbrowser::open(&auth_url).is_err() {
        println!("Please open this URL in your browser:\n{}", auth_url);
    }

    let code = tokio::time::timeout(std::time::Duration::from_secs(300), rx)
        .await
        .map_err(|_| anyhow!("OAuth flow timed out"))??
        .map_err(|e| anyhow!("OAuth callback error: {}", e))?;

    server_handle.abort();

    let pkce_challenge = PkceChallenge {
        verifier: (*pkce_for_handler).clone(),
        challenge: pkce.challenge,
    };
    let tokens = exchange_code_for_tokens(&code, &redirect_uri, &pkce_challenge).await?;

    let expires_at = Utc::now() + chrono::Duration::seconds(tokens.expires_in.unwrap_or(3600));

    let mut token_data = TokenData {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        id_token: tokens.id_token,
        expires_at,
        account_id: None,
    };

    token_data.account_id = extract_account_id(&token_data);

    Ok(token_data)
}

#[derive(Debug)]
struct ChatGptCodexAuthProvider {
    cache: TokenCache,
}

impl ChatGptCodexAuthProvider {
    fn new() -> Self {
        Self {
            cache: TokenCache::new(),
        }
    }

    async fn get_valid_token(&self) -> Result<TokenData> {
        if let Some(mut token_data) = self.cache.load() {
            if token_data.expires_at > Utc::now() + chrono::Duration::seconds(60) {
                return Ok(token_data);
            }

            tracing::debug!("Token expired, attempting refresh");
            match refresh_access_token(&token_data.refresh_token).await {
                Ok(new_tokens) => {
                    token_data.access_token = new_tokens.access_token;
                    token_data.refresh_token = new_tokens.refresh_token;
                    if new_tokens.id_token.is_some() {
                        token_data.id_token = new_tokens.id_token;
                    }
                    token_data.expires_at = Utc::now()
                        + chrono::Duration::seconds(new_tokens.expires_in.unwrap_or(3600));
                    if token_data.account_id.is_none() {
                        token_data.account_id = extract_account_id(&token_data);
                    }
                    self.cache.save(&token_data)?;
                    tracing::info!("Token refreshed successfully");
                    return Ok(token_data);
                }
                Err(e) => {
                    tracing::warn!("Token refresh failed, will re-authenticate: {}", e);
                    self.cache.clear();
                }
            }
        }

        tracing::info!("Starting OAuth flow for ChatGPT Codex");
        let token_data = perform_oauth_flow().await?;
        self.cache.save(&token_data)?;
        Ok(token_data)
    }
}

#[async_trait]
impl AuthProvider for ChatGptCodexAuthProvider {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        let token_data = self.get_valid_token().await?;
        Ok((
            "Authorization".to_string(),
            format!("Bearer {}", token_data.access_token),
        ))
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ChatGptCodexProvider {
    #[serde(skip)]
    auth_provider: Arc<ChatGptCodexAuthProvider>,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl ChatGptCodexProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let auth_provider = Arc::new(ChatGptCodexAuthProvider::new());

        Ok(Self {
            auth_provider,
            model,
            name: Self::metadata().name,
        })
    }

    async fn post_streaming(&self, payload: &Value) -> Result<reqwest::Response, ProviderError> {
        let token_data = self
            .auth_provider
            .get_valid_token()
            .await
            .map_err(|e| ProviderError::Authentication(e.to_string()))?;

        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(account_id) = &token_data.account_id {
            headers.insert(
                reqwest::header::HeaderName::from_static("chatgpt-account-id"),
                reqwest::header::HeaderValue::from_str(account_id)
                    .map_err(|e| ProviderError::ExecutionError(e.to_string()))?,
            );
        }

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/responses", CODEX_API_ENDPOINT))
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

#[async_trait]
impl Provider for ChatGptCodexProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "chatgpt_codex",
            "ChatGPT Codex",
            "Use your ChatGPT Plus/Pro subscription for GPT-5 Codex models via OAuth",
            CHATGPT_CODEX_DEFAULT_MODEL,
            CHATGPT_CODEX_KNOWN_MODELS.to_vec(),
            CHATGPT_CODEX_DOC_URL,
            vec![ConfigKey::new_oauth(
                "CHATGPT_CODEX_TOKEN",
                true,
                true,
                None,
            )],
        )
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // ChatGPT Codex API requires streaming - collect the stream into a single response
        let mut payload = create_codex_request(model_config, system, messages, tools)
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
        payload["stream"] = serde_json::Value::Bool(true);

        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post_streaming(&payload_clone).await
            })
            .await?;

        let stream = response.bytes_stream().map_err(io::Error::other);
        let stream_reader = StreamReader::new(stream);
        let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);

        let message_stream = responses_api_to_streaming_message(framed);
        pin!(message_stream);

        let mut final_message: Option<Message> = None;
        let mut final_usage: Option<ProviderUsage> = None;

        while let Some(result) = message_stream.next().await {
            let (message, usage) = result
                .map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
            if let Some(msg) = message {
                final_message = Some(msg);
            }
            if let Some(u) = usage {
                final_usage = Some(u);
            }
        }

        let message = final_message.ok_or_else(|| {
            ProviderError::ExecutionError("No message received from stream".to_string())
        })?;
        let usage = final_usage.unwrap_or_else(|| {
            ProviderUsage::new(
                model_config.model_name.clone(),
                crate::providers::base::Usage::default(),
            )
        });

        Ok((message, usage))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_codex_request(&self.model, system, messages, tools)
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
        payload["stream"] = serde_json::Value::Bool(true);

        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post_streaming(&payload_clone).await
            })
            .await?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = responses_api_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                yield (message, usage);
            }
        }))
    }

    async fn configure_oauth(&self) -> Result<(), ProviderError> {
        self.auth_provider
            .get_valid_token()
            .await
            .map_err(|e| ProviderError::Authentication(format!("OAuth flow failed: {}", e)))?;
        Ok(())
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        Ok(Some(
            CHATGPT_CODEX_KNOWN_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ))
    }
}
