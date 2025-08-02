use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use axum::http::HeaderMap;
use futures::TryStreamExt;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::io;
use std::time::Duration;
use tokio::pin;

use tokio_util::io::StreamReader;

use super::base::{ConfigKey, MessageStream, ModelInfo, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::formats::anthropic::{
    create_request, get_usage, response_to_message, response_to_streaming_message,
};
use super::utils::{emit_debug_trace, get_model, map_http_error_to_provider_error};
use crate::impl_provider_default;
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::retry::ProviderRetry;
use rmcp::model::Tool;

pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-3-5-sonnet-latest";
pub const ANTHROPIC_KNOWN_MODELS: &[&str] = &[
    "claude-sonnet-4-0",
    "claude-sonnet-4-20250514",
    "claude-opus-4-0",
    "claude-opus-4-20250514",
    "claude-3-7-sonnet-latest",
    "claude-3-7-sonnet-20250219",
    "claude-3-5-sonnet-latest",
    "claude-3-5-haiku-latest",
    "claude-3-opus-latest",
];

pub const ANTHROPIC_DOC_URL: &str = "https://docs.anthropic.com/en/docs/about-claude/models";
pub const ANTHROPIC_API_VERSION: &str = "2023-06-01";

#[derive(serde::Serialize)]
pub struct AnthropicProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl_provider_default!(AnthropicProvider);

impl AnthropicProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("ANTHROPIC_API_KEY")?;
        let host: String = config
            .get_param("ANTHROPIC_HOST")
            .unwrap_or_else(|_| "https://api.anthropic.com".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            api_key,
            model,
        })
    }

    async fn post(&self, headers: HeaderMap, payload: &Value) -> Result<Value, ProviderError> {
        let base_url = url::Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("v1/messages").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(payload)
            .send()
            .await?;

        let status = response.status();
        let payload: Option<Value> = response.json().await.ok();
        Self::anthropic_api_call_result(status, payload)
    }

    fn anthropic_api_call_result(
        status: StatusCode,
        payload: Option<Value>,
    ) -> Result<Value, ProviderError> {
        // https://docs.anthropic.com/en/api/errors
        match status {
            StatusCode::OK => payload.ok_or_else(|| {
                ProviderError::RequestFailed("Response body is not valid JSON".to_string())
            }),
            _ => {
                if status == StatusCode::BAD_REQUEST {
                    if let Some(error_msg) = payload
                        .as_ref()
                        .and_then(|p| p.get("error"))
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                    {
                        let msg = error_msg.to_string();
                        if msg.to_lowercase().contains("too long")
                            || msg.to_lowercase().contains("too many")
                        {
                            return Err(ProviderError::ContextLengthExceeded(msg));
                        }
                    }
                }
                Err(map_http_error_to_provider_error(status, payload))
            }
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::with_models(
            "anthropic",
            "Anthropic",
            "Claude and other models from Anthropic",
            ANTHROPIC_DEFAULT_MODEL,
            vec![
                ModelInfo::new("claude-sonnet-4-0", 200000),
                ModelInfo::new("claude-sonnet-4-20250514", 200000),
                ModelInfo::new("claude-opus-4-0", 200000),
                ModelInfo::new("claude-opus-4-20250514", 200000),
                ModelInfo::new("claude-3-7-sonnet-latest", 200000),
                ModelInfo::new("claude-3-7-sonnet-20250219", 200000),
                ModelInfo::new("claude-3-5-sonnet-20241022", 200000),
                ModelInfo::new("claude-3-5-haiku-20241022", 200000),
                ModelInfo::new("claude-3-opus-20240229", 200000),
                ModelInfo::new("claude-3-sonnet-20240229", 200000),
                ModelInfo::new("claude-3-haiku-20240307", 200000),
            ],
            ANTHROPIC_DOC_URL,
            vec![
                ConfigKey::new("ANTHROPIC_API_KEY", true, true, None),
                ConfigKey::new(
                    "ANTHROPIC_HOST",
                    true,
                    false,
                    Some("https://api.anthropic.com"),
                ),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let payload = create_request(&self.model, system, messages, tools)?;

        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", self.api_key.parse().unwrap());
        headers.insert("anthropic-version", ANTHROPIC_API_VERSION.parse().unwrap());

        let is_thinking_enabled = std::env::var("CLAUDE_THINKING_ENABLED").is_ok();
        if self.model.model_name.starts_with("claude-3-7-sonnet-") && is_thinking_enabled {
            // https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking#extended-output-capabilities-beta
            headers.insert("anthropic-beta", "output-128k-2025-02-19".parse().unwrap());
        }

        if self.model.model_name.starts_with("claude-3-7-sonnet-") {
            // https://docs.anthropic.com/en/docs/build-with-claude/tool-use/token-efficient-tool-use
            headers.insert(
                "anthropic-beta",
                "token-efficient-tools-2025-02-19".parse().unwrap(),
            );
        }

        let response = self
            .with_retry(|| async {
                let headers_clone = headers.clone();
                let payload_clone = payload.clone();
                self.post(headers_clone, &payload_clone).await
            })
            .await?;

        let message = response_to_message(&response.clone())?;
        let usage = get_usage(&response)?;
        tracing::debug!("🔍 Anthropic non-streaming parsed usage: input_tokens={:?}, output_tokens={:?}, total_tokens={:?}", 
                usage.input_tokens, usage.output_tokens, usage.total_tokens);

        let model = get_model(&response);
        emit_debug_trace(&self.model, &payload, &response, &usage);
        let provider_usage = ProviderUsage::new(model, usage);
        tracing::debug!(
            "🔍 Anthropic non-streaming returning ProviderUsage: {:?}",
            provider_usage
        );
        Ok((message, provider_usage))
    }

    /// Fetch supported models from Anthropic; returns Err on failure, Ok(None) if not present
    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let url = format!("{}/v1/models", self.host);
        let response = self
            .client
            .get(&url)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("x-api-key", self.api_key.clone())
            .send()
            .await?;
        let json: Value = response.json().await?;
        // if 'models' key missing, return None
        let arr = match json.get("models").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return Ok(None),
        };
        let mut models: Vec<String> = arr
            .iter()
            .filter_map(|m| {
                if let Some(s) = m.as_str() {
                    Some(s.to_string())
                } else if let Some(obj) = m.as_object() {
                    obj.get("id").and_then(|v| v.as_str()).map(str::to_string)
                } else {
                    None
                }
            })
            .collect();
        models.sort();
        Ok(Some(models))
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_request(&self.model, system, messages, tools)?;

        // Add stream parameter
        payload
            .as_object_mut()
            .unwrap()
            .insert("stream".to_string(), Value::Bool(true));

        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", self.api_key.parse().unwrap());
        headers.insert("anthropic-version", ANTHROPIC_API_VERSION.parse().unwrap());

        let is_thinking_enabled = std::env::var("CLAUDE_THINKING_ENABLED").is_ok();
        if self.model.model_name.starts_with("claude-3-7-sonnet-") && is_thinking_enabled {
            // https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking#extended-output-capabilities-beta
            headers.insert("anthropic-beta", "output-128k-2025-02-19".parse().unwrap());
        }

        if self.model.model_name.starts_with("claude-3-7-sonnet-") {
            // https://docs.anthropic.com/en/docs/build-with-claude/tool-use/token-efficient-tool-use
            headers.insert(
                "anthropic-beta",
                "token-efficient-tools-2025-02-19".parse().unwrap(),
            );
        }

        let base_url = url::Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("v1/messages").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let error_json = serde_json::from_str::<Value>(&error_text).ok();
            return Err(map_http_error_to_provider_error(status, error_json));
        }

        // Map reqwest error to io::Error
        let stream = response.bytes_stream().map_err(io::Error::other);

        let model_config = self.model.clone();
        // Wrap in a line decoder and yield lines inside the stream
        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = tokio_util::codec::FramedRead::new(stream_reader, tokio_util::codec::LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = futures::StreamExt::next(&mut message_stream).await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                emit_debug_trace(&model_config, &payload, &message, &usage.as_ref().map(|f| f.usage).unwrap_or_default());
                yield (message, usage);
            }
        }))
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
