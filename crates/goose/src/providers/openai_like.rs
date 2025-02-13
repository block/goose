use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::errors::ProviderError::RequestFailed;
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::tool::Tool;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use url::Url;

// TODO - There shouldn't be a default model for this. Should be None().
pub const OPEN_AI_LIKE_DEFAULT_MODEL: &str = "";
pub const OPEN_AI_LIKE_KNOWN_MODELS: &[&str] = &[];

pub const OPEN_AI_LIKE_DOC_URL: &str = "https://platform.openai.com/docs/models";

#[derive(Debug, serde::Serialize)]
pub struct OpenAiLikeProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl Default for OpenAiLikeProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OpenAiLikeProvider::metadata().default_model);
        OpenAiLikeProvider::from_env(model).expect("Failed to initialize OpenAI-like provider")
    }
}

fn parse_host(host: &str) -> Result<Url, ProviderError> {
    let base_url = url::Url::parse(host)
        .map_err(|e| RequestFailed(format!("Invalid base URL: {e}")))?;

    let host =  match base_url.host_str() {
        Some(host) => host,
        None => return Err(RequestFailed(format!("Failed to parse host in {}", host)))
    };

    // Prevent people from blasting their API creds to the internet in plaintext
    if base_url.scheme().eq("http") && !(host == "localhost") {
        return Err(RequestFailed(String::from("http only supported for localhost")))
    }

    Ok(base_url)
}

impl OpenAiLikeProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("OPENAI_LIKE_API_KEY")?;
        let host: String = config
            .get("OPENAI_LIKE_HOST")
            .unwrap_or_else(|_| "http://localhost:8000/v1".to_string());

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
        let base_url = parse_host(&self.host)?;
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
impl Provider for OpenAiLikeProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "openai_like",
            "OpenAI-Like",
            "An OpenAI compatible provider",
            OPEN_AI_LIKE_DEFAULT_MODEL,
            OPEN_AI_LIKE_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            OPEN_AI_LIKE_DOC_URL,
            vec![
                ConfigKey::new("OPENAI_LIKE_API_KEY", true, true, None),
                ConfigKey::new("OPENAI_LIKE_HOST", true, false, Some("http://localhost:8000/v1")),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_validation() {
        assert_eq!(parse_host("http://localhost:8000").unwrap().host_str().unwrap(), "localhost");
        assert_eq!(parse_host("https://somewhere.com").unwrap().host_str().unwrap(), "somewhere.com");
        assert_eq!(parse_host("http://somewhere.com").unwrap_err(), RequestFailed(String::from("http only supported for localhost")));
    }

}
