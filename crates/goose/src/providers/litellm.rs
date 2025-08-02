use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

use super::base::{ConfigKey, ModelInfo, Provider, ProviderMetadata, ProviderUsage};
use super::embedding::EmbeddingCapable;
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::impl_provider_default;
use crate::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

pub const LITELLM_DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const LITELLM_DOC_URL: &str = "https://docs.litellm.ai/docs/";

#[derive(Debug, serde::Serialize)]
pub struct LiteLLMProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    base_path: String,
    api_key: String,
    model: ModelConfig,
    custom_headers: Option<HashMap<String, String>>,
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
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            client,
            host,
            base_path,
            api_key,
            model,
            custom_headers,
        })
    }

    fn add_headers(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(custom_headers) = &self.custom_headers {
            for (key, value) in custom_headers {
                request = request.header(key, value);
            }
        }

        request
    }

    async fn fetch_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let models_url = format!("{}/model/info", self.host);

        let mut req = self
            .client
            .get(&models_url)
            .header("Authorization", format!("Bearer {}", self.api_key));

        req = self.add_headers(req);

        let response = req
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to fetch models: {}", e)))?;

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
        let base_url = Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join(&self.base_path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let request = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key));

        let request = self.add_headers(request);

        let response = request.json(payload).send().await?;

        handle_response_openai_compat(response).await
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

        let response = self.with_retry(|| async {
            let payload_clone = payload.clone();
            self.post(&payload_clone).await
        }).await?;

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
        let endpoint = format!("{}/v1/embeddings", self.host);

        let embedding_model = std::env::var("GOOSE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        let payload = json!({
            "input": texts,
            "model": embedding_model,
            "encoding_format": "float"
        });

        let mut req = self
            .client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload);

        req = self.add_headers(req);

        let response = req.send().await?;
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
