use super::base::{Provider, ProviderUsage, Usage};
use super::configs::ModelConfig;
use super::utils::{get_model, handle_response};
use crate::message::Message;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::tool::Tool;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub const OLLAMA_HOST: &str = "http://localhost:11434";
pub const OLLAMA_MODEL: &str = "qwen2.5";

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    model: ModelConfig,
}

impl OllamaProvider {
    pub fn from_env() -> Result<Self> {
        // Although we don't need host to be stored secretly, we use the keyring to make
        // it easier to coordinate with configuration. We could consider a non secret storage tool
        // elsewhere in the future
        let host = crate::key_manager::get_keyring_secret("OLLAMA_HOST", Default::default())
            .unwrap_or_else(|_| OLLAMA_HOST.to_string());
        let model_name = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| OLLAMA_MODEL.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            model: ModelConfig::new(model_name),
        })
    }

    async fn post(&self, payload: Value) -> Result<Value> {
        let url = format!("{}/v1/chat/completions", self.host.trim_end_matches('/'));

        let response = self.client.post(&url).json(&payload).send().await?;

        handle_response(payload, response).await
    }
}

#[async_trait]
impl Provider for OllamaProvider {
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
        let usage = self.get_usage(&response)?;
        let model = get_model(&response);
        super::utils::emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }

    fn get_usage(&self, data: &Value) -> Result<Usage> {
        get_usage(data)
    }
}
