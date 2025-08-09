use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ModelInfo, Provider, ProviderMetadata, ProviderUsage};
use super::embedding::EmbeddingCapable;
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::conversation::message::Message;
use crate::impl_provider_default;
use crate::model::ModelConfig;
use rmcp::model::Tool;

pub const LITELLM_DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const LITELLM_DOC_URL: &str = "https://docs.litellm.ai/docs/";

#[derive(Debug, serde::Serialize)]
pub struct LiteLLMProvider {
    #[serde(skip)]
    api_client: ApiClient,
    base_path: String,
    model: ModelConfig,
}

impl_provider_default!(LiteLLMProvider);

impl LiteLLMProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config
            .get_secret("LITELLM_API_KEY")
            .unwrap_or_else(|_| String::new());
        let host: String = config
            .get_param("LITELLM_HOST")
            .unwrap_or_else(|_| "https://api.litellm.ai".to_string());
        let base_path: String = config
            .get_param("LITELLM_BASE_PATH")
            .unwrap_or_else(|_| "v1/chat/completions".to_string());
        let custom_headers: Option<HashMap<String, String>> = config
            .get_secret("LITELLM_CUSTOM_HEADERS")
            .or_else(|_| config.get_param("LITELLM_CUSTOM_HEADERS"))
            .ok()
            .map(parse_custom_headers);
        let timeout_secs: u64 = config.get_param("LITELLM_TIMEOUT").unwrap_or(600);

        let auth = if api_key.is_empty() {
            AuthMethod::Custom(Box::new(NoAuth))
        } else {
            AuthMethod::BearerToken(api_key)
        };

        let mut api_client =
            ApiClient::with_timeout(host, auth, std::time::Duration::from_secs(timeout_secs))?;

        if let Some(headers) = custom_headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
                let header_value = reqwest::header::HeaderValue::from_str(&value)?;
                header_map.insert(header_name, header_value);
            }
            api_client = api_client.with_headers(header_map)?;
        }

        Ok(Self {
            api_client,
            base_path,
            model,
        })
    }

    async fn fetch_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let response = self.api_client.response_get("model/info").await?;

        if !response.status().is_success() {
            return Err(ProviderError::RequestFailed(format!(
                "Models endpoint returned status: {}",
                response.status()
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse models response: {}", e))
        })?;

        let models_data = response_json["data"].as_array().ok_or_else(|| {
            ProviderError::RequestFailed("Missing data field in models response".to_string())
        })?;

        let mut models = Vec::new();
        for model_data in models_data {
            if let Some(model_name) = model_data["model_name"].as_str() {
                if model_name.contains("/*") {
                    continue;
                }

                let model_info = &model_data["model_info"];
                let context_length =
                    model_info["max_input_tokens"].as_u64().unwrap_or(128000) as usize;
                let supports_cache_control = model_info["supports_prompt_caching"].as_bool();

                let mut model_info_obj = ModelInfo::new(model_name, context_length);
                model_info_obj.supports_cache_control = supports_cache_control;
                models.push(model_info_obj);
            }
        }

        Ok(models)
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post(&self.base_path, payload)
            .await?;
        handle_response_openai_compat(response).await
    }
}

// No authentication provider for LiteLLM when API key is not provided
struct NoAuth;

#[async_trait]
impl super::api_client::AuthProvider for NoAuth {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        // Return a dummy header that won't be used
        Ok(("X-No-Auth".to_string(), "true".to_string()))
    }
}

#[async_trait]
impl Provider for LiteLLMProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "litellm",
            "LiteLLM",
            "LiteLLM proxy supporting multiple models with automatic prompt caching",
            LITELLM_DEFAULT_MODEL,
            vec![],
            LITELLM_DOC_URL,
            vec![
                ConfigKey::new("LITELLM_API_KEY", false, true, None),
                ConfigKey::new("LITELLM_HOST", true, false, Some("http://localhost:4000")),
                ConfigKey::new(
                    "LITELLM_BASE_PATH",
                    true,
                    false,
                    Some("v1/chat/completions"),
                ),
                ConfigKey::new("LITELLM_CUSTOM_HEADERS", false, true, None),
                ConfigKey::new("LITELLM_TIMEOUT", false, false, Some("600")),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(skip_all, name = "provider_complete")]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let mut payload = super::formats::openai::create_request(
            &self.model,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
        )?;

        if self.supports_cache_control() {
            payload = update_request_for_cache_control(&payload);
        }

        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post(&payload_clone).await
            })
            .await?;

        let message = super::formats::openai::response_to_message(&response)?;
        let usage = super::formats::openai::get_usage(&response);
        let model = get_model(&response);
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }

    fn supports_embeddings(&self) -> bool {
        true
    }

    fn supports_cache_control(&self) -> bool {
        if let Ok(models) = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.fetch_models())
        }) {
            if let Some(model_info) = models.iter().find(|m| m.name == self.model.model_name) {
                return model_info.supports_cache_control.unwrap_or(false);
            }
        }

        self.model.model_name.to_lowercase().contains("claude")
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        match self.fetch_models().await {
            Ok(models) => {
                let model_names: Vec<String> = models.into_iter().map(|m| m.name).collect();
                Ok(Some(model_names))
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models from LiteLLM: {}", e);
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl EmbeddingCapable for LiteLLMProvider {
    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        let embedding_model = std::env::var("GOOSE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        let payload = json!({
            "input": texts,
            "model": embedding_model,
            "encoding_format": "float"
        });

        let response = self
            .api_client
            .response_post("v1/embeddings", &payload)
            .await?;
        let response_text = response.text().await?;
        let response_json: Value = serde_json::from_str(&response_text)?;

        let data = response_json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing data field"))?;

        let mut embeddings = Vec::new();
        for item in data {
            let embedding: Vec<f32> = item["embedding"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Missing embedding field"))?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }
}

/// Updates the request payload to include cache control headers for automatic prompt caching
/// Adds ephemeral cache control to the last 2 user messages, system message, and last tool
pub fn update_request_for_cache_control(original_payload: &Value) -> Value {
    let mut payload = original_payload.clone();

    if let Some(messages_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("messages"))
        .and_then(|messages| messages.as_array_mut())
    {
        let mut user_count = 0;
        for message in messages_spec.iter_mut().rev() {
            if message.get("role") == Some(&json!("user")) {
                if let Some(content) = message.get_mut("content") {
                    if let Some(content_str) = content.as_str() {
                        *content = json!([{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]);
                    }
                }
                user_count += 1;
                if user_count >= 2 {
                    break;
                }
            }
        }

        if let Some(system_message) = messages_spec
            .iter_mut()
            .find(|msg| msg.get("role") == Some(&json!("system")))
        {
            if let Some(content) = system_message.get_mut("content") {
                if let Some(content_str) = content.as_str() {
                    *system_message = json!({
                        "role": "system",
                        "content": [{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]
                    });
                }
            }
        }
    }

    if let Some(tools_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("tools"))
        .and_then(|tools| tools.as_array_mut())
    {
        if let Some(last_tool) = tools_spec.last_mut() {
            if let Some(function) = last_tool.get_mut("function") {
                function
                    .as_object_mut()
                    .unwrap()
                    .insert("cache_control".to_string(), json!({ "type": "ephemeral" }));
            }
        }
    }
    payload
}

fn parse_custom_headers(headers_str: String) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in headers_str.lines() {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    headers
}
