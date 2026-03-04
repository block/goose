use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use std::io;
use tokio::pin;
use tokio_util::io::StreamReader;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, ModelInfo, Provider, ProviderDef, ProviderMetadata};
use super::errors::ProviderError;
use super::formats::anthropic::create_request;
use super::formats::anthropic::response_to_streaming_message;
use super::openai_compatible::handle_status_openai_compat;
use super::retry::ProviderRetry;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::utils::RequestLog;
use futures::future::BoxFuture;
use rmcp::model::Tool;
use serde_json::Value;

const MINIMAX_PROVIDER_NAME: &str = "minimax";
pub const MINIMAX_API_HOST: &str = "https://api.minimax.io/anthropic";
pub const MINIMAX_DEFAULT_MODEL: &str = "MiniMax-M2.5";
const MINIMAX_DEFAULT_FAST_MODEL: &str = "MiniMax-M2.5-highspeed";
const MINIMAX_KNOWN_MODELS: &[(&str, usize)] = &[
    ("MiniMax-M2.5", 204_800),
    ("MiniMax-M2.5-highspeed", 204_800),
];

const MINIMAX_DOC_URL: &str = "https://platform.minimax.io/docs/guides/models-intro";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

#[derive(serde::Serialize)]
pub struct MiniMaxProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
}

impl MiniMaxProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let model = model.with_fast(MINIMAX_DEFAULT_FAST_MODEL, MINIMAX_PROVIDER_NAME)?;

        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("MINIMAX_API_KEY")?;
        let host: String = config
            .get_param("MINIMAX_HOST")
            .unwrap_or_else(|_| MINIMAX_API_HOST.to_string());

        let auth = AuthMethod::ApiKey {
            header_name: "x-api-key".to_string(),
            key: api_key,
        };

        let api_client =
            ApiClient::new(host, auth)?.with_header("anthropic-version", ANTHROPIC_API_VERSION)?;

        Ok(Self { api_client, model })
    }
}

impl ProviderDef for MiniMaxProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        let models = MINIMAX_KNOWN_MODELS
            .iter()
            .map(|(name, limit)| ModelInfo::new(*name, *limit))
            .collect();

        ProviderMetadata::with_models(
            MINIMAX_PROVIDER_NAME,
            "MiniMax",
            "MiniMax AI models with long context support via Anthropic-compatible API",
            MINIMAX_DEFAULT_MODEL,
            models,
            MINIMAX_DOC_URL,
            vec![
                ConfigKey::new("MINIMAX_API_KEY", true, true, None, true),
                ConfigKey::new("MINIMAX_HOST", false, false, Some(MINIMAX_API_HOST), false),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for MiniMaxProvider {
    fn get_name(&self) -> &str {
        MINIMAX_PROVIDER_NAME
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn fetch_supported_models(
        &self,
    ) -> Result<Vec<String>, super::errors::ProviderError> {
        Ok(MINIMAX_KNOWN_MODELS
            .iter()
            .map(|(name, _)| name.to_string())
            .collect())
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_request(model_config, system, messages, tools)?;
        payload
            .as_object_mut()
            .unwrap()
            .insert("stream".to_string(), Value::Bool(true));

        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let request = self.api_client.request(Some(session_id), "v1/messages");
                let resp = request.response_post(&payload).await?;
                handle_status_openai_compat(resp).await
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
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                yield (message, usage);
            }
        }))
    }
}
