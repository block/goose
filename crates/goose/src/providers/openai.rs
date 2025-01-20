use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::base::{Provider, ProviderUsage, Usage};
use super::configs::ModelConfig;
use super::formats::openai::{
    create_request, get_usage, is_context_length_error, response_to_message,
};
use super::utils::{emit_debug_trace, get_model, handle_response, ImageFormat};
use crate::message::Message;
use mcp_core::tool::Tool;

pub const OPEN_AI_DEFAULT_MODEL: &str = "gpt-4o";

#[derive(Debug, serde::Serialize)]
pub struct OpenAiProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl OpenAiProvider {
    pub fn from_env() -> Result<Self> {
        let api_key = crate::key_manager::get_keyring_secret("OPENAI_API_KEY", Default::default())?;
        let host =
            std::env::var("OPENAI_HOST").unwrap_or_else(|_| "https://api.openai.com".to_string());
        let model_name =
            std::env::var("OPENAI_MODEL").unwrap_or_else(|_| OPEN_AI_DEFAULT_MODEL.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            api_key,
            model: ModelConfig::new(model_name),
        })
    }

    async fn post(&self, payload: Value) -> Result<Value> {
        let url = format!("{}/v1/chat/completions", self.host.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;

        handle_response(payload, response).await
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn get_model_config(&self) -> &ModelConfig {
        &self.model
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
    ) -> Result<(Message, ProviderUsage)> {
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;

        // Make request
        let response = self.post(payload.clone()).await?;

        // Raise specific error if context length is exceeded
        if let Some(error) = response.get("error") {
            if let Some(err) = is_context_length_error(error) {
                return Err(err.into());
            }
            return Err(anyhow!("OpenAI API error: {}", error));
        }

        // Parse response
        let message = response_to_message(response.clone())?;
        let usage = self.get_usage(&response)?;
        let model = get_model(&response);
        emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }

    fn get_usage(&self, data: &Value) -> Result<Usage> {
        get_usage(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_construction() -> Result<()> {
        std::env::set_var("OPENAI_API_KEY", "test-key");
        std::env::set_var("OPENAI_HOST", "https://test.openai.com");
        std::env::set_var("OPENAI_MODEL", "gpt-4o");

        let provider = OpenAiProvider::from_env()?;
        assert_eq!(provider.host, "https://test.openai.com");
        assert_eq!(provider.model.model_name, "gpt-4o");

        Ok(())
    }
}
