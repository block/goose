use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

use super::azureauth::AzureAuth;
use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::retry::ProviderRetry;
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::impl_provider_default;
use crate::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

pub const AZURE_DEFAULT_MODEL: &str = "gpt-4o";
pub const AZURE_DOC_URL: &str =
    "https://learn.microsoft.com/en-us/azure/ai-services/openai/concepts/models";
pub const AZURE_DEFAULT_API_VERSION: &str = "2024-10-21";
pub const AZURE_OPENAI_KNOWN_MODELS: &[&str] = &["gpt-4o", "gpt-4o-mini", "gpt-4"];

#[derive(Debug)]
pub struct AzureProvider {
    client: Client,
    auth: AzureAuth,
    endpoint: String,
    deployment_name: String,
    api_version: String,
    model: ModelConfig,
}

impl Serialize for AzureProvider {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AzureProvider", 3)?;
        state.serialize_field("endpoint", &self.endpoint)?;
        state.serialize_field("deployment_name", &self.deployment_name)?;
        state.serialize_field("api_version", &self.api_version)?;
        state.end()
    }
}

impl_provider_default!(AzureProvider);

impl AzureProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let endpoint: String = config.get_param("AZURE_OPENAI_ENDPOINT")?;
        let deployment_name: String = config.get_param("AZURE_OPENAI_DEPLOYMENT_NAME")?;
        let api_version: String = config
            .get_param("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| AZURE_DEFAULT_API_VERSION.to_string());

        let api_key = config
            .get_secret("AZURE_OPENAI_API_KEY")
            .ok()
            .filter(|key: &String| !key.is_empty());
        let auth = AzureAuth::new(api_key)?;

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            endpoint,
            auth,
            deployment_name,
            api_version,
            model,
        })
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        let mut base_url = url::Url::parse(&self.endpoint)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;

        // Get the existing path without trailing slashes
        let existing_path = base_url.path().trim_end_matches('/');
        let new_path = if existing_path.is_empty() {
            format!(
                "/openai/deployments/{}/chat/completions",
                self.deployment_name
            )
        } else {
            format!(
                "{}/openai/deployments/{}/chat/completions",
                existing_path, self.deployment_name
            )
        };

        base_url.set_path(&new_path);
        base_url.set_query(Some(&format!("api-version={}", self.api_version)));

        // Get a fresh auth token for each attempt
        let auth_token = self.auth.get_token().await.map_err(|e| {
            tracing::error!("Authentication error: {:?}", e);
            ProviderError::RequestFailed(format!("Failed to get authentication token: {}", e))
        })?;

        let mut request_builder = self.client.post(base_url.clone());
        let token_value = auth_token.token_value.clone();

        // Set the correct header based on authentication type
        match self.auth.credential_type() {
            super::azureauth::AzureCredentials::ApiKey(_) => {
                request_builder = request_builder.header("api-key", token_value.clone());
            }
            super::azureauth::AzureCredentials::DefaultCredential => {
                request_builder = request_builder
                    .header("Authorization", format!("Bearer {}", token_value.clone()));
            }
        }

        let response = request_builder.json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for AzureProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "azure_openai",
            "Azure OpenAI",
            "Models through Azure OpenAI Service (uses Azure credential chain by default)",
            "gpt-4o",
            AZURE_OPENAI_KNOWN_MODELS.to_vec(),
            AZURE_DOC_URL,
            vec![
                ConfigKey::new("AZURE_OPENAI_ENDPOINT", true, false, None),
                ConfigKey::new("AZURE_OPENAI_DEPLOYMENT_NAME", true, false, None),
                ConfigKey::new("AZURE_OPENAI_API_VERSION", true, false, Some("2024-10-21")),
                ConfigKey::new("AZURE_OPENAI_API_KEY", true, true, Some("")),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;
        let response = self.with_retry(|| async {
            let payload_clone = payload.clone();
            self.post(&payload_clone).await
        }).await?;

        let message = response_to_message(&response)?;
        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let model = get_model(&response);
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
