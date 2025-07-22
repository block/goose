use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

use super::azureauth::AzureAuth;
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

// Default retry configuration
const DEFAULT_MAX_RETRIES: usize = 5;
const DEFAULT_INITIAL_RETRY_INTERVAL_MS: u64 = 1000; // Start with 1 second
const DEFAULT_MAX_RETRY_INTERVAL_MS: u64 = 32000; // Max 32 seconds
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;

#[derive(Debug)]
pub struct AzureProvider {
    client: Client,
    auth: AzureAuth,
    endpoint: String,
    deployment_name: String,
    api_version: String,
    model: ModelConfig,
}

impl Serialize for AzureProvider {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AzureProvider", 3)?;
        state.serialize_field("endpoint", &self.endpoint)?;
        state.serialize_field("deployment_name", &self.deployment_name)?;
        state.serialize_field("api_version", &self.api_version)?;
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
        let endpoint: String = config.get_param("AZURE_OPENAI_ENDPOINT")?;
        let deployment_name: String = config.get_param("AZURE_OPENAI_DEPLOYMENT_NAME")?;
        let api_version: String = config
            .get_param("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| AZURE_DEFAULT_API_VERSION.to_string());

        let api_key = config
            .get_secret("AZURE_OPENAI_API_KEY")
            .ok()
            .filter(|key: &String| !key.is_empty());
        let auth = AzureAuth::new(api_key)?;

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            endpoint,
            auth,
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

        let mut attempts = 0;
        let mut last_error = None;
        let mut current_delay = DEFAULT_INITIAL_RETRY_INTERVAL_MS;

        loop {
            // Check if we've exceeded max retries
            if attempts > DEFAULT_MAX_RETRIES {
                let error_msg = format!(
                    "Exceeded maximum retry attempts ({}) for rate limiting",
                    DEFAULT_MAX_RETRIES
                );
                tracing::error!("{}", error_msg);
                return Err(last_error.unwrap_or(ProviderError::RateLimitExceeded(error_msg)));
            }

            // Get a fresh auth token for each attempt
            let auth_token = self.auth.get_token().await.map_err(|e| {
                tracing::error!("Authentication error: {:?}", e);
                ProviderError::RequestFailed(format!("Failed to get authentication token: {}", e))
            })?;

            let mut request_builder = self.client.post(base_url.clone());
            let token_value = auth_token.token_value.clone();

            // Set the correct header based on authentication type
            match self.auth.credential_type() {
                super::azureauth::AzureCredentials::ApiKey(_) => {
                    request_builder = request_builder.header("api-key", token_value.clone());
                }
                super::azureauth::AzureCredentials::DefaultCredential => {
                    request_builder = request_builder
                        .header("Authorization", format!("Bearer {}", token_value.clone()));
                }
            }

            let response_result = request_builder.json(&payload).send().await;

            match response_result {
                Ok(response) => match handle_response_openai_compat(response).await {
                    Ok(result) => {
                        return Ok(result);
                    }
                    Err(ProviderError::RateLimitExceeded(msg)) => {
                        attempts += 1;
                        last_error = Some(ProviderError::RateLimitExceeded(msg.clone()));

                        let retry_after =
                            if let Some(secs) = msg.to_lowercase().find("try again in ") {
                                msg[secs..]
                                    .split_whitespace()
                                    .nth(3)
                                    .and_then(|s| s.parse::<u64>().ok())
                                    .unwrap_or(0)
                            } else {
                                0
                            };

                        let delay = if retry_after > 0 {
                            Duration::from_secs(retry_after)
                        } else {
                            let delay = current_delay.min(DEFAULT_MAX_RETRY_INTERVAL_MS);
                            current_delay =
                                (current_delay as f64 * DEFAULT_BACKOFF_MULTIPLIER) as u64;
                            Duration::from_millis(delay)
                        };

                        sleep(delay).await;
                        continue;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Error response from Azure OpenAI (attempt {}): {:?}",
                            attempts + 1,
                            e
                        );
                        return Err(e);
                    }
                },
                Err(e) => {
                    tracing::error!(
                        "Request failed (attempt {}): {:?}\nIs timeout: {}\nIs connect: {}\nIs request: {}",
                        attempts + 1,
                        e,
                        e.is_timeout(),
                        e.is_connect(),
                        e.is_request(),
                    );

                    // For timeout errors, we should retry
                    if e.is_timeout() {
                        attempts += 1;
                        let delay = current_delay.min(DEFAULT_MAX_RETRY_INTERVAL_MS);
                        current_delay = (current_delay as f64 * DEFAULT_BACKOFF_MULTIPLIER) as u64;
                        sleep(Duration::from_millis(delay)).await;
                        continue;
                    }

                    return Err(ProviderError::RequestFailed(format!(
                        "Request failed: {}",
                        e
                    )));
                }
            }
        }
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
                ConfigKey::new("AZURE_OPENAI_API_KEY", true, true, Some("")),
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
        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let model = get_model(&response);
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
