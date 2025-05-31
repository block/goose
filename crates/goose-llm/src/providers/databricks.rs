use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue; // Added for WASM logging

use super::{
    errors::ProviderError,
    formats::databricks::{create_request, get_usage, response_to_message},
    utils::{get_env, get_model, ImageFormat},
};
use crate::{
    message::Message,
    model::ModelConfig,
    providers::{Provider, ProviderCompleteResponse, ProviderExtractResponse, Usage},
    types::core::Tool,
};

pub const DATABRICKS_DEFAULT_MODEL: &str = "databricks-claude-3-7-sonnet";
// Databricks can passthrough to a wide range of models, we only provide the default
pub const _DATABRICKS_KNOWN_MODELS: &[&str] = &[
    "databricks-meta-llama-3-3-70b-instruct",
    "databricks-claude-3-7-sonnet",
];

fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabricksProviderConfig {
    pub host: String,
    pub token: String,
    #[serde(default)]
    pub image_format: ImageFormat,
    #[serde(default = "default_timeout")]
    pub timeout: u64, // timeout in seconds
}

impl DatabricksProviderConfig {
    pub fn new(host: String, token: String) -> Self {
        Self {
            host,
            token,
            image_format: ImageFormat::OpenAi,
            timeout: default_timeout(),
        }
    }

    pub fn from_env() -> Self {
        let host = get_env("DATABRICKS_HOST").expect("Missing DATABRICKS_HOST");
        let token = get_env("DATABRICKS_TOKEN").expect("Missing DATABRICKS_TOKEN");
        Self::new(host, token)
    }
}

#[derive(Debug)]
pub struct DatabricksProvider {
    config: DatabricksProviderConfig,
    model: ModelConfig,
    client: Client,
}

impl DatabricksProvider {
    pub fn from_env(model: ModelConfig) -> Self {
        let config = DatabricksProviderConfig::from_env();
        DatabricksProvider::from_config(config, model)
            .expect("Failed to initialize DatabricksProvider")
    }
}

impl Default for DatabricksProvider {
    fn default() -> Self {
        let config = DatabricksProviderConfig::from_env();
        let model = ModelConfig::new(DATABRICKS_DEFAULT_MODEL.to_string());
        DatabricksProvider::from_config(config, model)
            .expect("Failed to initialize DatabricksProvider")
    }
}

impl DatabricksProvider {
    pub fn from_config(config: DatabricksProviderConfig, model: ModelConfig) -> Result<Self> {
        let client = Client::new(); 

        Ok(Self {
            config,
            model,
            client,
        })
    }

    pub async fn static_post(client: &Client, config: &DatabricksProviderConfig, model_name: &str, payload: Value) -> Result<Value, ProviderError> {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&JsValue::from_str(&format!("[DatabricksProvider::static_post] Payload: {}", serde_json::to_string(&payload).unwrap_or_else(|_| "<payload serialization error>".to_string()))));
        
        let base_url = Url::parse(&config.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let path = format!("serving-endpoints/{}/invocations", model_name);
        let url_to_request = base_url.join(&path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&JsValue::from_str(&format!("[DatabricksProvider::static_post] Requesting URL: {}", url_to_request.as_str())));

        let auth_header = format!("Bearer {}", &config.token);
        let request_builder = client
            .post(url_to_request.clone())
            .header("Authorization", auth_header)
            .json(&payload);
        
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&JsValue::from_str("[DatabricksProvider::static_post] Sending request..."));

        let response = request_builder.send().await?;
        
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&JsValue::from_str(&format!("[DatabricksProvider::static_post] Received response status: {}", response.status())));

        let status = response.status();
        let response_payload: Option<Value> = response.json().await.ok(); // Changed variable name to avoid conflict

        match status {
            StatusCode::OK => response_payload.ok_or_else(|| {
                ProviderError::RequestFailed("Response body is not valid JSON".to_string())
            }),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(ProviderError::Authentication(format!(
                    "Authentication failed. Status: {}. Response: {:?}",
                    status, response_payload
                )))
            }
            StatusCode::BAD_REQUEST => {
                let payload_str = serde_json::to_string(&response_payload)
                    .unwrap_or_default()
                    .to_lowercase();
                let check_phrases = [
                    "too long",
                    "context length",
                    "context_length_exceeded",
                    "reduce the length",
                    "token count",
                    "exceeds",
                ];
                if check_phrases.iter().any(|c| payload_str.contains(c)) {
                    return Err(ProviderError::ContextLengthExceeded(payload_str));
                }
                let mut error_msg = "Unknown error".to_string();
                if let Some(p) = &response_payload { // Changed variable name
                    error_msg = p
                        .get("message")
                        .and_then(|m| m.as_str())
                        .or_else(|| {
                            p
                                .get("external_model_message")
                                .and_then(|ext| ext.get("message"))
                                .and_then(|m| m.as_str())
                        })
                        .unwrap_or("Unknown error")
                        .to_string();
                }
                tracing::debug!(
                    "Provider request failed with status: {}. Payload: {:?}",
                    status, response_payload
                );
                Err(ProviderError::RequestFailed(format!(
                    "Request failed with status: {}. Message: {}",
                    status, error_msg
                )))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                Err(ProviderError::RateLimitExceeded(format!("{:?}", response_payload)))
            }
            StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
                Err(ProviderError::ServerError(format!("{:?}", response_payload)))
            }
            _ => {
                tracing::debug!(
                    "Provider request failed with status: {}. Payload: {:?}",
                    status, response_payload
                );
                Err(ProviderError::RequestFailed(format!(
                    "Request failed with status: {}",
                    status
                )))
            }
        }
    }

    // Keep the original post method for non-WASM trait implementation if needed
    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        Self::static_post(&self.client, &self.config, &self.model.model_name, payload).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for DatabricksProvider {
    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<ProviderCompleteResponse, ProviderError> {
        let mut payload = create_request(
            &self.model,
            system,
            messages,
            tools,
            &self.config.image_format,
        )?;
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
        super::utils::emit_debug_trace(&self.model, &payload, &response, &usage);

        Ok(ProviderCompleteResponse::new(message, model, usage))
    }

    async fn extract(
        &self,
        system: &str,
        messages: &[Message],
        schema: &Value,
    ) -> Result<ProviderExtractResponse, ProviderError> {
        // 1. Build base payload (no tools)
        let mut payload = create_request(&self.model, system, messages, &[], &ImageFormat::OpenAi)?;

        // 2. Inject strict JSON‐Schema wrapper
        payload
            .as_object_mut()
            .expect("payload must be an object")
            .insert(
                "response_format".to_string(),
                json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": "extraction",
                        "schema": schema,
                        "strict": true
                    }
                }),
            );

        // 3. Call OpenAI
        let response = self.post(payload.clone()).await?;

        // 4. Extract the assistant’s `content` and parse it into JSON
        let msg = &response["choices"][0]["message"];
        let raw = msg.get("content").cloned().ok_or_else(|| {
            ProviderError::ResponseParseError("Missing content in extract response".into())
        })?;
        let data = match raw {
            Value::String(s) => serde_json::from_str(&s)
                .map_err(|e| ProviderError::ResponseParseError(format!("Invalid JSON: {}", e)))?,
            Value::Object(_) | Value::Array(_) => raw,
            other => {
                return Err(ProviderError::ResponseParseError(format!(
                    "Unexpected content type: {:?}",
                    other
                )))
            }
        };

        // 5. Gather usage & model info
        let usage = match get_usage(&response) {
            Ok(u) => u,
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage in extract: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);

        Ok(ProviderExtractResponse::new(data, model, usage))
    }
}
