use anyhow::Result;
use async_trait::async_trait;
use axum::http::HeaderMap;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;

use super::base::{ConfigKey, ModelInfo, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::formats::anthropic::{create_request, get_usage, response_to_message};
use super::provider_common::{
    build_endpoint_url, get_shared_client, retry_with_backoff, AuthType, HeaderBuilder,
    ProviderConfigBuilder, RetryConfig,
};
use super::utils::{emit_debug_trace, get_model};
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;

pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-3-5-sonnet-latest";
pub const ANTHROPIC_KNOWN_MODELS: &[&str] = &[
    "claude-sonnet-4-latest",
    "claude-sonnet-4-20250514",
    "claude-opus-4-latest",
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
    client: Arc<Client>,
    host: String,
    api_key: String,
    model: ModelConfig,
    #[serde(skip)]
    retry_config: RetryConfig,
}

impl Default for AnthropicProvider {
    fn default() -> Self {
        let model = ModelConfig::new(AnthropicProvider::metadata().default_model);
        AnthropicProvider::from_env(model).expect("Failed to initialize Anthropic provider")
    }
}

impl AnthropicProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let config_builder = ProviderConfigBuilder::new(config, "ANTHROPIC");

        let api_key = config_builder.get_api_key()?;
        let host = config_builder.get_host("https://api.anthropic.com");

        // Use shared client for better connection pooling
        let client = get_shared_client();

        // Configure retry settings
        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 32000,
            backoff_multiplier: 2.0,
        };

        Ok(Self {
            client,
            host,
            api_key,
            model,
            retry_config,
        })
    }

    async fn post(&self, headers: HeaderMap, payload: Value) -> Result<Value, ProviderError> {
        let url = build_endpoint_url(&self.host, "v1/messages")?;

        retry_with_backoff(&self.retry_config, || async {
            let response = self
                .client
                .post(url.clone())
                .headers(headers.clone())
                .json(&payload)
                .send()
                .await?;

            let status = response.status();
            let payload: Option<Value> = response.json().await.ok();

            // https://docs.anthropic.com/en/api/errors
            match status {
                StatusCode::OK => payload.ok_or_else( || ProviderError::RequestFailed("Response body is not valid JSON".to_string()) ),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    Err(ProviderError::Authentication(format!("Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                        Status: {}. Response: {:?}", status, payload)))
                }
                StatusCode::BAD_REQUEST => {
                    let mut error_msg = "Unknown error".to_string();
                    if let Some(payload) = &payload {
                        if let Some(error) = payload.get("error") {
                        tracing::debug!("Bad Request Error: {error:?}");
                        error_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error").to_string();
                        if error_msg.to_lowercase().contains("too long") || error_msg.to_lowercase().contains("too many") {
                            return Err(ProviderError::ContextLengthExceeded(error_msg.to_string()));
                        }
                    }}
                    tracing::debug!(
                        "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
                    );
                    Err(ProviderError::RequestFailed(format!("Request failed with status: {}. Message: {}", status, error_msg)))
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    Err(ProviderError::RateLimitExceeded(format!("{:?}", payload)))
                }
                StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
                    Err(ProviderError::ServerError(format!("{:?}", payload)))
                }
                _ => {
                    tracing::debug!(
                        "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
                    );
                    Err(ProviderError::RequestFailed(format!("Request failed with status: {}", status)))
                }
            }
        }).await
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
                ModelInfo::new("claude-sonnet-4-latest", 200000),
                ModelInfo::new("claude-sonnet-4-20250514", 200000),
                ModelInfo::new("claude-opus-4-latest", 200000),
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

        // Build headers using the new HeaderBuilder
        let mut header_builder = HeaderBuilder::new(
            self.api_key.clone(),
            AuthType::Custom("x-api-key".to_string()),
        );
        header_builder = header_builder.add_custom_header(
            "anthropic-version".to_string(),
            ANTHROPIC_API_VERSION.to_string(),
        );

        let is_thinking_enabled = std::env::var("CLAUDE_THINKING_ENABLED").is_ok();
        if self.model.model_name.starts_with("claude-3-7-sonnet-") {
            if is_thinking_enabled {
                // https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking#extended-output-capabilities-beta
                header_builder = header_builder.add_custom_header(
                    "anthropic-beta".to_string(),
                    "output-128k-2025-02-19".to_string(),
                );
            } else {
                // https://docs.anthropic.com/en/docs/build-with-claude/tool-use/token-efficient-tool-use
                header_builder = header_builder.add_custom_header(
                    "anthropic-beta".to_string(),
                    "token-efficient-tools-2025-02-19".to_string(),
                );
            }
        }

        let headers = header_builder.build();

        // Make request
        let response = self.post(headers, payload.clone()).await?;

        // Parse response
        let message = response_to_message(response.clone())?;
        let usage = get_usage(&response)?;

        let model = get_model(&response);
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }

    /// Fetch supported models from Anthropic; returns Err on failure, Ok(None) if not present
    async fn fetch_supported_models_async(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let url = format!("{}/v1/models", self.host);
        let response = self
            .client
            .get(&url)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("x-api-key", self.api_key.clone())
            .send()
            .await?;
        let json: serde_json::Value = response.json().await?;
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
}
