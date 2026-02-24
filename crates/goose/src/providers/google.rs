use super::api_client::{ApiClient, AuthMethod};
use super::base::MessageStream;
use super::errors::ProviderError;
use super::openai_compatible::handle_status_openai_compat;
use super::retry::ProviderRetry;
use super::utils::RequestLog;
use crate::conversation::message::Message;

use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderDef, ProviderMetadata};
use crate::providers::formats::google::{create_request, response_to_streaming_message};
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::future::BoxFuture;
use futures::TryStreamExt;
use rmcp::model::Tool;
use serde_json::Value;
use std::io;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

const GOOGLE_PROVIDER_NAME: &str = "google";
pub const GOOGLE_API_HOST: &str = "https://generativelanguage.googleapis.com";
pub const GOOGLE_DEFAULT_MODEL: &str = "gemini-2.5-pro";
pub const GOOGLE_DEFAULT_FAST_MODEL: &str = "gemini-2.5-flash";

pub const GOOGLE_DOC_URL: &str = "https://ai.google.dev/gemini-api/docs/models";

#[derive(Debug, serde::Serialize)]
pub struct GoogleProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl GoogleProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let model = model.with_fast(GOOGLE_DEFAULT_FAST_MODEL, GOOGLE_PROVIDER_NAME)?;

        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("GOOGLE_API_KEY")?;
        let host: String = config
            .get_param("GOOGLE_HOST")
            .unwrap_or_else(|_| GOOGLE_API_HOST.to_string());

        let auth = AuthMethod::ApiKey {
            header_name: "x-goog-api-key".to_string(),
            key: api_key,
        };

        let api_client =
            ApiClient::new(host, auth)?.with_header("Content-Type", "application/json")?;

        Ok(Self {
            api_client,
            model,
            name: GOOGLE_PROVIDER_NAME.to_string(),
        })
    }

    async fn post_stream(
        &self,
        session_id: Option<&str>,
        model_name: &str,
        payload: &Value,
    ) -> Result<reqwest::Response, ProviderError> {
        let path = format!("v1beta/models/{}:streamGenerateContent?alt=sse", model_name);
        let response = self
            .api_client
            .response_post(session_id, &path, payload)
            .await?;
        handle_status_openai_compat(response).await
    }
}

impl ProviderDef for GoogleProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::from_canonical(
            GOOGLE_PROVIDER_NAME,
            "Google Gemini",
            "Gemini models from Google AI",
            GOOGLE_DEFAULT_MODEL,
            vec![],
            GOOGLE_DOC_URL,
            vec![
                ConfigKey::new("GOOGLE_API_KEY", true, true, None, true),
                ConfigKey::new("GOOGLE_HOST", false, false, Some(GOOGLE_API_HOST), false),
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
impl Provider for GoogleProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request(None, "v1beta/models")
            .response_get()
            .await?;
        let json: serde_json::Value = response.json().await?;
        let arr = json
            .get("models")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ProviderError::RequestFailed(
                    "Missing 'models' array in Google models response".to_string(),
                )
            })?;
        let mut models: Vec<String> = arr
            .iter()
            .filter_map(|m| m.get("name").and_then(|v| v.as_str()))
            .map(|name| name.split('/').next_back().unwrap_or(name).to_string())
            .collect();
        models.sort();
        Ok(models)
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let payload = create_request(model_config, system, messages, tools)?;
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                self.post_stream(Some(session_id), &model_config.model_name, &payload)
                    .await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new())
                .map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e|
                    ProviderError::RequestFailed(format!("Stream decode error: {}", e))
                )?;
                if message.is_some() || usage.is_some() {
                    log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                }
                yield (message, usage);
            }
        }))
    }
}
