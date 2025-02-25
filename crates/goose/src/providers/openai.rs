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

pub const OPEN_AI_DEFAULT_MODEL: &str = "gpt-4o";
pub const OPEN_AI_KNOWN_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4-turbo",
    "gpt-3.5-turbo",
    "o1",
];

pub const OPEN_AI_DOC_URL: &str = "https://platform.openai.com/docs/models";

#[derive(Debug, serde::Serialize)]
pub struct OpenAiProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    base_path: String,
    api_key: String,
    organization: Option<String>,
    project: Option<String>,
    model: ModelConfig,
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OpenAiProvider::metadata().default_model);
        OpenAiProvider::from_env(model).expect("Failed to initialize OpenAI provider")
    }
}

impl OpenAiProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("OPENAI_API_KEY")?;
        let host: String = config
            .get("OPENAI_HOST")
            .unwrap_or_else(|_| "https://api.openai.com".to_string());
        let base_path: String = config
            .get("OPENAI_BASE_PATH")
            .unwrap_or_else(|_| "v1/chat/completions".to_string());
        let organization: Option<String> = config.get("OPENAI_ORGANIZATION").ok();
        let project: Option<String> = config.get("OPENAI_PROJECT").ok();
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            base_path,
            api_key,
            organization,
            project,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = url::Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join(&self.base_path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let mut request = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key));

        // Add organization header if present
        if let Some(org) = &self.organization {
            request = request.header("OpenAI-Organization", org);
        }

        // Add project header if present
        if let Some(project) = &self.project {
            request = request.header("OpenAI-Project", project);
        }

        let response = request.json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "openai",
            "OpenAI",
            "GPT-4 and other OpenAI models, including OpenAI compatible ones",
            OPEN_AI_DEFAULT_MODEL,
            OPEN_AI_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            OPEN_AI_DOC_URL,
            vec![
                ConfigKey::new("OPENAI_API_KEY", true, true, None),
                ConfigKey::new("OPENAI_HOST", true, false, Some("https://api.openai.com")),
                ConfigKey::new("OPENAI_BASE_PATH", true, false, Some("v1/chat/completions")),
                ConfigKey::new("OPENAI_ORGANIZATION", false, false, None),
                ConfigKey::new("OPENAI_PROJECT", false, false, None),
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
