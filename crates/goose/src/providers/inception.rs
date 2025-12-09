use super::api_client::{ApiClient, AuthMethod};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{get_model, handle_response_openai_compat, RequestLog};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;
use serde_json::Value;

pub const INCEPTION_API_HOST: &str = "https://api.inceptionlabs.ai";
pub const INCEPTION_DEFAULT_MODEL: &str = "mercury-coder";
pub const INCEPTION_KNOWN_MODELS: &[&str] = &["mercury-coder"];

pub const INCEPTION_DOC_URL: &str = "https://www.inceptionlabs.ai/";

#[derive(serde::Serialize)]
pub struct InceptionProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl InceptionProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("INCEPTION_API_KEY")?;
        let host: String = config
            .get_param("INCEPTION_HOST")
            .unwrap_or_else(|_| INCEPTION_API_HOST.to_string());

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new(host, auth)?;

        Ok(Self {
            api_client,
            model,
            name: Self::metadata().name,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        tracing::debug!("Inception request model: {:?}", self.model.model_name);

        let response = self
            .api_client
            .response_post("v1/chat/completions", &payload)
            .await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for InceptionProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "inception",
            "Inception",
            "Mercury models from Inception leveraging diffusion for lightning speeds",
            INCEPTION_DEFAULT_MODEL,
            INCEPTION_KNOWN_MODELS.to_vec(),
            INCEPTION_DOC_URL,
            vec![
                ConfigKey::new("INCEPTION_API_KEY", true, true, None),
                ConfigKey::new("INCEPTION_HOST", false, false, Some(INCEPTION_API_HOST)),
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
        let payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
        )?;

        let mut log = RequestLog::start(&self.model, &payload)?;
        let response = self.with_retry(|| self.post(payload.clone())).await?;

        let message = response_to_message(&response)?;
        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let response_model = get_model(&response);
        log.write(&response, Some(&usage))?;
        Ok((message, ProviderUsage::new(response_model, usage)))
    }
}
