use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{get_model, handle_response_openai_compat};
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::tool::Tool;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub const LMSTUDIO_HOST: &str = "localhost";
pub const LMSTUDIO_DEFAULT_PORT: u16 = 1234;
pub const LMSTUDIO_DEFAULT_MODEL: &str = "default"; // This can be customized based on your needs
pub const LMSTUDIO_KNOWN_MODELS: &[&str] = &[LMSTUDIO_DEFAULT_MODEL];
pub const LMSTUDIO_DOC_URL: &str = "https://lmstudio.ai/"; // Update with actual documentation URL

#[derive(serde::Serialize)]
pub struct LmStudioProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    model: ModelConfig,
}

impl Default for LmStudioProvider {
    fn default() -> Self {
        let model = ModelConfig::new(LmStudioProvider::metadata().default_model);
        LmStudioProvider::from_env(model).expect("Failed to initialize LM Studio provider")
    }
}

impl LmStudioProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let host: String = config
            .get("LMSTUDIO_HOST")
            .unwrap_or_else(|_| LMSTUDIO_HOST.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base = if self.host.starts_with("http://") || self.host.starts_with("https://") {
            self.host.clone()
        } else {
            format!("http://{}", self.host)
        };

        let mut base_url = Url::parse(&base)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;

        // Set the default port if missing
        if base_url.port().is_none() {
            base_url.set_port(Some(LMSTUDIO_DEFAULT_PORT)).map_err(|_| {
                ProviderError::RequestFailed("Failed to set default port".to_string())
            })?;
        }

        let url = base_url.join("v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self.client.post(url).json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for LmStudioProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "lmstudio",
            "LM Studio",
            "Local LM Studio models",
            LMSTUDIO_DEFAULT_MODEL,
            LMSTUDIO_KNOWN_MODELS.iter().map(|&s| s.to_string()).collect(),
            LMSTUDIO_DOC_URL,
            vec![ConfigKey::new(
                "LMSTUDIO_HOST",
                true,
                false,
                Some(LMSTUDIO_HOST),
            )],
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
        let payload = create_request(
            &self.model,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
        )?;
        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = response_to_message(response.clone())?;
        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::warn!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        super::utils::emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}