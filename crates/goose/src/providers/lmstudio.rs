use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::errors::ProviderError;
use super::openai_compatible::handle_status_openai_compat;
use super::retry::ProviderRetry;
use super::utils::{ImageFormat, RequestLog};
use crate::config::declarative_providers::DeclarativeProviderConfig;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use rmcp::model::Tool;
use std::time::Duration;
use url::Url;

const LMSTUDIO_PROVIDER_NAME: &str = "lmstudio";
pub const LMSTUDIO_HOST: &str = "http://localhost:1234/v1";
pub const LMSTUDIO_TIMEOUT: u64 = 600;
pub const LMSTUDIO_DEFAULT_MODEL: &str = "llama-3.2-3b-instruct";
pub const LMSTUDIO_KNOWN_MODELS: &[&str] = &[LMSTUDIO_DEFAULT_MODEL];
pub const LMSTUDIO_DOC_URL: &str = "https://lmstudio.ai/docs/developer/rest";

#[derive(serde::Serialize)]
pub struct LmStudioProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    supports_streaming: bool,
    name: String,
}

impl LmStudioProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let host: String = config
            .get_param("LMSTUDIO_HOST")
            .unwrap_or_else(|_| LMSTUDIO_HOST.to_string());

        let timeout: Duration = Duration::from_secs(
            config
                .get_param("LMSTUDIO_TIMEOUT")
                .unwrap_or(LMSTUDIO_TIMEOUT),
        );

        let base = if host.starts_with("http://") || host.starts_with("https://") {
            host.clone()
        } else {
            format!("http://{}", host)
        };

        let base_url = Url::parse(&base).map_err(|e| anyhow::anyhow!("Invalid base URL: {e}"))?;

        let api_client =
            ApiClient::with_timeout(base_url.to_string(), AuthMethod::NoAuth, timeout)?;

        Ok(Self {
            api_client,
            model,
            supports_streaming: true,
            name: LMSTUDIO_PROVIDER_NAME.to_string(),
        })
    }

    pub fn from_custom_config(
        model: ModelConfig,
        config: DeclarativeProviderConfig,
    ) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(LMSTUDIO_TIMEOUT));

        let base =
            if config.base_url.starts_with("http://") || config.base_url.starts_with("https://") {
                config.base_url.clone()
            } else {
                format!("http://{}", config.base_url)
            };

        let base_url = Url::parse(&base)
            .map_err(|e| anyhow::anyhow!("Invalid base URL '{}': {}", config.base_url, e))?;

        let mut api_client =
            ApiClient::with_timeout(base_url.to_string(), AuthMethod::NoAuth, timeout)?;

        if let Some(headers) = &config.headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
                let header_value = reqwest::header::HeaderValue::from_str(value)?;
                header_map.insert(header_name, header_value);
            }
            api_client = api_client.with_headers(header_map)?;
        }

        let supports_streaming = config.supports_streaming.unwrap_or(true);

        Ok(Self {
            api_client,
            model,
            supports_streaming,
            name: config.name.clone(),
        })
    }
}

impl ProviderDef for LmStudioProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            LMSTUDIO_PROVIDER_NAME,
            "LM Studio",
            "Run local models with LM Studio",
            LMSTUDIO_DEFAULT_MODEL,
            LMSTUDIO_KNOWN_MODELS.to_vec(),
            LMSTUDIO_DOC_URL,
            vec![
                ConfigKey::new("LMSTUDIO_HOST", true, false, Some(LMSTUDIO_HOST), true),
                ConfigKey::new(
                    "LMSTUDIO_TIMEOUT",
                    false,
                    false,
                    Some(&(LMSTUDIO_TIMEOUT.to_string())),
                    false,
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
impl Provider for LmStudioProvider {
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
        let payload = super::formats::openai::create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            self.supports_streaming,
        )?;
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post(Some(session_id), "chat/completions", &payload)
                    .await?;
                handle_status_openai_compat(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        if self.supports_streaming {
            super::openai_compatible::stream_openai_compat(response, log)
        } else {
            let json: serde_json::Value = response.json().await.map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to parse JSON: {}", e))
            })?;

            let message = super::formats::openai::response_to_message(&json).map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to parse message: {}", e))
            })?;

            let usage_data = super::formats::openai::get_usage(
                json.get("usage").unwrap_or(&serde_json::Value::Null),
            );
            let usage =
                super::base::ProviderUsage::new(model_config.model_name.clone(), usage_data);

            log.write(
                &serde_json::to_value(&message).unwrap_or_default(),
                Some(&usage_data),
            )?;

            Ok(super::base::stream_from_single_message(message, usage))
        }
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request(None, "models")
            .response_get()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to fetch models: {}", e)))?;

        let json = super::openai_compatible::handle_response_openai_compat(response).await?;
        if let Some(err_obj) = json.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::Authentication(msg.to_string()));
        }

        let data = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::UsageError("Missing data field in JSON response".into())
        })?;
        let mut models: Vec<String> = data
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        models.sort();
        Ok(models)
    }

    async fn fetch_recommended_models(&self) -> Result<Vec<String>, ProviderError> {
        self.fetch_supported_models().await
    }
}
