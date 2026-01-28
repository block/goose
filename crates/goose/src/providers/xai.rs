use super::api_client::{ApiClient, AuthMethod};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{get_model, handle_response_openai_compat, handle_status_openai_compat};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::{
    ConfigKey, Provider, ProviderMetadata, ProviderUsage, StreamFormat, StreamRequest, Usage,
};
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;
use serde_json::Value;
pub const XAI_API_HOST: &str = "https://api.x.ai/v1";
pub const XAI_DEFAULT_MODEL: &str = "grok-code-fast-1";
pub const XAI_KNOWN_MODELS: &[&str] = &[
    "grok-code-fast-1",
    "grok-4-0709",
    "grok-3",
    "grok-3-fast",
    "grok-3-mini",
    "grok-3-mini-fast",
    "grok-2-vision-1212",
    "grok-2-image-1212",
    "grok-3-latest",
    "grok-3-fast-latest",
    "grok-3-mini-latest",
    "grok-3-mini-fast-latest",
    "grok-2-vision",
    "grok-2-vision-latest",
    "grok-2-image",
    "grok-2-image-latest",
    "grok-2",
    "grok-2-latest",
];

pub const XAI_DOC_URL: &str = "https://docs.x.ai/docs/overview";

#[derive(serde::Serialize)]
pub struct XaiProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    supports_streaming: bool,
    #[serde(skip)]
    name: String,
}

impl XaiProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("XAI_API_KEY")?;
        let host: String = config
            .get_param("XAI_HOST")
            .unwrap_or_else(|_| XAI_API_HOST.to_string());

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new(host, auth)?;

        Ok(Self {
            api_client,
            model,
            supports_streaming: true,
            name: Self::metadata().name,
        })
    }

    async fn post(&self, session_id: Option<&str>, payload: Value) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post(session_id, "chat/completions", &payload)
            .await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for XaiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "xai",
            "xAI",
            "Grok models from xAI, including reasoning and multimodal capabilities",
            XAI_DEFAULT_MODEL,
            XAI_KNOWN_MODELS.to_vec(),
            XAI_DOC_URL,
            vec![
                ConfigKey::new("XAI_API_KEY", true, true, None),
                ConfigKey::new("XAI_HOST", false, false, Some(XAI_API_HOST)),
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
        session_id: Option<&str>,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
            false,
        )?;

        let response = self
            .with_retry(|| self.post(session_id, payload.clone()))
            .await?;

        let message = response_to_message(&response)?;
        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let response_model = get_model(&response);
        Ok((message, ProviderUsage::new(response_model, usage)))
    }

    fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }

    fn build_stream_request(
        &self,
        _session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<StreamRequest, ProviderError> {
        let payload = create_request(
            &self.model,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
            true,
        )?;

        Ok(StreamRequest::new(
            "chat/completions",
            payload,
            StreamFormat::OpenAiCompat,
        ))
    }

    async fn execute_stream_request(
        &self,
        request: &StreamRequest,
    ) -> Result<reqwest::Response, ProviderError> {
        let mut api_request = self.api_client.request(None, &request.url);

        for (key, value) in &request.headers {
            api_request = api_request.header(key.as_str(), value.to_str().unwrap_or(""))?;
        }

        let resp = api_request.response_post(&request.payload).await?;
        handle_status_openai_compat(resp).await
    }
}
