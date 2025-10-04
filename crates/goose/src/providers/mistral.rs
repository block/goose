use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::StatusCode;
use serde_json::Value;
use std::io;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, ModelInfo, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::formats::openai::{
    create_request, get_usage, response_to_message, response_to_streaming_message,
};
use super::retry::ProviderRetry;
use super::utils::{
    emit_debug_trace, get_model, handle_response_openai_compat, handle_status_openai_compat,
    map_http_error_to_provider_error, ImageFormat,
};
use crate::config::custom_providers::CustomProviderConfig;
use crate::conversation::message::Message;
use crate::impl_provider_default;
use crate::model::ModelConfig;
use rmcp::model::Tool;

const MISTRAL_DOC_URL: &str = "https://docs.mistral.ai/";
const MISTRAL_DEFAULT_HOST: &str = "https://api.mistral.ai";
const MISTRAL_DEFAULT_BASE_PATH: &str = "v1/chat/completions";
const MISTRAL_DEFAULT_MODELS_PATH: &str = "v1/models";
const MISTRAL_DEFAULT_MODEL: &str = "mistral-medium-latest";
const MISTRAL_DEFAULT_FAST_MODEL: &str = "mistral-small-2506";

const MISTRAL_KNOWN_MODELS: &[(&str, usize)] = &[
    ("mistral-medium-2508", 128_000),
    ("magistral-medium-2509", 128_000),
    ("codestral-2508", 256_000),
    ("pixtral-large-2411", 128_000),
    ("ministral-8b-2410", 128_000),
    ("mistral-medium-2505", 128_000),
    ("ministral-3b-2410", 128_000),
    ("mistral-small-2506", 128_000),
];

#[derive(serde::Serialize)]
pub struct MistralProvider {
    #[serde(skip)]
    api_client: ApiClient,
    base_path: String,
    models_path: String,
    model: ModelConfig,
    supports_streaming: bool,
}

impl_provider_default!(MistralProvider);

impl MistralProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let model = model.with_fast(MISTRAL_DEFAULT_FAST_MODEL.to_string());

        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("MISTRAL_API_KEY")?;
        let host: String = config
            .get_param("MISTRAL_HOST")
            .unwrap_or_else(|_| MISTRAL_DEFAULT_HOST.to_string());
        let base_path: String = config
            .get_param("MISTRAL_BASE_PATH")
            .unwrap_or_else(|_| MISTRAL_DEFAULT_BASE_PATH.to_string());
        let models_path: String = config
            .get_param("MISTRAL_MODELS_PATH")
            .unwrap_or_else(|_| MISTRAL_DEFAULT_MODELS_PATH.to_string());

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new(host, auth)?;

        Ok(Self {
            api_client,
            base_path,
            models_path,
            model,
            supports_streaming: true,
        })
    }

    pub fn from_custom_config(model: ModelConfig, config: CustomProviderConfig) -> Result<Self> {
        let global_config = crate::config::Config::global();
        let api_key: String = global_config
            .get_secret(&config.api_key_env)
            .map_err(|_| anyhow::anyhow!("Missing API key: {}", config.api_key_env))?;

        let url = url::Url::parse(&config.base_url)
            .map_err(|e| anyhow::anyhow!("Invalid base URL '{}': {}", config.base_url, e))?;

        let host = if let Some(port) = url.port() {
            format!(
                "{}://{}:{}",
                url.scheme(),
                url.host_str().unwrap_or(""),
                port
            )
        } else {
            format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""))
        };
        let base_path = url.path().trim_start_matches('/').to_string();
        let base_path = if base_path.is_empty() {
            MISTRAL_DEFAULT_BASE_PATH.to_string()
        } else {
            base_path
        };

        let auth = AuthMethod::BearerToken(api_key);
        let mut api_client = ApiClient::new(host, auth)?;

        if let Some(headers) = &config.headers {
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
            models_path: MISTRAL_DEFAULT_MODELS_PATH.to_string(),
            model,
            supports_streaming: config.supports_streaming.unwrap_or(true),
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post(&self.base_path, &payload)
            .await?;
        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for MistralProvider {
    fn metadata() -> ProviderMetadata {
        let models: Vec<ModelInfo> = MISTRAL_KNOWN_MODELS
            .iter()
            .map(|(name, limit)| ModelInfo::new(*name, *limit))
            .collect();

        ProviderMetadata::with_models(
            "mistral",
            "Mistral AI",
            "Frontier models from Mistral AI",
            MISTRAL_DEFAULT_MODEL,
            models,
            MISTRAL_DOC_URL,
            vec![
                ConfigKey::new("MISTRAL_API_KEY", true, true, None),
                ConfigKey::new("MISTRAL_HOST", false, false, Some(MISTRAL_DEFAULT_HOST)),
                ConfigKey::new(
                    "MISTRAL_BASE_PATH",
                    false,
                    false,
                    Some(MISTRAL_DEFAULT_BASE_PATH),
                ),
                ConfigKey::new(
                    "MISTRAL_MODELS_PATH",
                    false,
                    false,
                    Some(MISTRAL_DEFAULT_MODELS_PATH),
                ),
            ],
        )
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
        let payload = create_request(model_config, system, messages, tools, &ImageFormat::OpenAi)?;

        let response = self.with_retry(|| self.post(payload.clone())).await?;

        let message = response_to_message(&response)?;
        let usage = response.get("usage").map(get_usage).unwrap_or_default();
        let response_model = get_model(&response);
        emit_debug_trace(model_config, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(response_model, usage)))
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let response = self.api_client.api_get(&self.models_path).await?;

        if response.status != StatusCode::OK {
            return Err(map_http_error_to_provider_error(
                response.status,
                response.payload,
            ));
        }

        let payload = response.payload.unwrap_or_default();
        let data = match payload.get("data").and_then(|v| v.as_array()) {
            Some(data) => data,
            None => return Ok(None),
        };

        let mut models: Vec<String> = data
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
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
        if !self.supports_streaming {
            return Err(ProviderError::NotImplemented(
                "Streaming is disabled for this provider configuration".to_string(),
            ));
        }

        let mut payload =
            create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;
        payload["stream"] = Value::Bool(true);

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
        self.supports_streaming
    }
}
