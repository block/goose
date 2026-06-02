use crate::config::paths::Paths;
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::api_client::AuthProvider;
use crate::providers::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use crate::providers::errors::ProviderError;
use crate::providers::formats::openai::responses_api_to_streaming_message;
use crate::providers::openai_compatible::handle_status;
use crate::providers::retry::ProviderRetry;
use crate::session_context::SESSION_ID_HEADER;
use anyhow::{anyhow, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use axum::{extract::Query, response::Html, routing::get, Router};
use base64::Engine;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::{StreamExt, TryStreamExt};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use reqwest::header::{HeaderName, HeaderValue};
use rmcp::model::{RawContent, Role, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Digest;
use std::io;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tokio::pin;
use tokio::sync::{oneshot, Mutex as TokioMutex};
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

mod auth;
mod request;

use auth::{perform_oauth_flow, ChatGptCodexAuthProvider, TokenCache};
use request::create_codex_request;

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const ISSUER: &str = "https://auth.openai.com";
const CODEX_API_ENDPOINT: &str = "https://chatgpt.com/backend-api/codex";
const OAUTH_SCOPES: &[&str] = &["openid", "profile", "email", "offline_access"];
// Canonical localhost callback port for Codex OAuth (default localhost:1455 per OpenAI docs).
// https://developers.openai.com/codex/auth/
const OAUTH_PORT: u16 = 1455;
// Allow time for users to complete the browser-based OAuth flow.
const OAUTH_TIMEOUT_SECS: u64 = 300;
const HTML_AUTO_CLOSE_TIMEOUT_MS: u64 = 2000;

const CHATGPT_CODEX_PROVIDER_NAME: &str = "chatgpt_codex";
pub const CHATGPT_CODEX_DEFAULT_MODEL: &str = "gpt-5.5";

#[derive(Debug)]
pub struct ChatGptCodexModelAttrs {
    pub name: &'static str,
    pub reasoning_levels: &'static [&'static str],
}

pub const CHATGPT_CODEX_KNOWN_MODELS: &[ChatGptCodexModelAttrs] = &[
    ChatGptCodexModelAttrs {
        name: "gpt-5.5",
        reasoning_levels: &["low", "medium", "high", "xhigh"],
    },
    ChatGptCodexModelAttrs {
        name: "gpt-5.4",
        reasoning_levels: &["low", "medium", "high", "xhigh"],
    },
    ChatGptCodexModelAttrs {
        name: "gpt-5.3-codex",
        reasoning_levels: &["low", "medium", "high", "xhigh"],
    },
];

const CHATGPT_CODEX_DOC_URL: &str = "https://openai.com/chatgpt";

const DEFAULT_REASONING_LEVELS: &[&str] = &["medium", "high"];

pub fn reasoning_levels_for_model(model_name: &str) -> &'static [&'static str] {
    CHATGPT_CODEX_KNOWN_MODELS
        .iter()
        .find(|m| m.name == model_name)
        .map(|m| m.reasoning_levels)
        .unwrap_or(DEFAULT_REASONING_LEVELS)
}

fn known_model_names() -> Vec<&'static str> {
    CHATGPT_CODEX_KNOWN_MODELS.iter().map(|m| m.name).collect()
}

const GPT_53_CODEX_TOOL_PREAMBLE: &str = "\
You are a coding agent. You have access to tools to accomplish tasks. \
Always use your tools to fulfill requests - do not just describe what you would do. \
Keep going until the query is completely resolved before yielding back to the user. \
Autonomously resolve the query using the tools available to you. \
Do NOT guess or make up an answer. \
Before making tool calls, send a brief message explaining what you're about to do.";

#[derive(Debug)]
pub(super) struct ChatGptCodexAuthState {
    pub oauth_mutex: TokioMutex<()>,
    pub jwks_cache: TokioMutex<Option<JwkSet>>,
}

impl ChatGptCodexAuthState {
    fn new() -> Self {
        Self {
            oauth_mutex: TokioMutex::new(()),
            jwks_cache: TokioMutex::new(None),
        }
    }

    fn instance() -> Arc<Self> {
        Arc::clone(&CHATGPT_CODEX_AUTH_STATE)
    }
}

static CHATGPT_CODEX_AUTH_STATE: LazyLock<Arc<ChatGptCodexAuthState>> =
    LazyLock::new(|| Arc::new(ChatGptCodexAuthState::new()));

fn get_reasoning_effort(model_name: &str) -> String {
    let config = crate::config::Config::global();
    let effort = config
        .get_chatgpt_codex_reasoning_effort()
        .map(String::from)
        .unwrap_or_else(|_| "medium".to_string());

    let valid_levels = reasoning_levels_for_model(model_name);
    if valid_levels.contains(&effort.as_str()) {
        effort
    } else {
        tracing::warn!(
            "Invalid CHATGPT_CODEX_REASONING_EFFORT '{}' for model '{}', using 'medium'",
            effort,
            model_name
        );
        "medium".to_string()
    }
}

fn reasoning_effort_for_config(model_config: &ModelConfig) -> Option<String> {
    use crate::model::ThinkingEffort;

    model_config
        .thinking_effort()
        .map(|effort| {
            let valid_levels = reasoning_levels_for_model(&model_config.model_name);
            let preferred_levels: &[&str] = match effort {
                ThinkingEffort::Off => return None,
                ThinkingEffort::Low => &["low", "medium", "high", "xhigh"],
                ThinkingEffort::Medium => &["medium", "high", "low", "xhigh"],
                ThinkingEffort::High => &["high", "medium", "xhigh", "low"],
                ThinkingEffort::Max => &["xhigh", "high", "medium", "low"],
            };

            preferred_levels
                .iter()
                .find(|level| valid_levels.contains(level))
                .map(|level| (*level).to_string())
        })
        .unwrap_or_else(|| Some(get_reasoning_effort(&model_config.model_name)))
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
    pub async fn cleanup() -> Result<()> {
        TokenCache::new().clear();
        Ok(())
    }

    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let auth_provider = Arc::new(ChatGptCodexAuthProvider::new(
            ChatGptCodexAuthState::instance(),
        ));

        Ok(Self {
            auth_provider,
            model,
            name: CHATGPT_CODEX_PROVIDER_NAME.to_string(),
        })
    }

    async fn post_streaming(
        &self,
        session_id: Option<&str>,
        payload: &Value,
    ) -> Result<reqwest::Response, ProviderError> {
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

        if let Some(session_id) = session_id.filter(|id| !id.is_empty()) {
            headers.insert(
                HeaderName::from_static(SESSION_ID_HEADER),
                HeaderValue::from_str(session_id)
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

        handle_status(response).await
    }
}

impl ProviderDef for ChatGptCodexProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            CHATGPT_CODEX_PROVIDER_NAME,
            "ChatGPT Codex",
            "Use your ChatGPT Plus/Pro subscription for GPT-5 Codex models via OAuth",
            CHATGPT_CODEX_DEFAULT_MODEL,
            known_model_names(),
            CHATGPT_CODEX_DOC_URL,
            vec![ConfigKey::new_oauth(
                "CHATGPT_CODEX_TOKEN",
                true,
                true,
                None,
                false,
            )],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }

    fn inventory_configured() -> bool {
        TokenCache::new().load().is_some()
    }
}

#[async_trait]
impl Provider for ChatGptCodexProvider {
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
        let mut payload = create_codex_request(model_config, system, messages, tools)
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))?;
        payload["stream"] = serde_json::Value::Bool(true);

        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post_streaming(Some(session_id), &payload_clone).await
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
        let previous_token = self.auth_provider.cache.load();
        self.auth_provider.clear_cached_tokens();

        let result = perform_oauth_flow(self.auth_provider.state.as_ref())
            .await
            .and_then(|token_data| self.auth_provider.cache.save(&token_data));

        if let Err(e) = result {
            if let Some(previous_token) = previous_token.as_ref() {
                if self.auth_provider.cache.load().is_none() {
                    let _ = self.auth_provider.cache.save(previous_token);
                }
            }
            return Err(ProviderError::Authentication(format!(
                "OAuth flow failed: {}",
                e
            )));
        }

        Ok(())
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(known_model_names().into_iter().map(String::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use goose_test_support::TEST_IMAGE_B64;
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use rmcp::model::{CallToolRequestParams, CallToolResult, Content, ErrorCode, ErrorData};
    use rmcp::object;
    use test_case::test_case;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use auth::{
        account_id_from_claims, fetch_jwks_for, parse_jwt_claims_with_jwks, AuthClaims, JwtClaims,
        OrgInfo, PkceChallenge, TokenData,
    };
    use request::build_input_items;

    fn input_kinds(payload: &Value) -> Vec<String> {
        payload["input"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        if let Some(role) = item.get("role").and_then(|r| r.as_str()) {
                            format!("message:{role}")
                        } else {
                            item.get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("unknown")
                                .to_string()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    #[serial_test::serial]
    fn inventory_configured_uses_oauth_token_cache() {
        let root = tempfile::tempdir().unwrap();
        let root_path = root.path().to_string_lossy().to_string();
        let _guard = env_lock::lock_env([("GOOSE_PATH_ROOT", Some(root_path.as_str()))]);

        TokenCache::new().clear();
        assert!(!ChatGptCodexProvider::inventory_configured());

        TokenCache::new()
            .save(&TokenData {
                access_token: "access".to_string(),
                refresh_token: "refresh".to_string(),
                id_token: None,
                expires_at: Utc::now() + chrono::Duration::hours(1),
                account_id: Some("account".to_string()),
            })
            .unwrap();

        assert!(ChatGptCodexProvider::inventory_configured());
    }

    #[test_case(
        vec![
            Message::user().with_text("user text"),
            Message::assistant().with_text("assistant prelude").with_tool_request(
                "call-1",
                Ok(CallToolRequestParams::new("tool_name").with_arguments(object!({"param": "value"}))),
            ),
            Message::user().with_tool_response(
                "call-1",
                Ok(CallToolResult::success(vec![Content::text("tool output")])),
            ),
            Message::assistant().with_text("assistant follow-up"),
        ],
        vec![
            "message:user".to_string(),
            "message:assistant".to_string(),
            "function_call".to_string(),
            "function_call_output".to_string(),
            "message:assistant".to_string(),
        ];
        "preserves order when assistant includes text"
    )]
    #[test_case(
        vec![
            Message::user().with_text("user text"),
            Message::assistant().with_tool_request(
                "call-1",
                Ok(CallToolRequestParams::new("tool_name").with_arguments(object!({"param": "value"}))),
            ),
            Message::user().with_tool_response(
                "call-1",
                Ok(CallToolResult::success(vec![Content::text("tool output")])),
            ),
            Message::assistant().with_text("assistant follow-up"),
        ],
        vec![
            "message:user".to_string(),
            "function_call".to_string(),
            "function_call_output".to_string(),
            "message:assistant".to_string(),
        ];
        "skips empty assistant message and preserves tool order"
    )]
    #[test_case(
        vec![
            Message::user().with_text("user text"),
            Message::assistant().with_tool_request(
                "call-1",
                Ok(CallToolRequestParams::new("tool_name").with_arguments(object!({"param": "value"}))),
            ),
            Message::user().with_tool_response(
                "call-1",
                Err(ErrorData::new(ErrorCode::INTERNAL_ERROR, "boom", None)),
            ),
        ],
        vec![
            "message:user".to_string(),
            "function_call".to_string(),
            "function_call_output".to_string(),
        ];
        "includes tool error output"
    )]
    #[test_case(
        vec![
            Message::user()
                .with_text("describe this")
                .with_image(TEST_IMAGE_B64, "image/png"),
        ],
        vec![
            "message:user".to_string(),
        ];
        "image content included in user message"
    )]
    fn test_codex_input_order(messages: Vec<Message>, expected: Vec<String>) {
        let items = build_input_items(&messages).unwrap();
        let payload = json!({ "input": items });
        let kinds = input_kinds(&payload);
        assert_eq!(kinds, expected);
    }

    #[test]
    fn test_image_url_format() {
        let messages = vec![Message::user().with_image(TEST_IMAGE_B64, "image/png")];
        let items = build_input_items(&messages).unwrap();
        // The image is inside the content array of the user message
        let content = items[0]["content"].as_array().unwrap();
        let image_item = &content[0];
        assert_eq!(image_item["type"], "input_image");
        let url = image_item["image_url"].as_str().unwrap();
        assert!(
            url.starts_with("data:image/png;base64,"),
            "image_url should start with data:image/png;base64, but was: {}",
            url
        );
    }

    #[test]
    fn test_create_codex_request_reasoning_effort_from_unified_thinking() {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!("max"));
        let mut config = ModelConfig::new("gpt-5.3-codex").unwrap();
        config.request_params = Some(params);

        let payload = create_codex_request(&config, "sys", &[], &[]).unwrap();
        assert_eq!(payload["reasoning"]["effort"], "xhigh");
        assert!(payload.get("reasoning_effort").is_none());
    }

    #[test]
    fn test_create_codex_request_caps_unified_thinking_to_supported_level() {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!("max"));
        let mut config = ModelConfig::new("unknown-model").unwrap();
        config.request_params = Some(params);

        let payload = create_codex_request(&config, "sys", &[], &[]).unwrap();
        assert_eq!(payload["reasoning"]["effort"], "high");
        assert!(payload.get("reasoning_effort").is_none());
    }

    #[test]
    fn test_create_codex_request_off_omits_reasoning_for_codex_models() {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!("off"));
        let mut config = ModelConfig::new("gpt-5.2-codex").unwrap();
        config.request_params = Some(params);

        let payload = create_codex_request(&config, "sys", &[], &[]).unwrap();
        assert!(payload.get("reasoning").is_none());
        assert!(payload.get("reasoning_effort").is_none());
    }

    #[test_case(
        JwtClaims {
            chatgpt_account_id: Some("account-1".to_string()),
            auth_claims: None,
            organizations: None,
        },
        Some("account-1".to_string());
        "uses top-level account id"
    )]
    #[test_case(
        JwtClaims {
            chatgpt_account_id: None,
            auth_claims: Some(AuthClaims {
                chatgpt_account_id: Some("account-2".to_string()),
            }),
            organizations: None,
        },
        Some("account-2".to_string());
        "uses auth claims account id"
    )]
    #[test_case(
        JwtClaims {
            chatgpt_account_id: None,
            auth_claims: None,
            organizations: Some(vec![OrgInfo {
                id: "org-1".to_string(),
            }]),
        },
        Some("org-1".to_string());
        "falls back to first organization"
    )]
    fn test_account_id_from_claims(claims: JwtClaims, expected: Option<String>) {
        assert_eq!(account_id_from_claims(&claims), expected);
    }

    #[tokio::test]
    async fn test_exchange_code_for_tokens() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains("code=code-123"))
            .and(body_string_contains(
                "redirect_uri=http%3A%2F%2Flocalhost%2Fcallback",
            ))
            .and(body_string_contains("code_verifier=verifier-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "access-1",
                "refresh_token": "refresh-1",
                "id_token": "id-1",
                "expires_in": 3600
            })))
            .mount(&server)
            .await;

        let pkce = PkceChallenge {
            verifier: "verifier-123".to_string(),
            challenge: "challenge-123".to_string(),
        };
        let tokens = auth::exchange_code_for_tokens_with_issuer(
            &server.uri(),
            "code-123",
            "http://localhost/callback",
            &pkce,
        )
        .await
        .unwrap();

        assert_eq!(tokens.access_token, "access-1");
        assert_eq!(tokens.refresh_token, "refresh-1");
        assert_eq!(tokens.id_token.as_deref(), Some("id-1"));
        assert_eq!(tokens.expires_in, Some(3600));
    }

    #[tokio::test]
    async fn test_refresh_access_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=refresh-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "access-2",
                "refresh_token": "refresh-2",
                "id_token": "id-2",
                "expires_in": 1800
            })))
            .mount(&server)
            .await;

        let tokens = auth::refresh_access_token_with_issuer(&server.uri(), "refresh-123")
            .await
            .unwrap();

        assert_eq!(tokens.access_token, "access-2");
        assert_eq!(tokens.refresh_token, "refresh-2");
        assert_eq!(tokens.id_token.as_deref(), Some("id-2"));
        assert_eq!(tokens.expires_in, Some(1800));
    }

    #[derive(Serialize)]
    struct TestClaims {
        exp: usize,
        chatgpt_account_id: Option<String>,
    }

    #[tokio::test]
    async fn test_parse_jwt_claims_verified_with_issuer() {
        let server = MockServer::start().await;
        let jwks_uri = format!("{}/jwks", server.uri());
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jwks_uri": jwks_uri
            })))
            .mount(&server)
            .await;

        let secret = "test-secret";
        let key = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret);
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "keys": [{
                    "kty": "oct",
                    "alg": "HS256",
                    "kid": "test-kid",
                    "k": key
                }]
            })))
            .mount(&server)
            .await;

        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("test-kid".to_string());

        let claims = TestClaims {
            exp: (Utc::now() + chrono::Duration::seconds(60)).timestamp() as usize,
            chatgpt_account_id: Some("account-1".to_string()),
        };
        let token = jsonwebtoken::encode(
            &header,
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let jwks = fetch_jwks_for(&server.uri()).await.unwrap();
        let claims = parse_jwt_claims_with_jwks(&token, &jwks).unwrap();

        assert_eq!(claims.chatgpt_account_id.as_deref(), Some("account-1"));
    }

    #[test_case("unknown-model", &["medium", "high"]; "unknown model gets default reasoning levels")]
    fn test_reasoning_levels_for_model(model: &str, expected: &[&str]) {
        assert_eq!(reasoning_levels_for_model(model), expected);
    }

    #[test]
    fn test_gpt53_preamble_injected() {
        let model = ModelConfig::new("gpt-5.3-codex").unwrap();
        let payload = create_codex_request(&model, "system prompt", &[], &[]).unwrap();
        let instructions = payload["instructions"].as_str().unwrap();
        assert!(instructions.contains(GPT_53_CODEX_TOOL_PREAMBLE));
        assert!(instructions.contains("system prompt"));
    }

    #[test]
    fn test_other_models_no_preamble() {
        let model = ModelConfig::new("gpt-5.4").unwrap();
        let payload = create_codex_request(&model, "system prompt", &[], &[]).unwrap();
        let instructions = payload["instructions"].as_str().unwrap();
        assert_eq!(instructions, "system prompt");
    }
}
