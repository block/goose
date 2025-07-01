use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

use super::azureauth::AzureAuth;
use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::provider_common::{AuthType, HeaderBuilder, ProviderConfigBuilder, get_shared_client, retry_with_backoff_and_custom_delay, RetryConfig};
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;

pub const AZURE_DEFAULT_MODEL: &str = "gpt-4o";
pub const AZURE_DOC_URL: &str =
    "https://learn.microsoft.com/en-us/azure/ai-services/openai/concepts/models";
pub const AZURE_DEFAULT_API_VERSION: &str = "2024-10-21";
pub const AZURE_OPENAI_KNOWN_MODELS: &[&str] = &["gpt-4o", "gpt-4o-mini", "gpt-4"];

// Default retry configuration
const DEFAULT_MAX_RETRIES: usize = 5;
const DEFAULT_INITIAL_RETRY_INTERVAL_MS: u64 = 1000; // Start with 1 second
const DEFAULT_MAX_RETRY_INTERVAL_MS: u64 = 32000; // Max 32 seconds
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;

#[derive(Debug)]
pub struct AzureProvider {
    client: Arc<Client>,
    auth: AzureAuth,
    endpoint: String,
    deployment_name: String,
    api_version: String,
    model: ModelConfig,
    retry_config: RetryConfig,
}

impl Serialize for AzureProvider {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AzureProvider", 4)?;
        state.serialize_field("endpoint", &self.endpoint)?;
        state.serialize_field("deployment_name", &self.deployment_name)?;
        state.serialize_field("api_version", &self.api_version)?;
        state.serialize_field("model", &self.model)?;
        state.end()
    }
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
        let config_builder = ProviderConfigBuilder::new(&config, "AZURE_OPENAI");
        
        let endpoint = config_builder.get_param("ENDPOINT", None)
            .ok_or_else(|| anyhow::anyhow!("AZURE_OPENAI_ENDPOINT is required"))?;
        let deployment_name = config_builder.get_param("DEPLOYMENT_NAME", None)
            .ok_or_else(|| anyhow::anyhow!("AZURE_OPENAI_DEPLOYMENT_NAME is required"))?;
        let api_version = config_builder.get_param("API_VERSION", Some(AZURE_DEFAULT_API_VERSION))
            .unwrap_or_else(|| AZURE_DEFAULT_API_VERSION.to_string());

        // Try to get API key first, if not found use Azure credential chain
        let api_key = config.get_secret("AZURE_OPENAI_API_KEY").ok();
        let auth = AzureAuth::new(api_key)?;

        // Use shared client for better connection pooling
        let client = get_shared_client();
        
        // Configure retry settings with Azure's specific requirements
        let retry_config = RetryConfig {
            max_retries: DEFAULT_MAX_RETRIES as u32,
            initial_delay_ms: DEFAULT_INITIAL_RETRY_INTERVAL_MS,
            max_delay_ms: DEFAULT_MAX_RETRY_INTERVAL_MS,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
        };

        Ok(Self {
            client,
            endpoint,
            auth,
            deployment_name,
            api_version,
            model,
            retry_config,
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

        // Use the enhanced retry logic with custom delay extraction for Azure
        retry_with_backoff_and_custom_delay(
            &self.retry_config,
            || async {
                // Get a fresh auth token for each attempt
                let auth_token = self.auth.get_token().await.map_err(|e| {
                    tracing::error!("Authentication error: {:?}", e);
                    ProviderError::RequestFailed(format!("Failed to get authentication token: {}", e))
                })?;

                // Build headers using HeaderBuilder
                let header_builder = match self.auth.credential_type() {
                    super::azureauth::AzureCredentials::ApiKey(_) => {
                        HeaderBuilder::new(auth_token.token_value.clone(), AuthType::Custom("api-key".to_string()))
                    }
                    super::azureauth::AzureCredentials::DefaultCredential => {
                        HeaderBuilder::new(auth_token.token_value.clone(), AuthType::Bearer)
                    }
                };

                let headers = header_builder.build();
                
                let response = self.client
                    .post(base_url.clone())
                    .headers(headers)
                    .json(&payload)
                    .send()
                    .await?;

                handle_response_openai_compat(response).await
            },
            |error| {
                // Extract retry-after delay from Azure error messages
                match error {
                    ProviderError::RateLimitExceeded(msg) => {
                        // Look for "try again in X seconds" pattern
                        if let Some(pos) = msg.to_lowercase().find("try again in ") {
                            let rest = &msg[pos + 13..]; // Skip "try again in "
                            rest.split_whitespace()
                                .next()
                                .and_then(|s| s.parse::<u64>().ok())
                                .map(|secs| secs * 1000) // Convert to milliseconds
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
        ).await
    }
}

#[async_trait]
impl Provider for AzureProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "azure_openai",
            "Azure OpenAI",
            "Models through Azure OpenAI Service (uses Azure credential chain by default)",
            "gpt-4o",
            AZURE_OPENAI_KNOWN_MODELS.to_vec(),
            AZURE_DOC_URL,
            vec![
                ConfigKey::new("AZURE_OPENAI_ENDPOINT", true, false, None),
                ConfigKey::new("AZURE_OPENAI_DEPLOYMENT_NAME", true, false, None),
                ConfigKey::new("AZURE_OPENAI_API_VERSION", true, false, Some("2024-10-21")),
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
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
