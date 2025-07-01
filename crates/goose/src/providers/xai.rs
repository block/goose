use super::errors::ProviderError;
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use crate::providers::provider_common::{AuthType, HeaderBuilder, ProviderConfigBuilder, get_shared_client, build_endpoint_url, retry_with_backoff, RetryConfig};
use crate::providers::utils::{get_model, handle_response_openai_compat, emit_debug_trace};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::Tool;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

pub const XAI_API_HOST: &str = "https://api.x.ai/v1";
pub const XAI_DEFAULT_MODEL: &str = "grok-3";
pub const XAI_KNOWN_MODELS: &[&str] = &[
    "grok-3",
    "grok-3-fast",
    "grok-3-mini",
    "grok-3-mini-fast",
    "grok-2-vision-1212",
    "grok-2-image-1212",
    "grok-2-1212",
    "grok-3-latest",
    "grok-3-fast-latest",
    "grok-3-mini-latest",
    "grok-3-mini-fast-latest",
    "grok-2-vision",
    "grok-2-vision-latest",
    "grok-2-image",
    "grok-2-image-latest",
    "grok-2",
    "grok-2-latest",
];

pub const XAI_DOC_URL: &str = "https://docs.x.ai/docs/overview";

#[derive(serde::Serialize)]
pub struct XaiProvider {
    #[serde(skip)]
    client: Arc<Client>,
    host: String,
    api_key: String,
    model: ModelConfig,
    #[serde(skip)]
    retry_config: RetryConfig,
}

impl Default for XaiProvider {
    fn default() -> Self {
        let model = ModelConfig::new(XaiProvider::metadata().default_model);
        XaiProvider::from_env(model).expect("Failed to initialize xAI provider")
    }
}

impl XaiProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let config_builder = ProviderConfigBuilder::new(&config, "XAI");
        
        let api_key = config_builder.get_api_key()?;
        let host = config_builder.get_host(XAI_API_HOST);
        
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

    async fn post(&self, payload: Value) -> anyhow::Result<Value, ProviderError> {
        let url = build_endpoint_url(&self.host, "chat/completions")?;
        
        // Build headers using HeaderBuilder
        let headers = HeaderBuilder::new(self.api_key.clone(), AuthType::Bearer).build();
        
        tracing::debug!("xAI API URL: {}", url);
        tracing::debug!("xAI request model: {:?}", self.model.model_name);
        
        // Use retry logic for resilience
        retry_with_backoff(&self.retry_config, || async {
            let response = self.client
                .post(url.clone())
                .headers(headers.clone())
                .json(&payload)
                .send()
                .await?;

            handle_response_openai_compat(response).await
        }).await
    }
}

#[async_trait]
impl Provider for XaiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "xai",
            "xAI",
            "Grok models from xAI, including reasoning and multimodal capabilities",
            XAI_DEFAULT_MODEL,
            XAI_KNOWN_MODELS.to_vec(),
            XAI_DOC_URL,
            vec![
                ConfigKey::new("XAI_API_KEY", true, true, None),
                ConfigKey::new("XAI_HOST", false, false, Some(XAI_API_HOST)),
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
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
    
    /// Fetch supported models from xAI API; returns Err on failure, Ok(None) if no models found
    async fn fetch_supported_models_async(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let url = build_endpoint_url(&self.host, "models")?;
        
        // Build headers using HeaderBuilder
        let headers = HeaderBuilder::new(self.api_key.clone(), AuthType::Bearer).build();
        
        let response = self.client.get(url).headers(headers).send().await?;
        let json: serde_json::Value = response.json().await?;
        
        if let Some(err_obj) = json.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::Authentication(msg.to_string()));
        }
        
        let data = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::UsageError("Missing data field in JSON response".into())
        })?;
        
        let mut models: Vec<String> = data
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        models.sort();
        Ok(Some(models))
    }
}
