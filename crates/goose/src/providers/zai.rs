use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io;
use std::time::Duration;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

use super::base::{ConfigKey, ModelInfo, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::impl_provider_default;
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::MessageStream;
use crate::providers::formats::openai::response_to_streaming_message;
use crate::providers::utils::handle_status_openai_compat;
use rmcp::model::Tool;

pub const ZAI_DEFAULT_MODEL: &str = "glm-4.5";
pub const ZAI_KNOWN_MODELS: &[&str] = &["glm-4.5", "glm-4.5-air"];

pub const ZAI_DOC_URL: &str = "https://z.ai/docs";

#[derive(Debug, serde::Serialize)]
pub struct ZaiProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    base_path: String,
    api_key: String,
    model: ModelConfig,
    custom_headers: Option<HashMap<String, String>>,
}

impl_provider_default!(ZaiProvider);

impl ZaiProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("ZAI_API_KEY")?;
        let host: String = config
            .get_param("ZAI_HOST")
            .unwrap_or_else(|_| "https://api.z.ai".to_string());
        let base_path: String = config
            .get_param("ZAI_BASE_PATH")
            .unwrap_or_else(|_| "api/paas/v4/chat/completions".to_string());
        let custom_headers: Option<HashMap<String, String>> = config
            .get_secret("ZAI_CUSTOM_HEADERS")
            .or_else(|_| config.get_param("ZAI_CUSTOM_HEADERS"))
            .ok()
            .map(parse_custom_headers);
        let timeout_secs: u64 = config.get_param("ZAI_TIMEOUT").unwrap_or(600);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            client,
            host,
            base_path,
            api_key,
            model,
            custom_headers,
        })
    }

    /// Helper function to add Z.AI-specific headers to a request
    fn add_headers(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        // Add custom headers if present
        if let Some(custom_headers) = &self.custom_headers {
            for (key, value) in custom_headers {
                request = request.header(key, value);
            }
        }

        request
    }

    async fn post(&self, payload: &Value) -> Result<Response, ProviderError> {
        let base_url = url::Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join(&self.base_path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let request = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://block.github.io/goose")
            .header("X-Title", "Goose");

        let request = self.add_headers(request);

        Ok(request.json(&payload).send().await?)
    }
}

#[async_trait]
impl Provider for ZaiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::with_models(
            "zai",
            "Z.AI",
            "Z.AI provides access to GLM-4.5 and other advanced language models",
            ZAI_DEFAULT_MODEL,
            vec![
                ModelInfo {
                    name: "glm-4.5".to_string(),
                    context_limit: 131072,
                    input_token_cost: Some(0.6),
                    output_token_cost: Some(2.2),
                    currency: Some("$".to_string()),
                    supports_cache_control: Some(true),
                },
                ModelInfo {
                    name: "glm-4.5-air".to_string(),
                    context_limit: 131072,
                    input_token_cost: Some(0.2),
                    output_token_cost: Some(1.1),
                    currency: Some("$".to_string()),
                    supports_cache_control: Some(true),
                },
            ],
            ZAI_DOC_URL,
            vec![
                ConfigKey::new("ZAI_API_KEY", true, true, None),
                ConfigKey::new("ZAI_HOST", true, false, Some("https://api.z.ai")),
                ConfigKey::new("ZAI_BASE_PATH", true, false, Some("api/paas/v4/chat/completions")),
                ConfigKey::new("ZAI_CUSTOM_HEADERS", false, true, None),
                ConfigKey::new("ZAI_TIMEOUT", false, false, Some("600")),
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
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;

        // Make request
        let response = handle_response_openai_compat(self.post(&payload).await?).await?;

        // Parse response
        let message = response_to_message(&response)?;
        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let model = get_model(&response);
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
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
        let mut payload =
            create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;
        payload["stream"] = serde_json::Value::Bool(true);
        payload["stream_options"] = json!({
            "include_usage": true,
        });

        let response = handle_status_openai_compat(self.post(&payload).await?).await?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        let model_config = self.model.clone();
        // Wrap in a line decoder and yield lines inside the stream
        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                super::utils::emit_debug_trace(&model_config, &payload, &message, &usage.as_ref().map(|f| f.usage).unwrap_or_default());
                yield (message, usage);
            }
        }))
    }
}

fn parse_custom_headers(s: String) -> HashMap<String, String> {
    s.split(',')
        .filter_map(|header| {
            let mut parts = header.splitn(2, '=');
            let key = parts.next().map(|s| s.trim().to_string())?;
            let value = parts.next().map(|s| s.trim().to_string())?;
            Some((key, value))
        })
        .collect()
}