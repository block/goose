use super::errors::ProviderError;
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use crate::providers::formats::google::{create_request, get_usage, response_to_message};
use crate::providers::provider_common::{ProviderConfigBuilder, get_shared_client, build_endpoint_url, retry_with_backoff, RetryConfig};
use crate::providers::utils::{
    emit_debug_trace, handle_response_google_compat, unescape_json_values,
};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::tool::Tool;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

pub const GOOGLE_API_HOST: &str = "https://generativelanguage.googleapis.com";
pub const GOOGLE_DEFAULT_MODEL: &str = "gemini-2.5-flash";
pub const GOOGLE_KNOWN_MODELS: &[&str] = &[
    // Gemini 2.5 models (latest generation)
    "gemini-2.5-pro",
    "gemini-2.5-pro-preview-06-05",
    "gemini-2.5-pro-preview-05-06",
    "gemini-2.5-flash",
    "gemini-2.5-flash-preview-05-20",
    "gemini-2.5-flash-lite-preview-06-17",
    "gemini-2.5-flash-preview-native-audio-dialog",
    "gemini-2.5-flash-exp-native-audio-thinking-dialog",
    "gemini-2.5-flash-preview-tts",
    "gemini-2.5-pro-preview-tts",
    // Gemini 2.0 models
    "gemini-2.0-flash",
    "gemini-2.0-flash-exp",
    "gemini-2.0-flash-preview-image-generation",
    "gemini-2.0-flash-lite",
    // Gemini 1.5 models
    "gemini-1.5-flash",
    "gemini-1.5-flash-latest",
    "gemini-1.5-flash-002",
    "gemini-1.5-flash-8b",
    "gemini-1.5-flash-8b-latest",
    "gemini-1.5-pro",
    "gemini-1.5-pro-latest",
    "gemini-1.5-pro-002",
];

pub const GOOGLE_DOC_URL: &str = "https://ai.google.dev/gemini-api/docs/models";

#[derive(Debug, serde::Serialize)]
pub struct GoogleProvider {
    #[serde(skip)]
    client: Arc<Client>,
    host: String,
    api_key: String,
    model: ModelConfig,
    #[serde(skip)]
    retry_config: RetryConfig,
}

impl Default for GoogleProvider {
    fn default() -> Self {
        let model = ModelConfig::new(GoogleProvider::metadata().default_model);
        GoogleProvider::from_env(model).expect("Failed to initialize Google provider")
    }
}

impl GoogleProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let config_builder = ProviderConfigBuilder::new(&config, "GOOGLE");
        
        let api_key = config_builder.get_api_key()?;
        let host = config_builder.get_host(GOOGLE_API_HOST);
        
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
        let path = format!(
            "v1beta/models/{}:generateContent?key={}",
            self.model.model_name, self.api_key
        );
        let url = build_endpoint_url(&self.host, &path)?;
        
        // Use retry logic for resilience
        retry_with_backoff(&self.retry_config, || async {
            let response = self.client
                .post(url.clone())
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?;

            handle_response_google_compat(response).await
        }).await
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "google",
            "Google Gemini",
            "Gemini models from Google AI",
            GOOGLE_DEFAULT_MODEL,
            GOOGLE_KNOWN_MODELS.to_vec(),
            GOOGLE_DOC_URL,
            vec![
                ConfigKey::new("GOOGLE_API_KEY", true, true, None),
                ConfigKey::new("GOOGLE_HOST", false, false, Some(GOOGLE_API_HOST)),
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
        let payload = create_request(&self.model, system, messages, tools)?;

        // Make request
        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = response_to_message(unescape_json_values(&response))?;
        let usage = get_usage(&response)?;
        let model = match response.get("modelVersion") {
            Some(model_version) => model_version.as_str().unwrap_or_default().to_string(),
            None => self.model.model_name.clone(),
        };
        emit_debug_trace(&self.model, &payload, &response, &usage);
        let provider_usage = ProviderUsage::new(model, usage);
        Ok((message, provider_usage))
    }

    /// Fetch supported models from Google Generative Language API; returns Err on failure, Ok(None) if not present
    async fn fetch_supported_models_async(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // List models via the v1beta/models endpoint
        let path = format!("v1beta/models?key={}", self.api_key);
        let url = build_endpoint_url(&self.host, &path)?;
        
        let response = self.client.get(url).send().await?;
        let json: serde_json::Value = response.json().await?;
        
        // If 'models' field missing, return None
        let arr = match json.get("models").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return Ok(None),
        };
        
        let mut models: Vec<String> = arr
            .iter()
            .filter_map(|m| m.get("name").and_then(|v| v.as_str()))
            .map(|name| name.split('/').next_back().unwrap_or(name).to_string())
            .collect();
        models.sort();
        Ok(Some(models))
    }
}
