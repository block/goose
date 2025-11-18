use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;
use std::time::Duration;
use tokio::pin;
use tokio_util::io::StreamReader;

use super::api_client::{ApiClient, AuthMethod, AuthProvider};
use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::embedding::EmbeddingCapable;
use super::errors::ProviderError;
use super::formats::databricks::{create_request, response_to_message};
use super::oauth;
use super::retry::ProviderRetry;
use super::utils::{
    get_model, handle_response_openai_compat, map_http_error_to_provider_error, ImageFormat,
    RequestLog,
};
use crate::config::ConfigError;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::openai::{get_usage, response_to_streaming_message};
use crate::providers::retry::{
    RetryConfig, DEFAULT_BACKOFF_MULTIPLIER, DEFAULT_INITIAL_RETRY_INTERVAL_MS,
    DEFAULT_MAX_RETRIES, DEFAULT_MAX_RETRY_INTERVAL_MS,
};
use rmcp::model::Tool;
use serde_json::json;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

const DEFAULT_CLIENT_ID: &str = "databricks-cli";
const DEFAULT_REDIRECT_URL: &str = "http://localhost";
const DEFAULT_SCOPES: &[&str] = &["all-apis", "offline_access"];
const DEFAULT_TIMEOUT_SECS: u64 = 600;

pub const DATABRICKS_DEFAULT_MODEL: &str = "databricks-claude-sonnet-4";
const DATABRICKS_DEFAULT_FAST_MODEL: &str = "gemini-2-5-flash";
pub const DATABRICKS_KNOWN_MODELS: &[&str] = &[
    "databricks-claude-sonnet-4-5",
    "databricks-claude-3-7-sonnet",
    "databricks-meta-llama-3-3-70b-instruct",
    "databricks-meta-llama-3-1-405b-instruct",
    "databricks-dbrx-instruct",
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

struct DatabricksAuthProvider {
    auth: DatabricksAuth,
}

#[async_trait]
impl AuthProvider for DatabricksAuthProvider {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        let token = match &self.auth {
            DatabricksAuth::Token(token) => token.clone(),
            DatabricksAuth::OAuth {
                host,
                client_id,
                redirect_url,
                scopes,
            } => oauth::get_oauth_token_async(host, client_id, redirect_url, scopes).await?,
        };
        Ok(("Authorization".to_string(), format!("Bearer {}", token)))
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DatabricksProvider {
    #[serde(skip)]
    api_client: ApiClient,
    auth: DatabricksAuth,
    model: ModelConfig,
    image_format: ImageFormat,
    #[serde(skip)]
    retry_config: RetryConfig,
    #[serde(skip)]
    name: String,
}

impl DatabricksProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

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
        let retry_config = Self::load_retry_config(config);

        let auth = if let Ok(api_key) = config.get_secret("DATABRICKS_TOKEN") {
            DatabricksAuth::token(api_key)
        } else {
            DatabricksAuth::oauth(host.clone())
        };

        let auth_method =
            AuthMethod::Custom(Box::new(DatabricksAuthProvider { auth: auth.clone() }));

        let api_client =
            ApiClient::with_timeout(host, auth_method, Duration::from_secs(DEFAULT_TIMEOUT_SECS))?;

        // Create the provider without the fast model first
        let mut provider = Self {
            api_client,
            auth,
            model: model.clone(),
            image_format: ImageFormat::OpenAi,
            retry_config,
            name: Self::metadata().name,
        };

        // Check if the default fast model exists in the workspace
        let model_with_fast = if let Ok(Some(models)) = provider.fetch_supported_models().await {
            if models.contains(&DATABRICKS_DEFAULT_FAST_MODEL.to_string()) {
                tracing::debug!(
                    "Found {} in Databricks workspace, setting as fast model",
                    DATABRICKS_DEFAULT_FAST_MODEL
                );
                model.with_fast(DATABRICKS_DEFAULT_FAST_MODEL.to_string())
            } else {
                tracing::debug!(
                    "{} not found in Databricks workspace, not setting fast model",
                    DATABRICKS_DEFAULT_FAST_MODEL
                );
                model
            }
        } else {
            tracing::debug!("Could not fetch Databricks models, not setting fast model");
            model
        };

        provider.model = model_with_fast;
        Ok(provider)
    }

    fn load_retry_config(config: &crate::config::Config) -> RetryConfig {
        let max_retries = config
            .get_param("DATABRICKS_MAX_RETRIES")
            .ok()
            .and_then(|v: String| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_RETRIES);

        let initial_interval_ms = config
            .get_param("DATABRICKS_INITIAL_RETRY_INTERVAL_MS")
            .ok()
            .and_then(|v: String| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_INITIAL_RETRY_INTERVAL_MS);

        let backoff_multiplier = config
            .get_param("DATABRICKS_BACKOFF_MULTIPLIER")
            .ok()
            .and_then(|v: String| v.parse::<f64>().ok())
            .unwrap_or(DEFAULT_BACKOFF_MULTIPLIER);

        let max_interval_ms = config
            .get_param("DATABRICKS_MAX_RETRY_INTERVAL_MS")
            .ok()
            .and_then(|v: String| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_MAX_RETRY_INTERVAL_MS);

        RetryConfig {
            max_retries,
            initial_interval_ms,
            backoff_multiplier,
            max_interval_ms,
        }
    }

    pub fn from_params(host: String, api_key: String, model: ModelConfig) -> Result<Self> {
        let auth = DatabricksAuth::token(api_key);
        let auth_method =
            AuthMethod::Custom(Box::new(DatabricksAuthProvider { auth: auth.clone() }));

        let api_client = ApiClient::with_timeout(host, auth_method, Duration::from_secs(600))?;

        Ok(Self {
            api_client,
            auth,
            model,
            image_format: ImageFormat::OpenAi,
            retry_config: RetryConfig::default(),
            name: Self::metadata().name,
        })
    }

    fn get_endpoint_path(&self, model_name: &str, is_embedding: bool) -> String {
        if is_embedding {
            "serving-endpoints/text-embedding-3-small/invocations".to_string()
        } else {
            format!("serving-endpoints/{}/invocations", model_name)
        }
    }

    async fn post(&self, payload: Value, model_name: Option<&str>) -> Result<Value, ProviderError> {
        let is_embedding = payload.get("input").is_some() && payload.get("messages").is_none();
        let model_to_use = model_name.unwrap_or(&self.model.model_name);
        let path = self.get_endpoint_path(model_to_use, is_embedding);

        let response = self.api_client.response_post(&path, &payload).await?;
        handle_response_openai_compat(response).await
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

    fn get_name(&self) -> &str {
        &self.name
    }

    fn retry_config(&self) -> RetryConfig {
        self.retry_config.clone()
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let mut payload =
            create_request(model_config, system, messages, tools, &self.image_format)?;
        payload
            .as_object_mut()
            .expect("payload should have model key")
            .remove("model");

        let mut log = RequestLog::start(&self.model, &payload)?;

        let response = self
            .with_retry(|| self.post(payload.clone(), Some(&model_config.model_name)))
            .await?;

        let message = response_to_message(&response)?;
        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let response_model = get_model(&response);
        log.write(&response, Some(&usage))?;

        Ok((message, ProviderUsage::new(response_model, usage)))
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let model_config = self.model.clone();

        let mut payload =
            create_request(&model_config, system, messages, tools, &self.image_format)?;
        payload
            .as_object_mut()
            .expect("payload should have model key")
            .remove("model");

        payload
            .as_object_mut()
            .unwrap()
            .insert("stream".to_string(), Value::Bool(true));

        let path = self.get_endpoint_path(&model_config.model_name, false);
        let mut log = RequestLog::start(&self.model, &payload)?;
        let response = self
            .with_retry(|| async {
                let resp = self.api_client.response_post(&path, &payload).await?;
                if !resp.status().is_success() {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();

                    // Parse as JSON if possible to pass to map_http_error_to_provider_error
                    let json_payload = serde_json::from_str::<Value>(&error_text).ok();
                    return Err(map_http_error_to_provider_error(status, json_payload));
                }
                Ok(resp)
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                yield (message, usage);
            }
        }))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_embeddings(&self) -> bool {
        true
    }

    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        EmbeddingCapable::create_embeddings(self, texts)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let response = match self
            .api_client
            .response_get("api/2.0/serving-endpoints")
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                tracing::warn!("Failed to fetch Databricks models: {}", e);
                return Ok(None);
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
            return Ok(None);
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
            Ok(None)
        } else {
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

        let request = json!({
            "input": texts,
        });

        let response = self.with_retry(|| self.post(request.clone(), None)).await?;

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
