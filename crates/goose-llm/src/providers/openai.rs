use std::{collections::HashMap, time::Duration}; // Keep Duration for native
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

use super::{
    errors::ProviderError,
    formats::openai::{create_request, get_usage, response_to_message},
    utils::{emit_debug_trace, get_env, get_model, handle_response_openai_compat, ImageFormat},
};
use crate::{
    message::Message,
    model::ModelConfig,
    providers::{Provider, ProviderCompleteResponse, ProviderExtractResponse, Usage},
    types::core::Tool,
};

pub const OPEN_AI_DEFAULT_MODEL: &str = "gpt-4o";

fn default_timeout() -> u64 { 60 }
fn default_base_path() -> String { "v1/chat/completions".to_string() }
fn default_host() -> String { "https://api.openai.com".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiProviderConfig {
    pub api_key: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default = "default_base_path")]
    pub base_path: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub custom_headers: Option<HashMap<String, String>>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

impl OpenAiProviderConfig {
    pub fn new(api_key: String) -> Self { 
        Self {
            api_key,
            host: default_host(),
            organization: None,
            base_path: default_base_path(),
            project: None,
            custom_headers: None,
            timeout: 600,
        }
    }
    pub fn from_env() -> Self { 
        let api_key = get_env("OPENAI_API_KEY").expect("Missing OPENAI_API_KEY");
        Self::new(api_key)
    }
}

#[derive(Debug)]
pub struct OpenAiProvider {
    config: OpenAiProviderConfig,
    model: ModelConfig,
    client: Client,
}

impl OpenAiProvider {
    pub fn from_config(config: OpenAiProviderConfig, model: ModelConfig) -> Result<Self> {
        let client = Client::new(); 
        Ok(Self { config, model, client })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn static_post(/* No arguments here for WASM test */) -> Result<Value, ProviderError> { 
        web_sys::console::log_1(&JsValue::from_str("[OpenAiProvider::static_post WASM_NO_ARGS] Entered."));
        
        let client = reqwest::Client::new(); 
        web_sys::console::log_1(&JsValue::from_str("[OpenAiProvider::static_post WASM_NO_ARGS] Client created. Attempting GET example.com..."));
        
        match client.get("https://example.com").send().await {
            Ok(response) => {
                let status = response.status();
                let text = response.text().await.map_err(|e| ProviderError::RequestFailed(format!("Error getting text from example.com: {}", e)))?;
                let success_msg = format!("[OpenAiProvider::static_post WASM_NO_ARGS] example.com GET success! Status: {}, Body: {:.100}", status, text);
                web_sys::console::log_1(&JsValue::from_str(&success_msg));
                Ok(serde_json::json!({"status": "success_example_com_no_args", "message": success_msg})) 
            }
            Err(e) => {
                let error_msg = format!("[OpenAiProvider::static_post WASM_NO_ARGS] example.com GET error: {}", e);
                web_sys::console::error_1(&JsValue::from_str(&error_msg));
                Err(ProviderError::RequestFailed(error_msg))
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn static_post(client: &Client, config: &OpenAiProviderConfig, payload: Value) -> Result<Value, ProviderError> {
        let base_url = url::Url::parse(&config.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url_to_request = base_url.join(&config.base_path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;
        let mut request_builder = client
            .post(url_to_request.clone())
            .header("Authorization", format!("Bearer {}", config.api_key));
        if let Some(org) = &config.organization {
            request_builder = request_builder.header("OpenAI-Organization", org);
        }
        if let Some(project) = &config.project {
            request_builder = request_builder.header("OpenAI-Project", project);
        }
        if let Some(custom_headers) = &config.custom_headers {
            for (key, value) in custom_headers {
                request_builder = request_builder.header(key, value);
            }
        }
        let response = request_builder.json(&payload).send().await?;
        handle_response_openai_compat(response).await
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        #[cfg(target_arch = "wasm32")]
        {
            Self::static_post().await // Call WASM version (no args from self passed here for this call path from Provider trait)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::static_post(&self.client, &self.config, payload).await
        }
    }
}

impl Default for OpenAiProvider { 
    fn default() -> Self {
        let config = OpenAiProviderConfig::from_env();
        let model = ModelConfig::new(OPEN_AI_DEFAULT_MODEL.to_string());
        OpenAiProvider::from_config(config, model).expect("Failed to initialize OpenAiProvider")
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for OpenAiProvider {
    // #[tracing::instrument(...)] // Still commented out
    async fn complete( &self, system: &str, messages: &[Message], tools: &[Tool] ) -> Result<ProviderCompleteResponse, ProviderError> {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&JsValue::from_str(&format!("[OpenAiProvider::complete WASM] Before create_request. System: {:.30}, Messages: {}, Tools: {}", system, messages.len(), tools.len())));
        
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&JsValue::from_str("[OpenAiProvider::complete WASM] After create_request. Payload generated."));

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
        let model_name_from_response = get_model(&response); 
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok(ProviderCompleteResponse::new(message, model_name_from_response, usage))
    }

    async fn extract( &self, system: &str, messages: &[Message], schema: &Value ) -> Result<ProviderExtractResponse, ProviderError> {
        let mut payload = create_request(&self.model, system, messages, &[], &ImageFormat::OpenAi)?;
        payload.as_object_mut().expect("payload must be an object").insert(
            "response_format".to_string(),
            json!({
                "type": "json_schema",
                "json_schema": { "name": "extraction", "schema": schema, "strict": true }
            }),
        );
        let response = self.post(payload.clone()).await?;
        let msg = &response["choices"][0]["message"];
        let raw = msg.get("content").cloned().ok_or_else(|| {
            ProviderError::ResponseParseError("Missing content in extract response".into())
        })?;
        let data = match raw {
            Value::String(s) => serde_json::from_str(&s)
                .map_err(|e| ProviderError::ResponseParseError(format!("Invalid JSON: {}", e)))?,
            Value::Object(_) | Value::Array(_) => raw,
            other => return Err(ProviderError::ResponseParseError(format!("Unexpected content type: {:?}", other ))),
        };
        let usage = match get_usage(&response) {
            Ok(u) => u,
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage in extract: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model_name_from_response = get_model(&response);
        Ok(ProviderExtractResponse::new(data, model_name_from_response, usage))
    }
}
