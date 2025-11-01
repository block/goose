use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use serde_json::{json, Value};
use std::io;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::response_to_streaming_message;
use super::retry::ProviderRetry;
use super::utils::{
    get_model, handle_response_google_compat, handle_response_openai_compat,
    handle_status_openai_compat, is_google_model, RequestLog,
};
use crate::config::signup_tetrate::TETRATE_DEFAULT_MODEL;
use crate::conversation::message::Message;

use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use rmcp::model::Tool;

// Tetrate Agent Router Service can run many models, we suggest the default
pub const TETRATE_KNOWN_MODELS: &[&str] = &[
    "claude-opus-4-1",
    "claude-3-7-sonnet-latest",
    "claude-sonnet-4-20250514",
    "gemini-2.5-pro",
    "gemini-2.0-flash",
    "gemini-2.0-flash-lite",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
    "gpt-4.1",
];
pub const TETRATE_DOC_URL: &str = "https://router.tetrate.ai";

#[derive(serde::Serialize)]
pub struct TetrateProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    supports_streaming: bool,
    #[serde(skip)]
    name: String,
}

impl TetrateProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("TETRATE_API_KEY")?;
        // API host for LLM endpoints (/v1/chat/completions, /v1/models)
        let host: String = config
            .get_param("TETRATE_HOST")
            .unwrap_or_else(|_| "https://api.router.tetrate.ai".to_string());

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new(host, auth)?
            .with_header("HTTP-Referer", "https://block.github.io/goose")?
            .with_header("X-Title", "goose")?;

        Ok(Self {
            api_client,
            model,
            supports_streaming: true,
            name: Self::metadata().name,
        })
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post("v1/chat/completions", payload)
            .await?;

        // Handle Google-compatible model responses differently
        if is_google_model(payload) {
            return handle_response_google_compat(response).await;
        }

        // For OpenAI-compatible models, parse the response body to JSON
        let response_body = handle_response_openai_compat(response)
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse response: {e}")))?;

        let _debug = format!(
            "Tetrate Agent Router Service request with payload: {} and response: {}",
            serde_json::to_string_pretty(payload).unwrap_or_else(|_| "Invalid JSON".to_string()),
            serde_json::to_string_pretty(&response_body)
                .unwrap_or_else(|_| "Invalid JSON".to_string())
        );

        // Tetrate Agent Router Service can return errors in 200 OK responses, so we have to check for errors explicitly
        if let Some(error_obj) = response_body.get("error") {
            // If there's an error object, extract the error message and code
            let error_message = error_obj
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Tetrate Agent Router Service error");

            let error_code = error_obj.get("code").and_then(|c| c.as_u64()).unwrap_or(0);

            // Check for context length errors in the error message
            if error_code == 400 && error_message.contains("maximum context length") {
                return Err(ProviderError::ContextLengthExceeded(
                    error_message.to_string(),
                ));
            }

            // Return appropriate error based on the error code
            match error_code {
                401 | 403 => return Err(ProviderError::Authentication(error_message.to_string())),
                429 => {
                    return Err(ProviderError::RateLimitExceeded {
                        details: error_message.to_string(),
                        retry_delay: None,
                    })
                }
                500 | 503 => return Err(ProviderError::ServerError(error_message.to_string())),
                _ => return Err(ProviderError::RequestFailed(error_message.to_string())),
            }
        }

        // No error detected, return the response body
        Ok(response_body)
    }
}

#[async_trait]
impl Provider for TetrateProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "tetrate",
            "Tetrate Agent Router Service",
            "Enterprise router for AI models",
            TETRATE_DEFAULT_MODEL,
            TETRATE_KNOWN_MODELS.to_vec(),
            TETRATE_DOC_URL,
            vec![
                ConfigKey::new("TETRATE_API_KEY", true, true, None),
                ConfigKey::new(
                    "TETRATE_HOST",
                    false,
                    false,
                    Some("https://api.router.tetrate.ai"),
                ),
            ],
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
        let payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
        )?;
        let mut log = RequestLog::start(model_config, &payload)?;

        // Make request
        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post(&payload_clone).await
            })
            .await?;

        // Parse response
        let message = response_to_message(&response)?;
        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let model = get_model(&response);
        log.write(&response, Some(&usage))?;
        Ok((message, ProviderUsage::new(model, usage)))
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_request(
            &self.model,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
        )?;

        // Enable streaming
        payload["stream"] = json!(true);
        payload["stream_options"] = json!({
            "include_usage": true,
        });

        let response = self
            .api_client
            .response_post("v1/chat/completions", &payload)
            .await?;

        let response = handle_status_openai_compat(response).await?;
        let stream = response.bytes_stream().map_err(io::Error::other);
        let mut log = RequestLog::start(&self.model, &payload)?;

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                yield (message, usage);
            }
        }))
    }

    /// Fetch supported models from Tetrate Agent Router Service API (only models with tool support)
    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // Use the existing api_client which already has authentication configured
        let response = match self.api_client.response_get("v1/models").await {
            Ok(response) => response,
            Err(e) => {
                tracing::warn!("Failed to fetch models from Tetrate Agent Router Service API: {}, falling back to manual model entry", e);
                return Ok(None);
            }
        };

        // Handle JSON parsing failures gracefully
        let json: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to parse Tetrate Agent Router Service API response as JSON: {}, falling back to manual model entry", e);
                return Ok(None);
            }
        };

        // Check for error in response
        if let Some(err_obj) = json.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            tracing::warn!(
                "Tetrate Agent Router Service API returned an error: {}",
                msg
            );
            return Ok(None);
        }

        // The response format from /v1/models is expected to be OpenAI-compatible
        // It should have a "data" field with an array of model objects
        let data = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::UsageError("Missing data field in JSON response".into())
        })?;

        let mut models: Vec<String> = data
            .iter()
            .filter_map(|model| {
                // Get the model ID
                let id = model.get("id").and_then(|v| v.as_str())?;

                // Check if the model supports computer_use (which indicates tool/function support)
                // The Tetrate API uses "supports_computer_use" instead of "supported_parameters"
                let supports_computer_use = model
                    .get("supports_computer_use")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if supports_computer_use {
                    Some(id.to_string())
                } else {
                    tracing::debug!(
                        "Model '{}' does not support computer_use (tool support), skipping",
                        id
                    );
                    None
                }
            })
            .collect();

        // If no models with tool support were found, fall back to manual entry
        if models.is_empty() {
            tracing::warn!("No models with tool support found in Tetrate Agent Router Service API response, falling back to manual model entry");
            return Ok(None);
        }

        models.sort();
        Ok(Some(models))
    }

    fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }
}
