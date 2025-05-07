use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

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
pub const _OPEN_AI_KNOWN_MODELS: &[&str] = &["gpt-4o", "gpt-4.1", "o1", "o3", "o4-mini"];

#[derive(Debug)]
pub struct OpenAiProvider {
    client: Client,
    host: String,
    base_path: String,
    api_key: String,
    organization: Option<String>,
    project: Option<String>,
    model: ModelConfig,
    custom_headers: Option<HashMap<String, String>>,
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OPEN_AI_DEFAULT_MODEL.to_string());
        OpenAiProvider::from_env(model).expect("Failed to initialize OpenAI provider")
    }
}

impl OpenAiProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let api_key: String = get_env("OPENAI_API_KEY")?;
        let host: String =
            get_env("OPENAI_HOST").unwrap_or_else(|_| "https://api.openai.com".to_string());
        let base_path: String =
            get_env("OPENAI_BASE_PATH").unwrap_or_else(|_| "v1/chat/completions".to_string());
        let organization: Option<String> = get_env("OPENAI_ORGANIZATION").ok();
        let project: Option<String> = get_env("OPENAI_PROJECT").ok();
        let custom_headers: Option<HashMap<String, String>> = get_env("OPENAI_CUSTOM_HEADERS")
            .or_else(|_| get_env("OPENAI_CUSTOM_HEADERS"))
            .ok()
            .map(parse_custom_headers);
        // parse get_env("OPENAI_TIMEOUT") to u64 or set default to 600
        let timeout_secs = get_env("OPENAI_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(600);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            client,
            host,
            base_path,
            api_key,
            organization,
            project,
            model,
            custom_headers,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = url::Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join(&self.base_path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let mut request = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key));

        // Add organization header if present
        if let Some(org) = &self.organization {
            request = request.header("OpenAI-Organization", org);
        }

        // Add project header if present
        if let Some(project) = &self.project {
            request = request.header("OpenAI-Project", project);
        }

        if let Some(custom_headers) = &self.custom_headers {
            for (key, value) in custom_headers {
                request = request.header(key, value);
            }
        }

        let response = request.json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
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
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;

        // Make request
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

fn parse_custom_headers(s: String) -> HashMap<String, String> {
    s.split(',')
        .filter_map(|header| {
            let mut parts = header.splitn(2, '=');
            let key = parts.next().map(|s| s.trim().to_string())?;
            let value = parts.next().map(|s| s.trim().to_string())?;
            Some((key, value))
        })
        .collect()
}
