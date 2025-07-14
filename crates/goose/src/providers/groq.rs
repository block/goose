use super::errors::ProviderError;
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use crate::providers::provider_common::{
    build_endpoint_url, get_shared_client, retry_with_backoff, AuthType, HeaderBuilder,
    ProviderConfigBuilder, RetryConfig,
};
use crate::providers::utils::{get_model, handle_response_openai_compat};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::Tool;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;

pub const GROQ_API_HOST: &str = "https://api.groq.com";
pub const GROQ_DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";
pub const GROQ_KNOWN_MODELS: &[&str] = &["gemma2-9b-it", "llama-3.3-70b-versatile"];

pub const GROQ_DOC_URL: &str = "https://console.groq.com/docs/models";

#[derive(serde::Serialize)]
pub struct GroqProvider {
    #[serde(skip)]
    client: Arc<Client>,
    host: String,
    api_key: String,
    model: ModelConfig,
    #[serde(skip)]
    retry_config: RetryConfig,
}

impl Default for GroqProvider {
    fn default() -> Self {
        let model = ModelConfig::new(GroqProvider::metadata().default_model);
        GroqProvider::from_env(model).expect("Failed to initialize Groq provider")
    }
}

impl GroqProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let config_builder = ProviderConfigBuilder::new(config, "GROQ");

        let api_key = config_builder.get_api_key()?;
        let host = config_builder.get_host(GROQ_API_HOST);

        // Use shared client for better connection pooling
        let client = get_shared_client();

        // Configure retry settings
        let retry_config = RetryConfig::default();

        Ok(Self {
            client,
            host,
            api_key,
            model,
            retry_config,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let url = build_endpoint_url(&self.host, "openai/v1/chat/completions")?;

        // Build headers using the new HeaderBuilder
        let headers = HeaderBuilder::new(self.api_key.clone(), AuthType::Bearer).build();

        // Use retry logic for resilience
        retry_with_backoff(&self.retry_config, || async {
            let response = self
                .client
                .post(url.clone())
                .headers(headers.clone())
                .json(&payload)
                .send()
                .await?;

            // Use the common response handler
            handle_response_openai_compat(response).await
        })
        .await
    }
}

#[async_trait]
impl Provider for GroqProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "groq",
            "Groq",
            "Fast inference with Groq hardware",
            GROQ_DEFAULT_MODEL,
            GROQ_KNOWN_MODELS.to_vec(),
            GROQ_DOC_URL,
            vec![
                ConfigKey::new("GROQ_API_KEY", true, true, None),
                ConfigKey::new("GROQ_HOST", false, false, Some(GROQ_API_HOST)),
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
    ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
        let payload = create_request(
            &self.model,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
        )?;

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
        super::utils::emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }

    /// Fetch supported models from Groq; returns Err on failure, Ok(None) if no models found
    async fn fetch_supported_models_async(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // Construct the Groq models endpoint
        let url = build_endpoint_url(&self.host, "openai/v1/models")?;

        // Build headers using HeaderBuilder
        let headers = HeaderBuilder::new(self.api_key.clone(), AuthType::Bearer)
            .add_custom_header("Content-Type".to_string(), "application/json".to_string())
            .build();

        // Send request
        let response = self.client.get(url).headers(headers).send().await?;
        let status = response.status();
        let payload: serde_json::Value = response.json().await.map_err(|_| {
            ProviderError::RequestFailed("Response body is not valid JSON".to_string())
        })?;

        // Check for error response from API
        if let Some(err_obj) = payload.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::Authentication(msg.to_string()));
        }

        // Extract model names
        if status == StatusCode::OK {
            let data = payload
                .get("data")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    ProviderError::UsageError("Missing or invalid `data` field in response".into())
                })?;

            let mut model_names: Vec<String> = data
                .iter()
                .filter_map(|m| m.get("id").and_then(Value::as_str).map(String::from))
                .collect();
            model_names.sort();
            Ok(Some(model_names))
        } else {
            Err(ProviderError::RequestFailed(format!(
                "Groq API returned error status: {}. Payload: {:?}",
                status, payload
            )))
        }
    }
}
