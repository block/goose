use crate::base::ProviderDescriptor;
use crate::errors::ProviderError;
use crate::request_log::{start_log, LoggerHandleExt};
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::StatusCode;
use serde_json::Value;
use std::io;
use tokio::pin;
use tokio_util::io::StreamReader;

use super::api_client::ApiClient;
use super::base::{ConfigKey, MessageStream, ModelInfo, Provider, ProviderMetadata};
use super::formats::anthropic::{
    create_request, response_to_streaming_message, AnthropicFormatOptions, ANTHROPIC_PROVIDER_NAME,
};
use super::openai_compatible::handle_status;
use super::openai_compatible::map_http_error_to_provider_error;
use super::retry::ProviderRetry;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-sonnet-4-5";
pub const ANTHROPIC_DEFAULT_FAST_MODEL: &str = "claude-haiku-4-5";
const ANTHROPIC_KNOWN_MODELS: &[&str] = &[
    "claude-opus-4-8",
    "claude-opus-4-7",
    // Claude 4.6 models
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    // Claude 4.5 models with aliases
    "claude-sonnet-4-5",
    "claude-sonnet-4-5-20250929",
    "claude-haiku-4-5",
    "claude-haiku-4-5-20251001",
    "claude-opus-4-5",
    "claude-opus-4-5-20251101",
    // Legacy Claude 4.0 models
    "claude-sonnet-4-0",
    "claude-sonnet-4-20250514",
    "claude-opus-4-0",
    "claude-opus-4-20250514",
];

const ANTHROPIC_DOC_URL: &str = "https://docs.anthropic.com/en/docs/about-claude/models";
pub const ANTHROPIC_API_VERSION: &str = "2023-06-01";

#[derive(serde::Serialize)]
pub struct AnthropicProvider {
    #[serde(skip)]
    api_client: ApiClient,
    supports_streaming: bool,
    name: String,
    custom_models: Option<Vec<String>>,
    dynamic_models: Option<bool>,
    skip_canonical_filtering: bool,
    #[serde(skip)]
    format_options: AnthropicFormatOptions,
}

/// Builder for [`AnthropicProvider`].
///
/// Exposes every field of the provider so that constructors living outside
/// `anthropic.rs` (e.g. in `anthropic_def.rs`, which lives in the `goose`
/// crate) can assemble a provider without needing direct access to the
/// struct's private fields.
pub struct AnthropicProviderBuilder {
    api_client: ApiClient,
    supports_streaming: bool,
    name: String,
    custom_models: Option<Vec<String>>,
    dynamic_models: Option<bool>,
    skip_canonical_filtering: bool,
    format_options: AnthropicFormatOptions,
}

impl AnthropicProviderBuilder {
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client,
            supports_streaming: true,
            name: ANTHROPIC_PROVIDER_NAME.to_string(),
            custom_models: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            format_options: AnthropicFormatOptions::default(),
        }
    }

    pub fn supports_streaming(mut self, supports_streaming: bool) -> Self {
        self.supports_streaming = supports_streaming;
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn custom_models(mut self, custom_models: Option<Vec<String>>) -> Self {
        self.custom_models = custom_models;
        self
    }

    pub fn dynamic_models(mut self, dynamic_models: Option<bool>) -> Self {
        self.dynamic_models = dynamic_models;
        self
    }

    pub fn skip_canonical_filtering(mut self, skip_canonical_filtering: bool) -> Self {
        self.skip_canonical_filtering = skip_canonical_filtering;
        self
    }

    pub fn format_options(mut self, format_options: AnthropicFormatOptions) -> Self {
        self.format_options = format_options;
        self
    }

    pub fn build(self) -> AnthropicProvider {
        AnthropicProvider {
            api_client: self.api_client,
            supports_streaming: self.supports_streaming,
            name: self.name,
            custom_models: self.custom_models,
            dynamic_models: self.dynamic_models,
            skip_canonical_filtering: self.skip_canonical_filtering,
            format_options: self.format_options,
        }
    }
}

impl AnthropicProvider {
    async fn fetch_models_from_api(&self) -> Result<Vec<String>, ProviderError> {
        let response = self.api_client.request("v1/models").api_get().await?;

        if response.status == StatusCode::NOT_FOUND {
            let msg = response
                .payload
                .as_ref()
                .and_then(|p| p.get("error").and_then(|e| e.get("message")))
                .and_then(|m| m.as_str())
                .unwrap_or("models endpoint not found")
                .to_string();
            return Err(ProviderError::EndpointNotFound(msg));
        }

        if response.status != StatusCode::OK {
            return Err(map_http_error_to_provider_error(
                response.status,
                response.payload,
                "v1/models",
            ));
        }

        let json = response.payload.unwrap_or_default();
        let arr = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::RequestFailed(
                "Missing 'data' array in Anthropic models response".to_string(),
            )
        })?;

        let mut models: Vec<String> = arr
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        models.sort();
        Ok(models)
    }
}

impl ProviderDescriptor for AnthropicProvider {
    fn metadata() -> ProviderMetadata {
        let models: Vec<ModelInfo> = ANTHROPIC_KNOWN_MODELS
            .iter()
            .map(|&model_name| ModelInfo::new(model_name, 200_000))
            .collect();

        ProviderMetadata::with_models(
            ANTHROPIC_PROVIDER_NAME,
            "Anthropic",
            "Claude and other models from Anthropic",
            ANTHROPIC_DEFAULT_MODEL,
            models,
            ANTHROPIC_DOC_URL,
            vec![
                ConfigKey::new("ANTHROPIC_API_KEY", true, true, None, true),
                ConfigKey::new(
                    "ANTHROPIC_HOST",
                    true,
                    false,
                    Some("https://api.anthropic.com"),
                    false,
                ),
            ],
        )
        .with_fast_model(ANTHROPIC_DEFAULT_FAST_MODEL)
        .with_setup_steps(vec![
            "Go to https://platform.claude.com/settings/keys",
            "Click 'Create Key'",
            "Copy the key and paste it above",
        ])
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn skip_canonical_filtering(&self) -> bool {
        self.skip_canonical_filtering
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        if let Some(custom_models) = &self.custom_models {
            if self.dynamic_models == Some(false) {
                return Ok(custom_models.clone());
            }
            match self.fetch_models_from_api().await {
                Ok(models) => return Ok(models),
                Err(e) if e.is_endpoint_not_found() => {
                    tracing::debug!(
                        "Models endpoint not implemented for provider '{}' ({}), using predefined list",
                        self.name,
                        e
                    );
                    return Ok(custom_models.clone());
                }
                Err(e) => return Err(e),
            }
        }

        self.fetch_models_from_api().await
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_request(
            ANTHROPIC_PROVIDER_NAME,
            model_config,
            system,
            messages,
            tools,
            self.format_options,
        )?;
        payload
            .as_object_mut()
            .unwrap()
            .insert("stream".to_string(), Value::Bool(true));

        let mut log = start_log(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let request = self.api_client.request("v1/messages");
                let resp = request.response_post(&payload).await?;
                handle_status(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = tokio_util::codec::FramedRead::new(stream_reader, tokio_util::codec::LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = futures::StreamExt::next(&mut message_stream).await {
                let (message, usage) = message.map_err(ProviderError::from_stream_error)?;
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                yield (message, usage);
            }
        }))
    }
}
