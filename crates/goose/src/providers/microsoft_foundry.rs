use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::StatusCode;
use serde_json::Value;
use std::io;
use tokio::pin;
use tokio_util::io::StreamReader;

use super::api_client::{ApiClient, ApiResponse, AuthMethod};
use super::base::{ConfigKey, MessageStream, ModelInfo, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::formats::anthropic as anthropic_format;
use super::formats::openai as openai_format;
use super::retry::ProviderRetry;
use super::utils::{
    get_model, handle_response_openai_compat, handle_status_openai_compat,
    map_http_error_to_provider_error, stream_openai_compat, ImageFormat, RequestLog,
};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

pub const MICROSOFT_FOUNDRY_DEFAULT_MODEL: &str = "claude-sonnet-4-5";
const MICROSOFT_FOUNDRY_DEFAULT_FAST_MODEL: &str = "claude-haiku-4-5";

// Anthropic API version header required by Microsoft Foundry
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

// OpenAI API version for Microsoft Foundry
const OPENAI_API_VERSION: &str = "2024-05-01-preview";

// Known Claude models (use Anthropic API format)
const FOUNDRY_CLAUDE_MODELS: &[(&str, usize)] = &[
    ("claude-sonnet-4-5", 200_000),
    ("claude-haiku-4-5", 200_000),
    ("claude-opus-4-5", 200_000),
    ("claude-opus-4-1", 200_000),
];

// Known OpenAI-compatible models (use OpenAI API format)
const FOUNDRY_OPENAI_MODELS: &[(&str, usize)] = &[
    ("gpt-4o", 128_000),
    ("gpt-4o-mini", 128_000),
    ("mistral-large", 128_000),
    ("llama-3.1-70b", 128_000),
    ("deepseek-v3", 128_000),
];

const MICROSOFT_FOUNDRY_DOC_URL: &str =
    "https://learn.microsoft.com/en-us/azure/ai-foundry/foundry-models/how-to/use-foundry-models-claude";

#[derive(serde::Serialize)]
pub struct MicrosoftFoundryProvider {
    #[serde(skip)]
    anthropic_client: ApiClient,
    #[serde(skip)]
    openai_client: ApiClient,
    #[serde(skip)]
    resource: String,
    model: ModelConfig,
    supports_streaming: bool,
    name: String,
}

impl std::fmt::Debug for MicrosoftFoundryProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicrosoftFoundryProvider")
            .field("resource", &self.resource)
            .field("model", &self.model)
            .field("supports_streaming", &self.supports_streaming)
            .field("name", &self.name)
            .finish()
    }
}

impl MicrosoftFoundryProvider {
    /// Determine if a model uses the Anthropic API format
    fn is_claude_model(model_name: &str) -> bool {
        model_name.starts_with("claude-")
    }

    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let model = model.with_fast(MICROSOFT_FOUNDRY_DEFAULT_FAST_MODEL.to_string());

        let config = crate::config::Config::global();

        // Required configuration
        let resource: String = config.get_param("MICROSOFT_FOUNDRY_RESOURCE")?;
        let api_key: String = config.get_secret("MICROSOFT_FOUNDRY_API_KEY")?;

        // Construct base URL from resource name
        let base_url = format!("https://{}.services.ai.azure.com", resource);

        // Create Anthropic-style client (for Claude models)
        // Auth header: x-api-key + anthropic-version header
        let anthropic_auth = AuthMethod::ApiKey {
            header_name: "x-api-key".to_string(),
            key: api_key.clone(),
        };
        let anthropic_client = ApiClient::new(base_url.clone(), anthropic_auth)?
            .with_header("anthropic-version", ANTHROPIC_API_VERSION)?;

        // Create OpenAI-style client (for other models)
        // Auth header: api-key
        let openai_auth = AuthMethod::ApiKey {
            header_name: "api-key".to_string(),
            key: api_key,
        };
        let openai_client = ApiClient::new(base_url, openai_auth)?;

        Ok(Self {
            anthropic_client,
            openai_client,
            resource,
            model,
            supports_streaming: true,
            name: Self::metadata().name,
        })
    }

    /// Make a POST request using Anthropic API format (for Claude models)
    async fn post_anthropic(&self, payload: &Value) -> Result<ApiResponse, ProviderError> {
        Ok(self
            .anthropic_client
            .request("anthropic/v1/messages")
            .api_post(payload)
            .await?)
    }

    /// Handle Anthropic API response
    fn anthropic_api_call_result(response: ApiResponse) -> Result<Value, ProviderError> {
        match response.status {
            StatusCode::OK => response.payload.ok_or_else(|| {
                ProviderError::RequestFailed("Response body is not valid JSON".to_string())
            }),
            _ => {
                // Check for context length error in 400 responses
                if response.status == StatusCode::BAD_REQUEST {
                    if let Some(error_msg) = response
                        .payload
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
                Err(map_http_error_to_provider_error(
                    response.status,
                    response.payload,
                ))
            }
        }
    }

    /// Make a POST request using OpenAI API format (for non-Claude models)
    async fn post_openai(&self, payload: &Value) -> Result<Value, ProviderError> {
        let endpoint = format!("models/chat/completions?api-version={}", OPENAI_API_VERSION);
        let response = self.openai_client.response_post(&endpoint, payload).await?;

        handle_response_openai_compat(response).await
    }

    /// Complete using Anthropic API format
    async fn complete_anthropic(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let payload = anthropic_format::create_request(model_config, system, messages, tools)?;
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async { self.post_anthropic(&payload).await })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let json_response = Self::anthropic_api_call_result(response).inspect_err(|e| {
            let _ = log.error(e);
        })?;

        let message = anthropic_format::response_to_message(&json_response)?;
        let usage = anthropic_format::get_usage(&json_response)?;

        let response_model = get_model(&json_response);
        log.write(&json_response, Some(&usage))?;
        let provider_usage = ProviderUsage::new(response_model, usage);

        Ok((message, provider_usage))
    }

    /// Complete using OpenAI API format
    async fn complete_openai(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let payload = openai_format::create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            false,
        )?;
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async { self.post_openai(&payload).await })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let message = openai_format::response_to_message(&response)?;
        let usage = response
            .get("usage")
            .map(openai_format::get_usage)
            .unwrap_or_default();

        let response_model = get_model(&response);
        log.write(&response, Some(&usage))?;
        let provider_usage = ProviderUsage::new(response_model, usage);

        Ok((message, provider_usage))
    }

    /// Stream using Anthropic API format
    async fn stream_anthropic(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = anthropic_format::create_request(&self.model, system, messages, tools)?;
        payload
            .as_object_mut()
            .unwrap()
            .insert("stream".to_string(), Value::Bool(true));

        let mut log = RequestLog::start(&self.model, &payload)?;

        let resp = self
            .anthropic_client
            .request("anthropic/v1/messages")
            .response_post(&payload)
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let response = handle_status_openai_compat(resp).await.inspect_err(|e| {
            let _ = log.error(e);
        })?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = tokio_util::codec::FramedRead::new(
                stream_reader,
                tokio_util::codec::LinesCodec::new()
            ).map_err(anyhow::Error::from);

            let message_stream = anthropic_format::response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = futures::StreamExt::next(&mut message_stream).await {
                let (message, usage) = message.map_err(|e|
                    ProviderError::RequestFailed(format!("Stream decode error: {}", e))
                )?;
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                yield (message, usage);
            }
        }))
    }

    /// Stream using OpenAI API format
    async fn stream_openai(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let payload = openai_format::create_request(
            &self.model,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            true, // for_streaming = true
        )?;

        let log = RequestLog::start(&self.model, &payload)?;

        let endpoint = format!("models/chat/completions?api-version={}", OPENAI_API_VERSION);
        let response = self
            .with_retry(|| async {
                let resp = self
                    .openai_client
                    .response_post(&endpoint, &payload)
                    .await?;
                handle_status_openai_compat(resp).await
            })
            .await?;

        stream_openai_compat(response, log)
    }
}

#[async_trait]
impl Provider for MicrosoftFoundryProvider {
    fn metadata() -> ProviderMetadata {
        // Combine Claude and OpenAI models
        let mut models: Vec<ModelInfo> = FOUNDRY_CLAUDE_MODELS
            .iter()
            .map(|&(name, limit)| ModelInfo::new(name, limit))
            .collect();

        models.extend(
            FOUNDRY_OPENAI_MODELS
                .iter()
                .map(|&(name, limit)| ModelInfo::new(name, limit)),
        );

        ProviderMetadata::with_models(
            "microsoft_foundry",
            "Microsoft Foundry",
            "Claude, GPT, and other models through Microsoft Foundry (Azure infrastructure)",
            MICROSOFT_FOUNDRY_DEFAULT_MODEL,
            models,
            MICROSOFT_FOUNDRY_DOC_URL,
            vec![
                ConfigKey::new("MICROSOFT_FOUNDRY_RESOURCE", true, false, None),
                ConfigKey::new("MICROSOFT_FOUNDRY_API_KEY", true, true, None),
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
        let model_name = &model_config.model_name;

        if Self::is_claude_model(model_name) {
            self.complete_anthropic(model_config, system, messages, tools)
                .await
        } else {
            self.complete_openai(model_config, system, messages, tools)
                .await
        }
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let model_name = &self.model.model_name;

        if Self::is_claude_model(model_name) {
            self.stream_anthropic(system, messages, tools).await
        } else {
            self.stream_openai(system, messages, tools).await
        }
    }

    fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_claude_model() {
        // Claude models should return true
        assert!(MicrosoftFoundryProvider::is_claude_model(
            "claude-sonnet-4-5"
        ));
        assert!(MicrosoftFoundryProvider::is_claude_model(
            "claude-haiku-4-5"
        ));
        assert!(MicrosoftFoundryProvider::is_claude_model("claude-opus-4-5"));
        assert!(MicrosoftFoundryProvider::is_claude_model("claude-opus-4-1"));
        assert!(MicrosoftFoundryProvider::is_claude_model(
            "claude-some-future-model"
        ));

        // Non-Claude models should return false
        assert!(!MicrosoftFoundryProvider::is_claude_model("gpt-4o"));
        assert!(!MicrosoftFoundryProvider::is_claude_model("gpt-4o-mini"));
        assert!(!MicrosoftFoundryProvider::is_claude_model("mistral-large"));
        assert!(!MicrosoftFoundryProvider::is_claude_model("llama-3.1-70b"));
        assert!(!MicrosoftFoundryProvider::is_claude_model("deepseek-v3"));
    }

    #[test]
    fn test_metadata() {
        let metadata = MicrosoftFoundryProvider::metadata();

        assert_eq!(metadata.name, "microsoft_foundry");
        assert_eq!(metadata.display_name, "Microsoft Foundry");
        assert_eq!(metadata.default_model, "claude-sonnet-4-5");

        // Check config keys
        assert_eq!(metadata.config_keys.len(), 2);
        assert!(metadata
            .config_keys
            .iter()
            .any(|k| k.name == "MICROSOFT_FOUNDRY_RESOURCE" && !k.secret));
        assert!(metadata
            .config_keys
            .iter()
            .any(|k| k.name == "MICROSOFT_FOUNDRY_API_KEY" && k.secret));

        // Check that we have both Claude and OpenAI models
        let model_names: Vec<&str> = metadata
            .known_models
            .iter()
            .map(|m| m.name.as_str())
            .collect();
        assert!(model_names.contains(&"claude-sonnet-4-5"));
        assert!(model_names.contains(&"gpt-4o"));
        assert!(model_names.contains(&"mistral-large"));
    }
}
