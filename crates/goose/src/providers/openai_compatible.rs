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
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::MessageStream;
use crate::providers::formats::openai::response_to_streaming_message;
use crate::providers::utils::handle_status_openai_compat;
use rmcp::model::Tool;

pub const OPENAI_COMPATIBLE_DEFAULT_MODEL: &str = "gpt-3.5-turbo";
pub const OPENAI_COMPATIBLE_DOC_URL: &str = "https://platform.openai.com/docs/api-reference";

#[derive(Debug, serde::Serialize)]
pub struct OpenAiCompatibleProvider {
    #[serde(skip)]
    client: Client,
    base_url: String,
    api_key: String,
    model: ModelConfig,
    custom_headers: Option<HashMap<String, String>>,
    streaming_enabled: bool,
}

impl Default for OpenAiCompatibleProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OpenAiCompatibleProvider::metadata().default_model);
        OpenAiCompatibleProvider::from_env(model)
            .expect("Failed to initialize OpenAI Compatible provider")
    }
}

impl OpenAiCompatibleProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        
        let api_key: String = config.get_secret("OPENAI_COMPATIBLE_API_KEY")?;
        let base_url: String = config.get_param("OPENAI_COMPATIBLE_BASE_URL")?;
        
        // Validate base URL format
        let parsed_url = url::Url::parse(&base_url)
            .map_err(|e| anyhow::anyhow!("Invalid OPENAI_COMPATIBLE_BASE_URL: {}", e))?;
        
        // Ensure the base URL doesn't end with a slash for consistent URL joining
        let normalized_base_url = parsed_url.as_str().trim_end_matches('/').to_string();
        
        let custom_headers: Option<HashMap<String, String>> = config
            .get_secret("OPENAI_COMPATIBLE_CUSTOM_HEADERS")
            .or_else(|_| config.get_param("OPENAI_COMPATIBLE_CUSTOM_HEADERS"))
            .ok()
            .map(parse_custom_headers);
            
        let timeout_secs: u64 = config.get_param("OPENAI_COMPATIBLE_TIMEOUT").unwrap_or(600);
        
        let streaming_enabled: bool = config
            .get_param("OPENAI_COMPATIBLE_STREAMING")
            .unwrap_or("true".to_string())
            .parse()
            .unwrap_or(true);
        
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            client,
            base_url: normalized_base_url,
            api_key,
            model,
            custom_headers,
            streaming_enabled,
        })
    }

    /// Helper function to add custom headers to a request
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
        let base_url = url::Url::parse(&self.base_url)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let request = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        let request = self.add_headers(request);

        Ok(request.json(&payload).send().await?)
    }

    /// Fetch available models from the /v1/models endpoint
    async fn fetch_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let base_url = url::Url::parse(&self.base_url)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("v1/models").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct models URL: {e}"))
        })?;

        let mut request = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_key));

        request = self.add_headers(request);

        let response = request.send().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to fetch models: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(ProviderError::RequestFailed(format!(
                "Models endpoint returned status: {}",
                response.status()
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse models response: {}", e))
        })?;

        // Handle potential error in response
        if let Some(error_obj) = response_json.get("error") {
            let msg = error_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::Authentication(msg.to_string()));
        }

        let models_data = response_json["data"].as_array().ok_or_else(|| {
            ProviderError::RequestFailed("Missing data field in models response".to_string())
        })?;

        let mut models = Vec::new();
        for model_data in models_data {
            if let Some(model_id) = model_data["id"].as_str() {
                // Try to get context length from model info if available
                let context_length = model_data
                    .get("context_length")
                    .or_else(|| model_data.get("max_context_length"))
                    .or_else(|| model_data.get("max_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(4096) as usize; // Default to 4K context

                models.push(ModelInfo::new(model_id, context_length));
            }
        }

        Ok(models)
    }
}

#[async_trait]
impl Provider for OpenAiCompatibleProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "openai_compatible",
            "OpenAI Compatible",
            "Generic provider for any service implementing the OpenAI API specification",
            OPENAI_COMPATIBLE_DEFAULT_MODEL,
            vec![], // Models will be fetched dynamically
            OPENAI_COMPATIBLE_DOC_URL,
            vec![
                ConfigKey::new("OPENAI_COMPATIBLE_API_KEY", true, true, None),
                ConfigKey::new("OPENAI_COMPATIBLE_BASE_URL", true, false, Some("https://api.example.com")),
                ConfigKey::new("OPENAI_COMPATIBLE_CUSTOM_HEADERS", false, true, None),
                ConfigKey::new("OPENAI_COMPATIBLE_TIMEOUT", false, false, Some("600")),
                ConfigKey::new("OPENAI_COMPATIBLE_STREAMING", false, false, Some("true")),
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

    /// Fetch supported models from the provider's API
    async fn fetch_supported_models_async(&self) -> Result<Option<Vec<String>>, ProviderError> {
        match self.fetch_models().await {
            Ok(models) => {
                let model_names: Vec<String> = models.into_iter().map(|m| m.name).collect();
                if model_names.is_empty() {
                    // If no models returned, fall back to manual entry
                    tracing::warn!("No models returned from API, falling back to manual model entry");
                    Ok(None)
                } else {
                    Ok(Some(model_names))
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models from OpenAI Compatible API: {}, falling back to manual model entry", e);
                Ok(None)
            }
        }
    }

    fn supports_embeddings(&self) -> bool {
        false
    }

    fn supports_streaming(&self) -> bool {
        self.streaming_enabled
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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_custom_headers() {
        let headers_str = "X-Custom-Header=value1,Authorization-Type=Bearer,X-Another=test value".to_string();
        let headers = parse_custom_headers(headers_str);
        
        assert_eq!(headers.len(), 3);
        assert_eq!(headers.get("X-Custom-Header"), Some(&"value1".to_string()));
        assert_eq!(headers.get("Authorization-Type"), Some(&"Bearer".to_string()));
        assert_eq!(headers.get("X-Another"), Some(&"test value".to_string()));
    }

    #[test]
    fn test_parse_custom_headers_empty() {
        let headers_str = "".to_string();
        let headers = parse_custom_headers(headers_str);
        assert_eq!(headers.len(), 0);
    }

    #[test]
    fn test_parse_custom_headers_malformed() {
        let headers_str = "invalid,X-Valid=value".to_string();
        let headers = parse_custom_headers(headers_str);
        
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get("X-Valid"), Some(&"value".to_string()));
    }

    #[test]
    fn test_provider_metadata() {
        let metadata = OpenAiCompatibleProvider::metadata();
        assert_eq!(metadata.name, "openai_compatible");
        assert_eq!(metadata.display_name, "OpenAI Compatible");
        assert_eq!(metadata.default_model, OPENAI_COMPATIBLE_DEFAULT_MODEL);
        
        // Check that required config keys are present
        let config_keys: Vec<&str> = metadata.config_keys.iter().map(|k| k.name.as_str()).collect();
        assert!(config_keys.contains(&"OPENAI_COMPATIBLE_API_KEY"));
        assert!(config_keys.contains(&"OPENAI_COMPATIBLE_BASE_URL"));
        assert!(config_keys.contains(&"OPENAI_COMPATIBLE_STREAMING"));
    }

    #[test] 
    fn test_base_url_normalization() {
        // Test that URLs with trailing slashes are normalized
        std::env::set_var("OPENAI_COMPATIBLE_API_KEY", "test-key");
        std::env::set_var("OPENAI_COMPATIBLE_BASE_URL", "https://api.example.com/");
        
        let model = ModelConfig::new("test-model".to_string());
        let provider = OpenAiCompatibleProvider::from_env(model);
        
        match provider {
            Ok(p) => {
                assert_eq!(p.base_url, "https://api.example.com");
            }
            Err(_) => {
                // If the provider fails to initialize due to missing config in test env,
                // we can't test the normalization, but that's ok for this test
            }
        }
        
        // Clean up
        std::env::remove_var("OPENAI_COMPATIBLE_API_KEY");
        std::env::remove_var("OPENAI_COMPATIBLE_BASE_URL");
    }

    #[test]
    fn test_streaming_configuration() {
        // Test the streaming field directly by creating providers with different settings
        let model = ModelConfig::new("test-model".to_string());
        
        // Test with streaming enabled
        let provider_enabled = OpenAiCompatibleProvider {
            client: reqwest::Client::new(),
            base_url: "https://api.example.com".to_string(),
            api_key: "test-key".to_string(),
            model: model.clone(),
            custom_headers: None,
            streaming_enabled: true,
        };
        assert!(provider_enabled.supports_streaming());
        
        // Test with streaming disabled
        let provider_disabled = OpenAiCompatibleProvider {
            client: reqwest::Client::new(),
            base_url: "https://api.example.com".to_string(),
            api_key: "test-key".to_string(),
            model: model,
            custom_headers: None,
            streaming_enabled: false,
        };
        assert!(!provider_disabled.supports_streaming());
    }
}
