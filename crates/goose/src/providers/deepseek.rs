use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::pin;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::embedding::EmbeddingCapable;
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::MessageStream;
use crate::providers::formats::openai::response_to_streaming_message;
use rmcp::model::Tool;

pub const DEEPSEEK_DEFAULT_MODEL: &str = "deepseek-chat";
pub const DEEPSEEK_KNOWN_MODELS: &[&str] = &[
    "deepseek-chat",
    "deepseek-reasoner",
];

pub const DEEPSEEK_DOC_URL: &str = "https://api-docs.deepseek.com/";

#[derive(Debug, serde::Serialize)]
pub struct DeepSeekProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    base_path: String,
    api_key: String,
    model: ModelConfig,
    custom_headers: Option<HashMap<String, String>>,
}

impl Default for DeepSeekProvider {
    fn default() -> Self {
        let model = ModelConfig::new(DeepSeekProvider::metadata().default_model);
        DeepSeekProvider::from_env(model).expect("Failed to initialize DeepSeek provider")
    }
}

impl DeepSeekProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("DEEPSEEK_API_KEY")?;
        let host: String = config
            .get_param("DEEPSEEK_HOST")
            .unwrap_or_else(|_| "https://api.deepseek.com".to_string());
        let base_path: String = config
            .get_param("DEEPSEEK_BASE_PATH")
            .unwrap_or_else(|_| "v1/chat/completions".to_string());
        let custom_headers: Option<HashMap<String, String>> = config
            .get_secret("DEEPSEEK_CUSTOM_HEADERS")
            .or_else(|_| config.get_param("DEEPSEEK_CUSTOM_HEADERS"))
            .ok()
            .map(parse_custom_headers);
        let timeout_secs: u64 = config.get_param("DEEPSEEK_TIMEOUT").unwrap_or(600);
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

    /// Helper function to add DeepSeek-specific headers to a request
    fn add_headers(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request = request.header("Authorization", format!("Bearer {}", self.api_key));
        request = request.header("Content-Type", "application/json");

        // Add custom headers if present
        if let Some(headers) = &self.custom_headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        request
    }

    async fn post(&self, payload: &Value) -> Result<Response, ProviderError> {
        let url = format!("{}/{}", self.host, self.base_path);
        let request = self.client.post(&url);
        let request = self.add_headers(request);

        let response = request
            .json(payload)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        Ok(response)
    }
}

#[async_trait::async_trait]
impl Provider for DeepSeekProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "deepseek",
            "DeepSeek",
            "DeepSeek AI models including DeepSeek-V3 and DeepSeek-R1",
            DEEPSEEK_DEFAULT_MODEL,
            DEEPSEEK_KNOWN_MODELS.iter().copied().collect(),
            DEEPSEEK_DOC_URL,
            vec![
                ConfigKey::new("DEEPSEEK_API_KEY", true, true, None),
                ConfigKey::new("DEEPSEEK_HOST", false, false, Some("https://api.deepseek.com")),
                ConfigKey::new("DEEPSEEK_BASE_PATH", false, false, Some("v1/chat/completions")),
                ConfigKey::new("DEEPSEEK_TIMEOUT", false, false, Some("600")),
                ConfigKey::new("DEEPSEEK_CUSTOM_HEADERS", false, false, None),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let payload = create_request(
            &self.model,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
        )?;

        emit_debug_trace(&self.model, &payload, &Value::Null, &Usage::default());

        let response = self.post(&payload).await?;
        let response_data = handle_response_openai_compat(response).await?;

        let model_name = get_model(&response_data);
        set_current_model(&model_name);

        let message = response_to_message(&response_data)?;
        let usage = get_usage(&response_data);

        emit_debug_trace(&self.model, &payload, &response_data, &usage);

        Ok((message, ProviderUsage::new(model_name, usage)))
    }

    async fn fetch_supported_models_async(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // DeepSeek doesn't provide a models endpoint, so we return the known models
        Ok(Some(DEEPSEEK_KNOWN_MODELS.iter().map(|s| s.to_string()).collect()))
    }

    fn supports_embeddings(&self) -> bool {
        false
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
        let mut payload = create_request(
            &self.model,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
        )?;

        // Add streaming parameter
        payload["stream"] = json!(true);

        emit_debug_trace(&self.model, &payload, &Value::Null, &Usage::default());

        let response = self.post(&payload).await?;
        let stream = response.bytes_stream().map_err(std::io::Error::other);

        let model_config = self.model.clone();
        // Wrap in a line decoder and yield lines inside the stream
        Ok(Box::pin(try_stream! {
            let stream_reader = tokio_util::io::StreamReader::new(stream);
            let framed = tokio_util::codec::FramedRead::new(stream_reader, tokio_util::codec::LinesCodec::new()).map_err(anyhow::Error::from);

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

#[async_trait]
impl EmbeddingCapable for DeepSeekProvider {
    async fn create_embeddings(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Err(anyhow::anyhow!("DeepSeek provider does not support embeddings"))
    }
}

fn parse_custom_headers(s: String) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in s.lines() {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    headers
}

// Import the set_current_model function
use super::base::set_current_model;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_metadata() {
        let metadata = DeepSeekProvider::metadata();
        assert_eq!(metadata.name, "deepseek");
        assert_eq!(metadata.display_name, "DeepSeek");
        assert_eq!(metadata.default_model, "deepseek-chat");
        assert!(metadata.known_models.iter().any(|m| m.name == "deepseek-chat"));
        assert!(metadata.known_models.iter().any(|m| m.name == "deepseek-reasoner"));
    }

    #[test]
    fn test_deepseek_config_keys() {
        let metadata = DeepSeekProvider::metadata();
        let api_key_config = metadata.config_keys.iter().find(|k| k.name == "DEEPSEEK_API_KEY");
        assert!(api_key_config.is_some());
        assert!(api_key_config.unwrap().required);
        assert!(api_key_config.unwrap().secret);
    }
} 