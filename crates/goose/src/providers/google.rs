use super::api_client::{ApiClient, AuthMethod};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{handle_response_google_compat, unescape_json_values, RequestLog};
use crate::conversation::message::Message;

use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use crate::providers::formats::google::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;
use serde_json::Value;

pub const GOOGLE_API_HOST: &str = "https://generativelanguage.googleapis.com";
pub const GOOGLE_DEFAULT_MODEL: &str = "gemini-2.5-pro";
pub const GOOGLE_DEFAULT_FAST_MODEL: &str = "gemini-2.5-flash";
pub const GOOGLE_KNOWN_MODELS: &[&str] = &[
    // Gemini 3 models
    "gemini-3-pro-preview",
    "gemini-3-pro-image-preview",
    // Gemini 2.5 Pro models
    "gemini-2.5-pro",
    "gemini-2.5-pro-preview-tts",
    // Gemini 2.5 Flash models
    "gemini-2.5-flash",
    "gemini-2.5-flash-preview-09-2025",
    "gemini-2.5-flash-image",
    "gemini-2.5-flash-image-preview",
    "gemini-2.5-flash-native-audio-preview-09-2025",
    "gemini-2.5-flash-preview-tts",
    // Gemini 2.5 Flash-Lite models
    "gemini-2.5-flash-lite",
    "gemini-2.5-flash-lite-preview-09-2025",
    // Gemini 2.0 Flash models
    "gemini-2.0-flash",
    "gemini-2.0-flash-001",
    "gemini-2.0-flash-exp",
    "gemini-2.0-flash-preview-image-generation",
    "gemini-2.0-flash-live-001",
    // Gemini 2.0 Flash-Lite models
    "gemini-2.0-flash-lite",
    "gemini-2.0-flash-lite-001",
];

pub const GOOGLE_DOC_URL: &str = "https://ai.google.dev/gemini-api/docs/models";

#[derive(Debug, serde::Serialize)]
pub struct GoogleProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl GoogleProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let model = model.with_fast(GOOGLE_DEFAULT_FAST_MODEL.to_string());

        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("GOOGLE_API_KEY")?;
        let host: String = config
            .get_param("GOOGLE_HOST")
            .unwrap_or_else(|_| GOOGLE_API_HOST.to_string());

        let auth = AuthMethod::ApiKey {
            header_name: "x-goog-api-key".to_string(),
            key: api_key,
        };

        let api_client =
            ApiClient::new(host, auth)?.with_header("Content-Type", "application/json")?;

        Ok(Self {
            api_client,
            model,
            name: Self::metadata().name,
        })
    }

    async fn post(&self, model_name: &str, payload: &Value) -> Result<Value, ProviderError> {
        let path = format!("v1beta/models/{}:generateContent", model_name);
        let response = self.api_client.response_post(&path, payload).await?;
        handle_response_google_compat(response).await
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

    fn get_name(&self) -> &str {
        &self.name
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
        let payload = create_request(model_config, system, messages, tools)?;
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post(&model_config.model_name, &payload_clone).await
            })
            .await?;

        let message = response_to_message(unescape_json_values(&response))?;
        let usage = get_usage(&response)?;
        let response_model = match response.get("modelVersion") {
            Some(model_version) => model_version.as_str().unwrap_or_default().to_string(),
            None => model_config.model_name.clone(),
        };
        log.write(&response, Some(&usage))?;
        let provider_usage = ProviderUsage::new(response_model, usage);
        Ok((message, provider_usage))
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let response = self.api_client.response_get("v1beta/models").await?;
        let json: serde_json::Value = response.json().await?;
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
