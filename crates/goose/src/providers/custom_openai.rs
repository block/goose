use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{ImageFormat, handle_response_openai_compat};
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;

pub const CUSTOM_OPENAI_DEFAULT_MODEL: &str = "gpt-4";

#[derive(Debug, serde::Serialize)]
pub struct CustomOpenAiProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl Default for CustomOpenAiProvider {
    fn default() -> Self {
        let model = ModelConfig::new(CustomOpenAiProvider::metadata().default_model);
        CustomOpenAiProvider::from_env(model).expect("Failed to initialize Custom OpenAI provider")
    }
}

impl CustomOpenAiProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("CUSTOM_OPENAI_API_KEY")?;
        let host: String = config.get("CUSTOM_OPENAI_HOST")?;

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            api_key,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = url::Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for CustomOpenAiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "custom_openai",
            "Custom OpenAI",
            "OpenAI-compatible API with custom host",
            CUSTOM_OPENAI_DEFAULT_MODEL,
            vec![CUSTOM_OPENAI_DEFAULT_MODEL.to_string()],
            "",
            vec![
                ConfigKey::new("CUSTOM_OPENAI_API_KEY", true, true, None),
                ConfigKey::new("CUSTOM_OPENAI_HOST", true, false, Some("")),
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
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;

        let response = self.post(payload).await?;
        let message = response_to_message(response.clone())?;
        let usage = get_usage(&response)?;

        Ok((message, ProviderUsage::new(self.model.model_name.clone(), usage)))
    }
}
