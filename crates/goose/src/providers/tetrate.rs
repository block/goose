use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::errors::ProviderError;
use super::openai_compatible::{handle_status_openai_compat, stream_openai_compat};
use super::retry::ProviderRetry;
use super::utils::RequestLog;
use crate::config::signup_tetrate::TETRATE_DEFAULT_MODEL;
use crate::conversation::message::Message;
use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;

use crate::model::ModelConfig;
use crate::providers::formats::openai::create_request;
use rmcp::model::Tool;

const TETRATE_PROVIDER_NAME: &str = "tetrate";
// Tetrate Agent Router Service can run many models, we suggest the default
pub const TETRATE_KNOWN_MODELS: &[&str] = &[
    "claude-opus-4-1",
    "claude-3-7-sonnet-latest",
    "claude-sonnet-4-20250514",
    "gemini-2.5-pro",
    "gemini-2.0-flash",
    "gemini-2.0-flash-lite",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
    "gpt-4.1",
];
pub const TETRATE_DOC_URL: &str = "https://router.tetrate.ai";

#[derive(serde::Serialize)]
pub struct TetrateProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    supports_streaming: bool,
    #[serde(skip)]
    name: String,
}

impl TetrateProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("TETRATE_API_KEY")?;
        // API host for LLM endpoints (/v1/chat/completions, /v1/models)
        let host: String = config
            .get_param("TETRATE_HOST")
            .unwrap_or_else(|_| "https://api.router.tetrate.ai".to_string());

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new(host, auth)?
            .with_header("HTTP-Referer", "https://block.github.io/goose")?
            .with_header("X-Title", "goose")?;

        Ok(Self {
            api_client,
            model,
            supports_streaming: true,
            name: TETRATE_PROVIDER_NAME.to_string(),
        })
    }
}

impl ProviderDef for TetrateProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            TETRATE_PROVIDER_NAME,
            "Tetrate Agent Router Service",
            "Enterprise router for AI models",
            TETRATE_DEFAULT_MODEL,
            TETRATE_KNOWN_MODELS.to_vec(),
            TETRATE_DOC_URL,
            vec![
                ConfigKey::new("TETRATE_API_KEY", true, true, None),
                ConfigKey::new(
                    "TETRATE_HOST",
                    false,
                    false,
                    Some("https://api.router.tetrate.ai"),
                ),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for TetrateProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
            true,
        )?;

        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post(Some(session_id), "v1/chat/completions", &payload)
                    .await?;
                handle_status_openai_compat(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        stream_openai_compat(response, log)
    }

    /// Fetch supported models from Tetrate Agent Router Service API (only models with tool support)
    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        // Use the existing api_client which already has authentication configured
        let response = match self
            .api_client
            .request(None, "v1/models")
            .response_get()
            .await
        {
            Ok(response) => response,
            Err(e) => {
                return Err(ProviderError::ExecutionError(format!(
                    "Failed to fetch models from Tetrate API: {}. Please check your API key and account at {}",
                    e, TETRATE_DOC_URL
                )));
            }
        };

        let json: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::ExecutionError(format!(
                "Failed to parse Tetrate API response: {}. Please check your API key and account at {}",
                e, TETRATE_DOC_URL
            ))
        })?;

        // Check for error in response
        if let Some(err_obj) = json.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::ExecutionError(format!(
                "Tetrate API error: {}. Please check your API key and account at {}",
                msg, TETRATE_DOC_URL
            )));
        }

        // The response format from /v1/models is expected to be OpenAI-compatible
        // It should have a "data" field with an array of model objects
        let data = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::ExecutionError(format!(
                "Tetrate API response missing 'data' field. Please check your API key and account at {}",
                TETRATE_DOC_URL
            ))
        })?;

        let mut models: Vec<String> = data
            .iter()
            .filter_map(|model| {
                let id = model.get("id").and_then(|v| v.as_str())?;
                let supports_computer_use = model
                    .get("supports_computer_use")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if supports_computer_use {
                    Some(id.to_string())
                } else {
                    None
                }
            })
            .collect();

        models.sort();
        Ok(models)
    }
}
