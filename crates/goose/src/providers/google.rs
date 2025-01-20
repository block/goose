use crate::message::Message;
use crate::providers::base::{Provider, ProviderUsage, Usage};
use crate::providers::configs::ModelConfig;
use crate::providers::formats::google::{create_request, get_usage, response_to_message};
use crate::providers::utils::{emit_debug_trace, handle_response, unescape_json_values};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::tool::Tool;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub const GOOGLE_API_HOST: &str = "https://generativelanguage.googleapis.com";
pub const GOOGLE_DEFAULT_MODEL: &str = "gemini-2.0-flash-exp";

#[derive(Debug, serde::Serialize)]
pub struct GoogleProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl GoogleProvider {
    pub fn from_env() -> Result<Self> {
        let api_key = crate::key_manager::get_keyring_secret("GOOGLE_API_KEY", Default::default())?;
        let host = std::env::var("GOOGLE_HOST").unwrap_or_else(|_| GOOGLE_API_HOST.to_string());
        let model_name =
            std::env::var("GOOGLE_MODEL").unwrap_or_else(|_| GOOGLE_DEFAULT_MODEL.to_string());

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
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.host.trim_end_matches('/'),
            self.model.model_name,
            self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("CONTENT_TYPE", "application/json")
            .json(&payload)
            .send()
            .await?;

        handle_response(payload, response).await
    }
}

#[async_trait]
impl Provider for GoogleProvider {
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
        let payload = create_request(&self.model, system, messages, tools)?;

        // Make request
        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = response_to_message(unescape_json_values(&response))?;
        let usage = self.get_usage(&response)?;
        let model = match response.get("modelVersion") {
            Some(model_version) => model_version.as_str().unwrap_or_default().to_string(),
            None => self.model.model_name.clone(),
        };
        emit_debug_trace(self, &payload, &response, &usage);
        let provider_usage = ProviderUsage::new(model, usage);
        Ok((message, provider_usage))
    }

    fn get_usage(&self, data: &Value) -> Result<Usage> {
        get_usage(data)
    }
}
