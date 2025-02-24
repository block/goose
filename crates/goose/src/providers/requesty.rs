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

pub const REQUESTY_DEFAULT_MODEL: &str = "gpt-4";
const REQUESTY_BASE_URL: &str = "https://router.requesty.ai/v1";

#[derive(Debug, serde::Serialize)]
pub struct RequestyProvider {
    #[serde(skip)]
    client: Client,
    api_key: String,
    model: ModelConfig,
}

impl Default for RequestyProvider {
    fn default() -> Self {
        let model = ModelConfig::new(RequestyProvider::metadata().default_model);
        RequestyProvider::from_env(model).expect("Failed to initialize Requesty provider")
    }
}

impl RequestyProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("REQUESTY_API_KEY")?;

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            api_key,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let url = format!("{}/chat/completions", REQUESTY_BASE_URL);

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
impl Provider for RequestyProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "requesty",
            "Requesty",
            "Access OpenAI models through Requesty Router",
            REQUESTY_DEFAULT_MODEL,
            vec![REQUESTY_DEFAULT_MODEL.to_string()],
            "",
            vec![ConfigKey::new("REQUESTY_API_KEY", true, true, None)],
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
