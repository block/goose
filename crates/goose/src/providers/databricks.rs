use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::embedding::EmbeddingCapable;
use super::errors::ProviderError;
use super::formats::databricks::{create_request, get_usage, response_to_message};
use super::oauth;
use super::provider_common::{
    build_endpoint_url, get_shared_client, retry_with_backoff, ProviderConfigBuilder, RetryConfig,
};
use super::utils::{emit_debug_trace, get_model, ImageFormat};
use crate::config::ConfigError;
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;
use serde_json::json;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

const DEFAULT_CLIENT_ID: &str = "databricks-cli";
const DEFAULT_REDIRECT_URL: &str = "http://localhost:8020";
// "offline_access" scope is used to request an OAuth 2.0 Refresh Token
// https://openid.net/specs/openid-connect-core-1_0.html#OfflineAccess
const DEFAULT_SCOPES: &[&str] = &["all-apis", "offline_access"];

// Databricks specific retry settings
const DATABRICKS_MAX_RETRIES: u32 = 6;
const DATABRICKS_INITIAL_RETRY_INTERVAL_MS: u64 = 5000;
const DATABRICKS_MAX_RETRY_INTERVAL_MS: u64 = 320_000;

pub const DATABRICKS_DEFAULT_MODEL: &str = "databricks-claude-3-7-sonnet";
// Databricks can passthrough to a wide range of models, we only provide the default
pub const DATABRICKS_KNOWN_MODELS: &[&str] = &[
    "databricks-meta-llama-3-3-70b-instruct",
    "databricks-meta-llama-3-1-405b-instruct",
    "databricks-dbrx-instruct",
    "databricks-mixtral-8x7b-instruct",
];

pub const DATABRICKS_DOC_URL: &str =
    "https://docs.databricks.com/en/generative-ai/external-models/index.html";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabricksAuth {
    Token(String),
    OAuth {
        host: String,
        client_id: String,
        redirect_url: String,
        scopes: Vec<String>,
    },
}

impl DatabricksAuth {
    /// Create a new OAuth configuration with default values
    pub fn oauth(host: String) -> Self {
        Self::OAuth {
            host,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            redirect_url: DEFAULT_REDIRECT_URL.to_string(),
            scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }
    pub fn token(token: String) -> Self {
        Self::Token(token)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DatabricksProvider {
    #[serde(skip)]
    client: Arc<Client>,
    host: String,
    auth: DatabricksAuth,
    model: ModelConfig,
    image_format: ImageFormat,
    #[serde(skip)]
    retry_config: RetryConfig,
}

impl Default for DatabricksProvider {
    fn default() -> Self {
        let model = ModelConfig::new(DatabricksProvider::metadata().default_model);
        DatabricksProvider::from_env(model).expect("Failed to initialize Databricks provider")
    }
}

impl DatabricksProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let config_builder = ProviderConfigBuilder::new(config, "DATABRICKS");

        // For compatibility for now we check both config and secret for databricks host
        // but it is not actually a secret value
        let mut host: Result<String, ConfigError> = config.get_param("DATABRICKS_HOST");
        if host.is_err() {
            host = config.get_secret("DATABRICKS_HOST")
        }

        if host.is_err() {
            return Err(ConfigError::NotFound(
                "Did not find DATABRICKS_HOST in either config file or keyring".to_string(),
            )
            .into());
        }

        let host = host?;

        // Use shared client for better connection pooling
        let client = get_shared_client();

        // Configure retry settings with Databricks' specific requirements
        let retry_config = RetryConfig {
            max_retries: config_builder
                .get_param("MAX_RETRIES", None)
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(DATABRICKS_MAX_RETRIES),
            initial_delay_ms: config_builder
                .get_param("INITIAL_RETRY_INTERVAL_MS", None)
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(DATABRICKS_INITIAL_RETRY_INTERVAL_MS),
            max_delay_ms: config_builder
                .get_param("MAX_RETRY_INTERVAL_MS", None)
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(DATABRICKS_MAX_RETRY_INTERVAL_MS),
            backoff_multiplier: 2.0,
        };

        // If we find a databricks token we prefer that
        if let Ok(api_key) = config.get_secret("DATABRICKS_TOKEN") {
            return Ok(Self {
                client,
                host,
                auth: DatabricksAuth::token(api_key),
                model,
                image_format: ImageFormat::OpenAi,
                retry_config,
            });
        }

        // Otherwise use Oauth flow
        Ok(Self {
            client,
            auth: DatabricksAuth::oauth(host.clone()),
            host,
            model,
            image_format: ImageFormat::OpenAi,
            retry_config,
        })
    }

    /// Create a new DatabricksProvider with the specified host and token
    ///
    /// # Arguments
    ///
    /// * `host` - The Databricks host URL
    /// * `token` - The Databricks API token
    ///
    /// # Returns
    ///
    /// Returns a Result containing the new DatabricksProvider instance
    pub fn from_params(host: String, api_key: String, model: ModelConfig) -> Result<Self> {
        // Use shared client for better connection pooling
        let client = get_shared_client();

        Ok(Self {
            client,
            host,
            auth: DatabricksAuth::token(api_key),
            model,
            image_format: ImageFormat::OpenAi,
            retry_config: RetryConfig::default(),
        })
    }

    async fn ensure_auth_header(&self) -> Result<String> {
        match &self.auth {
            DatabricksAuth::Token(token) => Ok(format!("Bearer {}", token)),
            DatabricksAuth::OAuth {
                host,
                client_id,
                redirect_url,
                scopes,
            } => {
                let token =
                    oauth::get_oauth_token_async(host, client_id, redirect_url, scopes).await?;
                Ok(format!("Bearer {}", token))
            }
        }
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        // Check if this is an embedding request by looking at the payload structure
        let is_embedding = payload.get("input").is_some() && payload.get("messages").is_none();
        let path = if is_embedding {
            // For embeddings, use the embeddings endpoint
            format!("serving-endpoints/{}/invocations", "text-embedding-3-small")
        } else {
            // For chat completions, use the model name in the path
            format!("serving-endpoints/{}/invocations", self.model.model_name)
        };

        let url = build_endpoint_url(&self.host, &path)?;

        // Use retry logic for resilience
        retry_with_backoff(&self.retry_config, || async {
            // Get a fresh auth token for each attempt
            let auth_header = self.ensure_auth_header().await.map_err(|e| {
                tracing::error!("Authentication error: {:?}", e);
                ProviderError::RequestFailed(format!("Failed to get authentication token: {}", e))
            })?;

            let response = self
                .client
                .post(url.clone())
                .header("Authorization", auth_header)
                .json(&payload)
                .send()
                .await?;

            let status = response.status();
            let response_body: Option<Value> = response.json().await.ok();

            match status {
                StatusCode::OK => {
                    response_body.ok_or_else(|| {
                        ProviderError::RequestFailed("Response body is not valid JSON".to_string())
                    })
                }
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    Err(ProviderError::Authentication(format!(
                        "Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                        Status: {}. Response: {:?}",
                        status, response_body
                    )))
                }
                StatusCode::BAD_REQUEST => {
                    // Databricks provides a generic 'error' but also includes 'external_model_message' which is provider specific
                    // We try to extract the error message from the payload and check for phrases that indicate context length exceeded
                    let payload_str = serde_json::to_string(&response_body)
                        .unwrap_or_default()
                        .to_lowercase();
                    let check_phrases = [
                        "too long",
                        "context length",
                        "context_length_exceeded",
                        "reduce the length",
                        "token count",
                        "exceeds",
                        "exceed context limit",
                        "input length",
                        "max_tokens",
                        "decrease input length",
                        "context limit",
                    ];
                    if check_phrases.iter().any(|c| payload_str.contains(c)) {
                        return Err(ProviderError::ContextLengthExceeded(payload_str));
                    }

                    let mut error_msg = "Unknown error".to_string();
                    if let Some(payload) = &response_body {
                        // try to convert message to string, if that fails use external_model_message
                        error_msg = payload
                            .get("message")
                            .and_then(|m| m.as_str())
                            .or_else(|| {
                                payload
                                    .get("external_model_message")
                                    .and_then(|ext| ext.get("message"))
                                    .and_then(|m| m.as_str())
                            })
                            .unwrap_or("Unknown error")
                            .to_string();
                    }

                    tracing::debug!(
                        "{}",
                        format!(
                            "Provider request failed with status: {}. Payload: {:?}",
                            status, response_body
                        )
                    );
                    Err(ProviderError::RequestFailed(format!(
                        "Request failed with status: {}. Message: {}",
                        status, error_msg
                    )))
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    Err(ProviderError::RateLimitExceeded(format!("{:?}", response_body)))
                }
                StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
                    Err(ProviderError::ServerError(format!("{:?}", response_body)))
                }
                _ => {
                    tracing::debug!(
                        "{}",
                        format!(
                            "Provider request failed with status: {}. Payload: {:?}",
                            status, response_body
                        )
                    );
                    Err(ProviderError::RequestFailed(format!(
                        "Request failed with status: {}",
                        status
                    )))
                }
            }
        }).await
    }
}

#[async_trait]
impl Provider for DatabricksProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "databricks",
            "Databricks",
            "Models on Databricks AI Gateway",
            DATABRICKS_DEFAULT_MODEL,
            DATABRICKS_KNOWN_MODELS.to_vec(),
            DATABRICKS_DOC_URL,
            vec![
                ConfigKey::new("DATABRICKS_HOST", true, false, None),
                ConfigKey::new("DATABRICKS_TOKEN", false, true, None),
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
        let mut payload = create_request(&self.model, system, messages, tools, &self.image_format)?;
        // Remove the model key which is part of the url with databricks
        payload
            .as_object_mut()
            .expect("payload should have model key")
            .remove("model");

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
        emit_debug_trace(&self.model, &payload, &response, &usage);

        Ok((message, ProviderUsage::new(model, usage)))
    }

    fn supports_embeddings(&self) -> bool {
        true
    }

    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        EmbeddingCapable::create_embeddings(self, texts)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))
    }

    async fn fetch_supported_models_async(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let url = build_endpoint_url(&self.host, "api/2.0/serving-endpoints")?;

        let auth_header = match self.ensure_auth_header().await {
            Ok(header) => header,
            Err(e) => {
                tracing::warn!("Failed to authorize with Databricks: {}", e);
                return Ok(None); // Return None to fall back to manual input
            }
        };

        let response = match self
            .client
            .get(url)
            .header("Authorization", auth_header)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                tracing::warn!("Failed to fetch Databricks models: {}", e);
                return Ok(None); // Return None to fall back to manual input
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            if let Ok(error_text) = response.text().await {
                tracing::warn!(
                    "Failed to fetch Databricks models: {} - {}",
                    status,
                    error_text
                );
            } else {
                tracing::warn!("Failed to fetch Databricks models: {}", status);
            }
            return Ok(None); // Return None to fall back to manual input
        }

        let json: Value = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to parse Databricks API response: {}", e);
                return Ok(None);
            }
        };

        let endpoints = match json.get("endpoints").and_then(|v| v.as_array()) {
            Some(endpoints) => endpoints,
            None => {
                tracing::warn!(
                    "Unexpected response format from Databricks API: missing 'endpoints' array"
                );
                return Ok(None);
            }
        };

        let models: Vec<String> = endpoints
            .iter()
            .filter_map(|endpoint| {
                endpoint
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|name| name.to_string())
            })
            .collect();

        if models.is_empty() {
            tracing::debug!("No serving endpoints found in Databricks workspace");
            Ok(None)
        } else {
            tracing::debug!(
                "Found {} serving endpoints in Databricks workspace",
                models.len()
            );
            Ok(Some(models))
        }
    }
}

#[async_trait]
impl EmbeddingCapable for DatabricksProvider {
    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Create request in Databricks format for embeddings
        let request = json!({
            "input": texts,
        });

        let response = self.post(request).await?;

        let embeddings = response["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format: missing data array"))?
            .iter()
            .map(|item| {
                item["embedding"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Invalid embedding format"))?
                    .iter()
                    .map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Option<Vec<f32>>>()
                    .ok_or_else(|| anyhow::anyhow!("Invalid embedding values"))
            })
            .collect::<Result<Vec<Vec<f32>>>>()?;

        Ok(embeddings)
    }
}
