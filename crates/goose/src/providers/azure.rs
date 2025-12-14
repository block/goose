use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;

use super::api_client::{ApiClient, AuthMethod, AuthProvider};
use super::azureauth::{AuthError, AzureAuth};
use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::retry::ProviderRetry;
use super::utils::{get_model, handle_response_openai_compat, ImageFormat};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::utils::RequestLog;
use rmcp::model::Tool;

pub const AZURE_DEFAULT_MODEL: &str = "gpt-4o";
pub const AZURE_DOC_URL: &str =
    "https://learn.microsoft.com/en-us/azure/ai-services/openai/concepts/models";
/// Default API version - updated to support GPT-5 reasoning models
pub const AZURE_DEFAULT_API_VERSION: &str = "2025-04-01-preview";
pub const AZURE_OPENAI_KNOWN_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
];

#[derive(Debug)]
pub struct AzureProvider {
    api_client: ApiClient,
    deployment_name: String,
    api_version: String,
    model: ModelConfig,
    name: String,
    /// Use the v1 API format (/openai/v1/chat/completions) instead of the legacy
    /// deployment-based API. Required for GPT-5 reasoning models that use reasoning_effort.
    use_v1_api: bool,
}

impl Serialize for AzureProvider {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AzureProvider", 3)?;
        state.serialize_field("deployment_name", &self.deployment_name)?;
        state.serialize_field("api_version", &self.api_version)?;
        state.serialize_field("use_v1_api", &self.use_v1_api)?;
        state.end()
    }
}

// Custom auth provider that wraps AzureAuth
struct AzureAuthProvider {
    auth: AzureAuth,
}

#[async_trait]
impl AuthProvider for AzureAuthProvider {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        let auth_token = self
            .auth
            .get_token()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get authentication token: {}", e))?;

        match self.auth.credential_type() {
            super::azureauth::AzureCredentials::ApiKey(_) => {
                Ok(("api-key".to_string(), auth_token.token_value))
            }
            super::azureauth::AzureCredentials::DefaultCredential => Ok((
                "Authorization".to_string(),
                format!("Bearer {}", auth_token.token_value),
            )),
        }
    }
}

impl AzureProvider {
    /// Check if a model/deployment name indicates a reasoning model that requires v1 API
    fn is_reasoning_model(name: &str) -> bool {
        let lower = name.to_lowercase();
        lower.starts_with("o1")
            || lower.starts_with("o3")
            || lower.starts_with("o4")
            || lower.starts_with("gpt-5")
    }

    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let endpoint: String = config.get_param("AZURE_OPENAI_ENDPOINT")?;
        let deployment_name: String = config.get_param("AZURE_OPENAI_DEPLOYMENT_NAME")?;
        let api_version: String = config
            .get_param("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| AZURE_DEFAULT_API_VERSION.to_string());

        // Check if user explicitly requested v1 API, or auto-detect for reasoning models
        let use_v1_api_config: Option<bool> = config
            .get_param::<String>("AZURE_OPENAI_USE_V1_API")
            .ok()
            .and_then(|s| s.parse().ok());

        let use_v1_api = use_v1_api_config.unwrap_or_else(|| {
            // Auto-detect: use v1 API for reasoning models (GPT-5, o1, o3, o4)
            // These models require reasoning_effort parameter which only works with v1 API
            Self::is_reasoning_model(&deployment_name) || Self::is_reasoning_model(&model.model_name)
        });

        if use_v1_api {
            tracing::info!(
                "Using Azure OpenAI v1 API for deployment '{}' (reasoning model support)",
                deployment_name
            );
        }

        let api_key = config
            .get_secret("AZURE_OPENAI_API_KEY")
            .ok()
            .filter(|key: &String| !key.is_empty());
        let auth = AzureAuth::new(api_key).map_err(|e| match e {
            AuthError::Credentials(msg) => anyhow::anyhow!("Credentials error: {}", msg),
            AuthError::TokenExchange(msg) => anyhow::anyhow!("Token exchange error: {}", msg),
        })?;

        let auth_provider = AzureAuthProvider { auth };
        let api_client = ApiClient::new(endpoint, AuthMethod::Custom(Box::new(auth_provider)))?;

        Ok(Self {
            api_client,
            deployment_name,
            api_version,
            model,
            name: Self::metadata().name,
            use_v1_api,
        })
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        // Build the path for Azure OpenAI
        // Use v1 API for reasoning models (GPT-5, o1, o3, o4) which require reasoning_effort parameter
        let path = if self.use_v1_api {
            // v1 API: model/deployment specified in request body, no api-version in path
            "openai/v1/chat/completions".to_string()
        } else {
            // Legacy deployment-based API
            format!(
                "openai/deployments/{}/chat/completions?api-version={}",
                self.deployment_name, self.api_version
            )
        };

        let response = self.api_client.response_post(&path, payload).await?;
        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for AzureProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "azure_openai",
            "Azure OpenAI",
            "Models through Azure OpenAI Service (uses Azure credential chain by default). Supports GPT-5 reasoning models.",
            "gpt-4o",
            AZURE_OPENAI_KNOWN_MODELS.to_vec(),
            AZURE_DOC_URL,
            vec![
                ConfigKey::new("AZURE_OPENAI_ENDPOINT", true, false, None),
                ConfigKey::new("AZURE_OPENAI_DEPLOYMENT_NAME", true, false, None),
                ConfigKey::new(
                    "AZURE_OPENAI_API_VERSION",
                    true,
                    false,
                    Some(AZURE_DEFAULT_API_VERSION),
                ),
                ConfigKey::new("AZURE_OPENAI_API_KEY", true, true, Some("")),
                ConfigKey::new(
                    "AZURE_OPENAI_USE_V1_API",
                    false,
                    false,
                    Some("Auto-detected for GPT-5/o1/o3 models"),
                ),
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
            &ImageFormat::OpenAi,
            false,
        )?;
        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post(&payload_clone).await
            })
            .await?;

        let message = response_to_message(&response)?;
        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let response_model = get_model(&response);
        let mut log = RequestLog::start(model_config, &payload)?;
        log.write(&response, Some(&usage))?;
        Ok((message, ProviderUsage::new(response_model, usage)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_reasoning_model_gpt5_variants() {
        // GPT-5 models should be detected as reasoning models
        assert!(AzureProvider::is_reasoning_model("gpt-5"));
        assert!(AzureProvider::is_reasoning_model("gpt-5-mini"));
        assert!(AzureProvider::is_reasoning_model("gpt-5-nano"));
        assert!(AzureProvider::is_reasoning_model("GPT-5-mini")); // case insensitive
        assert!(AzureProvider::is_reasoning_model("gpt-5-mini-2025-08-07"));
    }

    #[test]
    fn test_is_reasoning_model_o_series() {
        // O-series models should be detected as reasoning models
        assert!(AzureProvider::is_reasoning_model("o1"));
        assert!(AzureProvider::is_reasoning_model("o1-mini"));
        assert!(AzureProvider::is_reasoning_model("o1-preview"));
        assert!(AzureProvider::is_reasoning_model("o3"));
        assert!(AzureProvider::is_reasoning_model("o3-mini"));
        assert!(AzureProvider::is_reasoning_model("o4"));
        assert!(AzureProvider::is_reasoning_model("O1")); // case insensitive
    }

    #[test]
    fn test_is_reasoning_model_non_reasoning() {
        // Non-reasoning models should NOT be detected
        assert!(!AzureProvider::is_reasoning_model("gpt-4o"));
        assert!(!AzureProvider::is_reasoning_model("gpt-4o-mini"));
        assert!(!AzureProvider::is_reasoning_model("gpt-4"));
        assert!(!AzureProvider::is_reasoning_model("gpt-4-turbo"));
        assert!(!AzureProvider::is_reasoning_model("gpt-35-turbo"));
    }

    #[test]
    fn test_default_api_version() {
        // Ensure default API version supports GPT-5 models
        assert_eq!(AZURE_DEFAULT_API_VERSION, "2025-04-01-preview");
    }

    #[test]
    fn test_known_models_include_gpt5() {
        // Ensure GPT-5 models are in the known models list
        assert!(AZURE_OPENAI_KNOWN_MODELS.contains(&"gpt-5"));
        assert!(AZURE_OPENAI_KNOWN_MODELS.contains(&"gpt-5-mini"));
        assert!(AZURE_OPENAI_KNOWN_MODELS.contains(&"gpt-5-nano"));
    }
}
