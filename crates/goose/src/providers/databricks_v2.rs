use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::future::BoxFuture;
use futures::TryStreamExt;
use goose_providers::formats::openai::{
    self, extract_reasoning_effort, is_openai_responses_model, ModelConfigParams,
};
use goose_providers::images::ImageFormat;
use serde::Serialize;
use serde_json::Value;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::pin;
use tokio_util::io::StreamReader;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{
    ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata,
    DEFAULT_PROVIDER_TIMEOUT_SECS,
};
use super::databricks_auth::{DatabricksAuth, DatabricksAuthProvider};
use super::formats::{anthropic, openai_responses};
use super::openai_compatible::{handle_status, stream_openai_compat, stream_responses_compat};
use super::retry::ProviderRetry;
use super::utils::RequestLog;
use crate::config::ConfigError;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::retry::{
    RetryConfig, DEFAULT_BACKOFF_MULTIPLIER, DEFAULT_INITIAL_RETRY_INTERVAL_MS,
    DEFAULT_MAX_RETRIES, DEFAULT_MAX_RETRY_INTERVAL_MS,
};
use goose_providers::errors::ProviderError;
use rmcp::model::Tool;

const DATABRICKS_V2_PROVIDER_NAME: &str = "databricks_v2";
const DATABRICKS_V2_LIST_ENDPOINTS_PATH: &str = "api/ai-gateway/v2/endpoints";
const DATABRICKS_V2_OPENAI_RESPONSES_PATH: &str = "ai-gateway/openai/v1/responses";
const DATABRICKS_V2_ANTHROPIC_MESSAGES_PATH: &str = "ai-gateway/anthropic/v1/messages";
const DATABRICKS_V2_MLFLOW_CHAT_COMPLETIONS_PATH: &str = "ai-gateway/mlflow/v1/chat/completions";
pub const DATABRICKS_V2_DEFAULT_MODEL: &str = "databricks-gpt-5-5";
pub const DATABRICKS_V2_KNOWN_MODELS: &[&str] =
    &["databricks-gpt-5-5", "databricks-claude-opus-4-7"];

pub const DATABRICKS_V2_DOC_URL: &str = "https://docs.databricks.com/en/generative-ai/ai-gateway/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DatabricksV2Route {
    OpenAiResponses,
    AnthropicMessages,
    MlflowChatCompletions,
}

#[derive(Debug, Serialize)]
pub struct DatabricksV2Provider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    #[serde(skip)]
    retry_config: RetryConfig,
    #[serde(skip)]
    name: String,
    #[serde(skip)]
    token_cache: Arc<Mutex<Option<String>>>,
}

impl DatabricksV2Provider {
    pub async fn cleanup() -> Result<()> {
        super::oauth::cleanup_oauth_cache()
    }

    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        let mut host: Result<String, ConfigError> = config.get_param("DATABRICKS_HOST");
        if host.is_err() {
            host = config.get_secret("DATABRICKS_HOST")
        }

        if host.is_err() {
            return Err(ConfigError::NotFound(
                "Did not find DATABRICKS_HOST in either config file or keyring".to_string(),
            )
            .into());
        }

        let host = host?;
        let retry_config = Self::load_retry_config(config);

        let auth = if let Ok(api_key) = config.get_secret("DATABRICKS_TOKEN") {
            DatabricksAuth::token(api_key)
        } else {
            DatabricksAuth::oauth(host.clone())
        };

        Self::new(host, auth, model, retry_config)
    }

    pub fn from_params(host: String, api_key: String, model: ModelConfig) -> Result<Self> {
        Self::new(
            host,
            DatabricksAuth::token(api_key),
            model,
            RetryConfig::default(),
        )
    }

    fn new(
        host: String,
        auth: DatabricksAuth,
        model: ModelConfig,
        retry_config: RetryConfig,
    ) -> Result<Self> {
        let token_cache = Arc::new(Mutex::new(match &auth {
            DatabricksAuth::Token(t) => Some(t.clone()),
            _ => None,
        }));

        let auth_method = AuthMethod::Custom(Box::new(DatabricksAuthProvider {
            auth: auth.clone(),
            token_cache: token_cache.clone(),
        }));

        let api_client = ApiClient::with_timeout(
            host,
            auth_method,
            Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS),
        )?;

        Ok(Self {
            api_client,
            model,
            retry_config,
            name: DATABRICKS_V2_PROVIDER_NAME.to_string(),
            token_cache,
        })
    }

    fn load_retry_config(config: &crate::config::Config) -> RetryConfig {
        let max_retries = config
            .get_param("DATABRICKS_MAX_RETRIES")
            .ok()
            .and_then(|v: String| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_RETRIES);

        let initial_interval_ms = config
            .get_param("DATABRICKS_INITIAL_RETRY_INTERVAL_MS")
            .ok()
            .and_then(|v: String| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_INITIAL_RETRY_INTERVAL_MS);

        let backoff_multiplier = config
            .get_param("DATABRICKS_BACKOFF_MULTIPLIER")
            .ok()
            .and_then(|v: String| v.parse::<f64>().ok())
            .unwrap_or(DEFAULT_BACKOFF_MULTIPLIER);

        let max_interval_ms = config
            .get_param("DATABRICKS_MAX_RETRY_INTERVAL_MS")
            .ok()
            .and_then(|v: String| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_MAX_RETRY_INTERVAL_MS);

        RetryConfig::new(
            max_retries,
            initial_interval_ms,
            backoff_multiplier,
            max_interval_ms,
        )
    }

    fn route_for_model(model_name: &str) -> DatabricksV2Route {
        let (clean_name, _) = extract_reasoning_effort(model_name);
        let lower = clean_name.to_lowercase();

        if is_openai_responses_model(&clean_name) || Self::looks_like_gpt5(&lower) {
            DatabricksV2Route::OpenAiResponses
        } else if Self::is_claude_model(&lower) {
            DatabricksV2Route::AnthropicMessages
        } else {
            DatabricksV2Route::MlflowChatCompletions
        }
    }

    fn looks_like_gpt5(model_name: &str) -> bool {
        model_name.contains("gpt-5") || model_name.contains("gpt5")
    }

    fn is_claude_model(model_name: &str) -> bool {
        model_name.contains("claude")
    }

    fn parse_list_endpoints_response(json: &Value) -> Result<Vec<String>, ProviderError> {
        let endpoints = json
            .get("endpoints")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ProviderError::RequestFailed(
                    "Unexpected response format from Databricks AI Gateway endpoints API"
                        .to_string(),
                )
            })?;

        let mut models: Vec<String> = endpoints
            .iter()
            .filter_map(|endpoint| {
                endpoint
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect();
        models.sort();
        Ok(models)
    }

    async fn stream_openai_responses(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload =
            openai_responses::create_responses_request(model_config, system, messages, tools)?;
        payload["stream"] = Value::Bool(true);
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post(
                        Some(session_id),
                        DATABRICKS_V2_OPENAI_RESPONSES_PATH,
                        &payload,
                    )
                    .await?;
                handle_status(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        stream_responses_compat(response, log)
    }

    async fn stream_mlflow_chat_completions(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = openai::create_request(
            ModelConfigParams {
                model_name: model_config.model_name.as_str(),
                thinking_effort: model_config.thinking_effort(),
                temperature: model_config.temperature,
                max_tokens: model_config.max_tokens,
                request_params: model_config.request_params.as_ref(),
            },
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            true,
        )?;
        if payload.get("max_tokens").is_none() {
            payload["max_tokens"] = Value::from(model_config.max_output_tokens());
        }
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post(
                        Some(session_id),
                        DATABRICKS_V2_MLFLOW_CHAT_COMPLETIONS_PATH,
                        &payload,
                    )
                    .await?;
                handle_status(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        stream_openai_compat(response, log)
    }

    async fn stream_anthropic_messages(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = anthropic::create_request(model_config, system, messages, tools)?;
        payload["stream"] = Value::Bool(true);
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post(
                        Some(session_id),
                        DATABRICKS_V2_ANTHROPIC_MESSAGES_PATH,
                        &payload,
                    )
                    .await?;
                handle_status(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = tokio_util::codec::FramedRead::new(stream_reader, tokio_util::codec::LinesCodec::new())
                .map_err(anyhow::Error::from);

            let message_stream = anthropic::response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = futures::StreamExt::next(&mut message_stream).await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {e}")))?;
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                yield (message, usage);
            }
        }))
    }
}

impl ProviderDef for DatabricksV2Provider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            DATABRICKS_V2_PROVIDER_NAME,
            "Databricks AI Gateway",
            "Models on Databricks AI Gateway v2",
            DATABRICKS_V2_DEFAULT_MODEL,
            DATABRICKS_V2_KNOWN_MODELS.to_vec(),
            DATABRICKS_V2_DOC_URL,
            vec![
                ConfigKey::new("DATABRICKS_HOST", true, false, None, true),
                ConfigKey::new("DATABRICKS_TOKEN", false, true, None, true),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }

    fn supports_inventory_refresh() -> bool {
        true
    }
}

#[async_trait]
impl Provider for DatabricksV2Provider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn retry_config(&self) -> RetryConfig {
        self.retry_config.clone()
    }

    async fn refresh_credentials(&self) -> Result<(), ProviderError> {
        crate::config::Config::global().invalidate_secrets_cache();
        *self.token_cache.lock().unwrap() = None;
        Ok(())
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
        match Self::route_for_model(&model_config.model_name) {
            DatabricksV2Route::OpenAiResponses => {
                self.stream_openai_responses(model_config, session_id, system, messages, tools)
                    .await
            }
            DatabricksV2Route::AnthropicMessages => {
                self.stream_anthropic_messages(model_config, session_id, system, messages, tools)
                    .await
            }
            DatabricksV2Route::MlflowChatCompletions => {
                self.stream_mlflow_chat_completions(
                    model_config,
                    session_id,
                    system,
                    messages,
                    tools,
                )
                .await
            }
        }
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .response_get(None, DATABRICKS_V2_LIST_ENDPOINTS_PATH)
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!(
                    "Failed to fetch Databricks AI Gateway endpoints: {e}"
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            return Err(ProviderError::RequestFailed(format!(
                "Failed to fetch Databricks AI Gateway endpoints: {status} {detail}"
            )));
        }

        let json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to parse Databricks AI Gateway endpoints response: {e}"
            ))
        })?;

        Self::parse_list_endpoints_response(&json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn routes_known_model_families() {
        for model in ["databricks-gpt-5-5", "databricks-gpt5"] {
            assert_eq!(
                DatabricksV2Provider::route_for_model(model),
                DatabricksV2Route::OpenAiResponses,
                "unexpected route for {model}"
            );
        }

        for model in ["databricks-claude-opus-4-7", "databricks-claude-sonnet-4-6"] {
            assert_eq!(
                DatabricksV2Provider::route_for_model(model),
                DatabricksV2Route::AnthropicMessages,
                "unexpected route for {model}"
            );
        }

        assert_eq!(
            DatabricksV2Provider::route_for_model("custom-model"),
            DatabricksV2Route::MlflowChatCompletions
        );
    }

    #[tokio::test]
    async fn claude_models_use_anthropic_messages_endpoint_and_payload() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/{DATABRICKS_V2_ANTHROPIC_MESSAGES_PATH}")))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                [
                    "event: message_start\n",
                    "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"model\":\"goose-claude-fable-5\",\"usage\":{\"input_tokens\":1}}}\n\n",
                    "event: content_block_start\n",
                    "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
                    "event: content_block_delta\n",
                    "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n",
                    "event: content_block_stop\n",
                    "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
                    "event: message_delta\n",
                    "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":1}}\n\n",
                    "event: message_stop\n",
                    "data: {\"type\":\"message_stop\"}\n\n",
                ]
                .concat(),
                "text/event-stream",
            ))
            .mount(&server)
            .await;

        let provider = DatabricksV2Provider::from_params(
            server.uri(),
            "test-token".to_string(),
            ModelConfig::new_or_fail("goose-claude-fable-5"),
        )
        .unwrap();

        let stream = provider
            .stream(
                &ModelConfig::new_or_fail("goose-claude-fable-5"),
                "session-id",
                "",
                &[
                    Message::user().with_text("Hello!"),
                    Message::assistant().with_text("Hello! How can I assist you today?"),
                    Message::user().with_text("What is Databricks?"),
                ],
                &[],
            )
            .await
            .unwrap();
        drop(stream);

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url.path(), "/ai-gateway/anthropic/v1/messages");

        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["model"], "goose-claude-fable-5");
        assert_eq!(body["max_tokens"], json!(4096));
        assert_eq!(body["stream"], json!(true));
        assert!(body.get("max_completion_tokens").is_none());
        assert!(body.get("stream_options").is_none());
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"][0]["text"], "Hello!");
        assert_eq!(body["messages"][1]["role"], "assistant");
        assert_eq!(
            body["messages"][1]["content"][0]["text"],
            "Hello! How can I assist you today?"
        );
        assert_eq!(body["messages"][2]["role"], "user");
        assert_eq!(
            body["messages"][2]["content"][0]["text"],
            "What is Databricks?"
        );
    }

    #[tokio::test]
    async fn non_claude_models_use_mlflow_chat_completions_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/{DATABRICKS_V2_MLFLOW_CHAT_COMPLETIONS_PATH}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: [DONE]\n\n",
                "text/event-stream",
            ))
            .mount(&server)
            .await;

        let provider = DatabricksV2Provider::from_params(
            server.uri(),
            "test-token".to_string(),
            ModelConfig::new_or_fail("custom-model"),
        )
        .unwrap();

        let stream = provider
            .stream(
                &ModelConfig::new_or_fail("custom-model"),
                "session-id",
                "",
                &[Message::user().with_text("Hello!")],
                &[],
            )
            .await
            .unwrap();
        drop(stream);

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].url.path(),
            "/ai-gateway/mlflow/v1/chat/completions"
        );

        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["model"], "custom-model");
        let user_message = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "user")
            .unwrap();
        assert_eq!(user_message["content"], "Hello!");
    }

    #[test]
    fn parses_list_endpoints_response() {
        let json = serde_json::json!({
            "endpoints": [
                {"name": "databricks-claude-opus-4-7"},
                {"name": "databricks-gpt-5-5"},
                {"name": "custom-model"}
            ]
        });

        let models = DatabricksV2Provider::parse_list_endpoints_response(&json).unwrap();

        assert_eq!(
            models,
            vec![
                "custom-model".to_string(),
                "databricks-claude-opus-4-7".to_string(),
                "databricks-gpt-5-5".to_string(),
            ]
        );
    }

    #[test]
    fn errors_when_list_endpoints_response_has_no_endpoints_array() {
        let json = serde_json::json!({"data": []});

        let error = DatabricksV2Provider::parse_list_endpoints_response(&json).unwrap_err();

        assert!(matches!(error, ProviderError::RequestFailed(_)));
        assert!(error
            .to_string()
            .contains("Unexpected response format from Databricks AI Gateway endpoints API"));
    }
}
