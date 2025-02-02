use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;

pub const KLUSTER_DEFAULT_MODEL: &str = "deepseek-ai/DeepSeek-R1";
pub const KLUSTER_KNOWN_MODELS: &[&str] = &[
    "deepseek-ai/DeepSeek-R1",
    "klusterai/Meta-Llama-3.1-405B-Instruct-Turbo",
];

pub const KLUSTER_DOC_URL: &str = "https://docs.kluster.ai/";

#[derive(Debug, serde::Serialize)]
pub struct KlusterProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl Default for KlusterProvider {
    fn default() -> Self {
        let model = ModelConfig::new(KlusterProvider::metadata().default_model);
        KlusterProvider::from_env(model).expect("Failed to initialize Kluster provider")
    }
}

impl KlusterProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("KLUSTER_API_KEY")?;
        let host: String = config
            .get("KLUSTER_HOST")
            .unwrap_or_else(|_| "https://api.kluster.ai/v1".to_string());
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
impl Provider for KlusterProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "kluster",
            "Kluster",
            "Kluster models",
            KLUSTER_DEFAULT_MODEL,
            KLUSTER_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            KLUSTER_DOC_URL,
            vec![
                ConfigKey::new("KLUSTER_API_KEY", true, true, None),
                ConfigKey::new(
                    "KLUSTER_HOST",
                    false,
                    false,
                    Some("https://api.kluster.ai/v1"),
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
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;

        // Make request
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
        emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
