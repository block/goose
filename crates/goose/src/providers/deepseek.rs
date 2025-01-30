use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat};
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use mcp_core::tool::Tool;

pub const DEEPSEEK_DEFAULT_MODEL: &str = "deepseek-chat";

// Deepseek can run two models, the default is recommended
pub const DEEPSEEK_KNOWN_MODELS: &[&str] = &[
    "deepseek-chat",
    "deepseek-reasoner",
];
pub const DEEPSEEK_DOC_URL: &str = "https://api-docs.deepseek.com/";

#[derive(serde::Serialize)]
pub struct DeepseekProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl Default for DeepseekProvider {
    fn default() -> Self {
        let model = ModelConfig::new(DeepseekProvider::metadata().default_model);
        DeepseekProvider::from_env(model).expect("Failed to initialize Deepseek provider")
    }
}

impl DeepseekProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("DEEPSEEK_API_KEY")?;
        let host: String = config
            .get("DEEPSEEK_HOST")
            .unwrap_or_else(|_| "https://api.deepseek.com/".to_string());

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
        let url = format!(
            "{}/chat/completions",
            self.host.trim_end_matches('/')
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/block/goose")
            .header("X-Title", "Goose")
            .json(&payload)
            .send()
            .await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for DeepseekProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "deepseek",
            "Deepseek",
            "Router for many model providers",
            DEEPSEEK_DEFAULT_MODEL,
            DEEPSEEK_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            DEEPSEEK_DOC_URL,
            vec![
                ConfigKey::new("DEEPSEEK_API_KEY", true, true, None),
                ConfigKey::new(
                    "DEEPSEEK_HOST",
                    false,
                    false,
                    Some("https://api.deepseek.com/"),
                ),
            ],
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
        // Create the base payload
        let payload = create_request(&self.model, system, messages, tools, &super::utils::ImageFormat::OpenAi)?;

        // Make request
        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = response_to_message(response.clone())?;
        let usage = get_usage(&response)?;
        let model = get_model(&response);
        emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
