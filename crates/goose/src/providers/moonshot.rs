use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::{pin_mut, StreamExt, TryStreamExt};
use serde_json::Value;
use std::io;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{
    get_model, handle_response_openai_compat, handle_status_openai_compat, RequestLog,
};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::moonshot as moonshot_format;
use crate::providers::formats::openai::{create_request, get_usage};
use rmcp::model::Tool;

pub const MOONSHOT_DEFAULT_MODEL: &str = "kimi-k2.5";
pub const MOONSHOT_KNOWN_MODELS: &[&str] = &["kimi-k2.5", "kimi-k2-thinking"];
pub const MOONSHOT_DOC_URL: &str = "https://platform.moonshot.ai/docs/introduction";

#[derive(serde::Serialize)]
pub struct MoonshotProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl MoonshotProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("MOONSHOT_API_KEY")?;
        let host: String = config
            .get_param("MOONSHOT_HOST")
            .unwrap_or_else(|_| "https://api.moonshot.ai".to_string());

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new(host, auth)?;

        Ok(Self {
            api_client,
            model,
            name: Self::metadata().name,
        })
    }

    async fn post(
        &self,
        session_id: Option<&str>,
        payload: &Value,
    ) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post(session_id, "v1/chat/completions", payload)
            .await?;

        let response_body = handle_response_openai_compat(response)
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse response: {e}")))?;

        if let Some(error_obj) = response_body.get("error") {
            let error_message = error_obj
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Moonshot error");

            let error_code = error_obj.get("code").and_then(|c| c.as_u64()).unwrap_or(0);

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

        Ok(response_body)
    }
}

#[async_trait]
impl Provider for MoonshotProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "moonshot",
            "Moonshot",
            "Moonshot AI Kimi models",
            MOONSHOT_DEFAULT_MODEL,
            MOONSHOT_KNOWN_MODELS.to_vec(),
            MOONSHOT_DOC_URL,
            vec![
                ConfigKey::new("MOONSHOT_API_KEY", true, true, None),
                ConfigKey::new(
                    "MOONSHOT_HOST",
                    false,
                    false,
                    Some("https://api.moonshot.ai"),
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
        skip(self, session_id, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        session_id: Option<&str>,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let mut payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
            false,
        )?;

        moonshot_format::add_reasoning_content_to_request(&mut payload, messages);

        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post(session_id, &payload_clone).await
            })
            .await?;

        let response_model = get_model(&response);
        let message = moonshot_format::response_to_message(&response)?;

        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        log.write(&response, Some(&usage))?;
        Ok((message, ProviderUsage::new(response_model, usage)))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn stream(
        &self,
        session_id: &str,
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
            true,
        )?;

        moonshot_format::add_reasoning_content_to_request(&mut payload, messages);

        let mut log = RequestLog::start(&self.model, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post(Some(session_id), "v1/chat/completions", &payload)
                    .await?;
                handle_status_openai_compat(resp).await
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

            let message_stream = moonshot_format::response_to_streaming_message(framed);
            pin_mut!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage): (Option<Message>, Option<ProviderUsage>) = message.map_err(|e|
                    ProviderError::RequestFailed(format!("Stream decode error: {}", e))
                )?;
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                yield (message, usage);
            }
        }))
    }
}
