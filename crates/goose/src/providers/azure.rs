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

pub const AZURE_DEFAULT_MODEL: &str = "gpt-4o";
pub const AZURE_DOC_URL: &str =
    "https://learn.microsoft.com/en-us/azure/ai-services/openai/concepts/models";
pub const AZURE_DEFAULT_API_VERSION: &str = "2024-10-21";
pub const AZURE_OPENAI_KNOWN_MODELS: &[&str] = &["gpt-4o", "gpt-4o-mini", "gpt-4"];

#[derive(Debug, serde::Serialize)]
pub struct AzureProvider {
    #[serde(skip)]
    client: Client,
    endpoint: String,
    api_key: String,
    deployment_name: String,
    api_version: String,
    model: ModelConfig,
}

impl Default for AzureProvider {
    fn default() -> Self {
        let model = ModelConfig::new(AzureProvider::metadata().default_model);
        AzureProvider::from_env(model).expect("Failed to initialize Azure OpenAI provider")
    }
}

impl AzureProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("AZURE_OPENAI_API_KEY")?;
        let endpoint: String = config.get("AZURE_OPENAI_ENDPOINT")?;
        let deployment_name: String = config.get("AZURE_OPENAI_DEPLOYMENT_NAME")?;
        let api_version: String = config
            .get("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| AZURE_DEFAULT_API_VERSION.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            endpoint,
            api_key,
            deployment_name,
            api_version,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let mut base_url = url::Url::parse(&self.endpoint)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;

        // Get the existing path without trailing slashes
        let existing_path = base_url.path().trim_end_matches('/');
        let new_path = if existing_path.is_empty() {
            format!(
                "/openai/deployments/{}/chat/completions",
                self.deployment_name
            )
        } else {
            format!(
                "{}/openai/deployments/{}/chat/completions",
                existing_path, self.deployment_name
            )
        };

        base_url.set_path(&new_path);
        base_url.set_query(Some(&format!("api-version={}", self.api_version)));

        let response: reqwest::Response = self
            .client
            .post(base_url)
            .header("api-key", &self.api_key)
            .json(&payload)
            .send()
            .await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for AzureProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "azure_openai",
            "Azure OpenAI",
            "Models through Azure OpenAI Service",
            "gpt-4o",
            AZURE_OPENAI_KNOWN_MODELS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            AZURE_DOC_URL,
            vec![
                ConfigKey::new("AZURE_OPENAI_API_KEY", true, true, None),
                ConfigKey::new("AZURE_OPENAI_ENDPOINT", true, false, None),
                ConfigKey::new(
                    "AZURE_OPENAI_DEPLOYMENT_NAME",
                    true,
                    false,
                    Some("Name of your Azure OpenAI deployment"),
                ),
                ConfigKey::new(
                    "AZURE_OPENAI_API_VERSION",
                    false,
                    false,
                    Some("Azure OpenAI API version, default: 2024-10-21"),
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
        let response = self.post(payload.clone()).await?;

        let message = response_to_message(response.clone())?;
        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
