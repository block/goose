use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::embedding::{EmbeddingCapable, EmbeddingRequest, EmbeddingResponse};
use super::errors::ProviderError;
use super::formats::openai::{create_request, response_to_message, response_to_streaming_message};
use super::utils::{emit_debug_trace, get_model, handle_status_openai_compat, ImageFormat};
use crate::conversation::message::Message;
use crate::impl_provider_default;
use crate::model::ModelConfig;
use crate::providers::base::MessageStream;
use rmcp::model::Tool;

pub const AIMLAPI_DEFAULT_MODEL: &str = "gpt-4o";
pub const AIMLAPI_KNOWN_MODELS: &[(&str, usize)] = &[
    // OpenAI models
    ("gpt-4o", 128_000),
    ("gpt-4o-mini", 128_000),
    ("gpt-3.5-turbo", 16_385),
    ("gpt-4-turbo", 128_000),
    ("o1", 200_000),
    ("o1-mini", 128_000),
    ("o1-preview", 128_000),
    // Anthropic models
    ("claude-3-5-sonnet-20241022", 200_000),
    ("claude-3-5-haiku-20241022", 200_000),
    ("claude-3-opus-20240229", 200_000),
    ("claude-3-sonnet-20240229", 200_000),
    ("claude-3-haiku-20240307", 200_000),
    // Google models
    ("gemini-2.0-flash-exp", 1_048_576),
    ("gemini-1.5-pro", 2_097_152),
    ("gemini-1.5-flash", 1_048_576),
    // DeepSeek models
    ("deepseek/deepseek-r1", 64_000),
    ("deepseek/deepseek-r1-distill-llama-70b", 64_000),
    ("deepseek/deepseek-chat", 64_000),
    // Mistral models
    ("mistral-large-latest", 128_000),
    ("mistral-medium-latest", 32_768),
    ("mistral-small-latest", 32_768),
    ("codestral-latest", 32_768),
    // Meta Llama models
    ("meta-llama/Meta-Llama-3.1-405B-Instruct", 128_000),
    ("meta-llama/Meta-Llama-3.1-70B-Instruct", 128_000),
    ("meta-llama/Meta-Llama-3.1-8B-Instruct", 128_000),
    // Qwen models
    ("Qwen/Qwen2.5-72B-Instruct", 32_768),
    ("Qwen/Qwen2.5-32B-Instruct", 32_768),
    ("Qwen/Qwen2.5-14B-Instruct", 32_768),
    ("Qwen/Qwen2.5-7B-Instruct", 32_768),
    // Other models
    ("databricks/dbrx-instruct", 32_768),
    ("NousResearch/Hermes-3-Llama-3.1-405B", 128_000),
];

pub const AIMLAPI_DOC_URL: &str = "https://docs.aimlapi.com";

#[derive(Debug, serde::Serialize)]
pub struct AimlApiProvider {
    #[serde(skip)]
    api_client: ApiClient,
    base_path: String,
    model: ModelConfig,
    custom_headers: Option<HashMap<String, String>>,
}

impl_provider_default!(AimlApiProvider);

impl AimlApiProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("AIMLAPI_API_KEY")?;
        let host: String = config
            .get_param("AIMLAPI_HOST")
            .unwrap_or_else(|_| "https://api.aimlapi.com".to_string());
        let base_path: String = config
            .get_param("AIMLAPI_BASE_PATH")
            .unwrap_or_else(|_| "v1/chat/completions".to_string());
        let custom_headers: Option<HashMap<String, String>> = config
            .get_secret("AIMLAPI_CUSTOM_HEADERS")
            .or_else(|_| config.get_param("AIMLAPI_CUSTOM_HEADERS"))
            .ok()
            .map(parse_custom_headers);
        let timeout_secs: u64 = config.get_param("AIMLAPI_TIMEOUT").unwrap_or(600);

        let auth = AuthMethod::BearerToken(api_key);
        let mut api_client =
            ApiClient::with_timeout(host, auth, std::time::Duration::from_secs(timeout_secs))?;

        if let Some(headers) = &custom_headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
                let header_value = reqwest::header::HeaderValue::from_str(value)?;
                header_map.insert(header_name, header_value);
            }
            api_client = api_client.with_headers(header_map)?;
        }

        Ok(Self {
            api_client,
            base_path,
            model,
            custom_headers,
        })
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        let response = self.api_client.api_post(&self.base_path, payload).await?;

        if response.status != reqwest::StatusCode::OK {
            let error_text = response
                .payload
                .as_ref()
                .and_then(|p| p.as_str())
                .unwrap_or("Unknown error");
            return Err(ProviderError::RequestFailed(format!(
                "Request failed with status: {}. Message: {}",
                response.status, error_text
            )));
        }

        response
            .payload
            .ok_or_else(|| ProviderError::RequestFailed("Empty response body".to_string()))
    }
}

fn parse_custom_headers(headers_str: String) -> HashMap<String, String> {
    headers_str
        .split(',')
        .filter_map(|s| {
            let parts: Vec<&str> = s.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
            } else {
                None
            }
        })
        .collect()
}

fn get_usage(usage_data: &Value) -> Usage {
    Usage::new(
        usage_data
            .get("prompt_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        usage_data
            .get("completion_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        usage_data
            .get("total_tokens")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
    )
}

#[async_trait]
impl Provider for AimlApiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "aimlapi",
            "AIML API",
            "Access 300+ AI models through a single API",
            AIMLAPI_DEFAULT_MODEL,
            AIMLAPI_KNOWN_MODELS
                .iter()
                .map(|(name, _)| *name)
                .collect(),
            AIMLAPI_DOC_URL,
            vec![ConfigKey::new("AIMLAPI_API_KEY", true, true, None)],
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
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;

        let json_response = self.post(&payload).await?;

        let message = response_to_message(&json_response)?;
        let usage = json_response
            .get("usage")
            .map(|u| get_usage(u))
            .unwrap_or_else(|| {
                tracing::debug!("Failed to get usage data");
                Usage::default()
            });
        let model = get_model(&json_response);
        emit_debug_trace(&self.model, &payload, &json_response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;
        payload["stream"] = serde_json::Value::Bool(true);
        payload["stream_options"] = json!({
            "include_usage": true,
        });

        let response = self
            .api_client
            .response_post(&self.base_path, &payload)
            .await?;
        let response = handle_status_openai_compat(response).await?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        let model_config = self.model.clone();

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                emit_debug_trace(&model_config, &payload, &message, &usage.as_ref().map(|f| f.usage).unwrap_or_default());
                yield (message, usage);
            }
        }))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_embeddings(&self) -> bool {
        true
    }

    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        EmbeddingCapable::create_embeddings(self, texts)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))
    }
}

#[async_trait]
impl EmbeddingCapable for AimlApiProvider {
    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let embedding_model = std::env::var("GOOSE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        let request = EmbeddingRequest {
            input: texts,
            model: embedding_model,
        };

        let response = self
            .api_client
            .api_post("v1/embeddings", &serde_json::to_value(request)?)
            .await?;

        if response.status != reqwest::StatusCode::OK {
            let error_text = response
                .payload
                .as_ref()
                .and_then(|p| p.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow::anyhow!("Embedding API error: {}", error_text));
        }

        let embedding_response: EmbeddingResponse = serde_json::from_value(
            response
                .payload
                .ok_or_else(|| anyhow::anyhow!("Empty response body"))?,
        )?;

        Ok(embedding_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }
}