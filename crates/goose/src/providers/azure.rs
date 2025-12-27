use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use url::Url;

use super::api_client::{ApiClient, AuthMethod, AuthProvider};
use super::azureauth::{AuthError, AzureAuth};
use super::base::{AuthModeChoice, ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::formats::openai_responses::{
    create_responses_request, get_responses_usage, responses_api_to_message, ResponsesApiResponse,
};
use super::retry::ProviderRetry;
use super::utils::handle_response_openai_compat;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::utils::RequestLog;
use rmcp::model::Tool;

pub const AZURE_DEFAULT_MODEL: &str = "gpt-4o";
pub const AZURE_DOC_URL: &str =
    "https://learn.microsoft.com/en-us/azure/ai-services/openai/concepts/models";
pub const AZURE_DEFAULT_API_VERSION: &str = "2025-04-01-preview";
pub const AZURE_OPENAI_KNOWN_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4",
    "gpt-5.1",
    "gpt-5.1-chat",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex",
    "gpt-5.2",
    "gpt-5-pro",
    "sora",
    "gpt-image-1.5",
];

#[derive(Debug)]
pub struct AzureProvider {
    api_client: ApiClient,
    api_version: String,
    model: ModelConfig,
    name: String,
}

impl Serialize for AzureProvider {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AzureProvider", 2)?;
        state.serialize_field("api_version", &self.api_version)?;
        state.serialize_field("configured_model", &self.model.model_name)?;
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

fn normalize_azure_endpoint(raw_endpoint: &str) -> Result<(String, Option<String>)> {
    let normalized = if raw_endpoint.starts_with("http://") || raw_endpoint.starts_with("https://")
    {
        raw_endpoint.to_string()
    } else {
        format!("https://{}", raw_endpoint)
    };

    let url = Url::parse(&normalized)
        .map_err(|e| anyhow::anyhow!("Invalid AZURE_OPENAI_ENDPOINT '{}': {}", raw_endpoint, e))?;

    let scheme = url.scheme();
    let host = url.host_str().ok_or_else(|| {
        anyhow::anyhow!("Missing host in AZURE_OPENAI_ENDPOINT '{}'", raw_endpoint)
    })?;

    let mut base = format!("{}://{}", scheme, host);
    if let Some(port) = url.port() {
        base = format!("{}:{}", base, port);
    }

    let api_version = url
        .query_pairs()
        .find(|(name, _)| name == "api-version")
        .map(|(_, value)| value.to_string());

    Ok((base, api_version))
}

impl AzureProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let raw_endpoint: String = config.get_param("AZURE_OPENAI_ENDPOINT")?;
        let (endpoint, url_api_version) = normalize_azure_endpoint(&raw_endpoint)?;
        let api_version: String = config
            .get_param("AZURE_OPENAI_API_VERSION")
            .ok()
            .or(url_api_version)
            .unwrap_or_else(|| AZURE_DEFAULT_API_VERSION.to_string());

        // Determine authentication mode: API key (default) or Entra ID / default Azure credentials
        let auth_type: String = config
            .get_param("AZURE_OPENAI_AUTH_TYPE")
            .unwrap_or_else(|_| "api_key".to_string());
        let auth_type_normalized = auth_type.to_lowercase();

        tracing::debug!(
            "Initializing Azure OpenAI provider with auth_type={}",
            auth_type_normalized
        );

        let api_key = if auth_type_normalized == "entra_id" {
            None
        } else {
            config
                .get_secret("AZURE_OPENAI_API_KEY")
                .ok()
                .filter(|key: &String| !key.is_empty())
        };
        let auth = AzureAuth::new(api_key).map_err(|e| match e {
            AuthError::Credentials(msg) => anyhow::anyhow!("Credentials error: {}", msg),
            AuthError::TokenExchange(msg) => anyhow::anyhow!("Token exchange error: {}", msg),
        })?;

        let auth_provider = AzureAuthProvider { auth };
        let api_client = ApiClient::new(endpoint, AuthMethod::Custom(Box::new(auth_provider)))?;

        Ok(Self {
            api_client,
            api_version,
            model,
            name: Self::metadata().name,
        })
    }

    /// Call the Azure OpenAI Responses API
    ///
    /// POST /openai/responses?api-version=...
    ///
    /// The deployment name is passed via the `model` field in the request body.
    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        let path = format!("openai/responses?api-version={}", self.api_version);

        let response = self.api_client.response_post(&path, payload).await?;
        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for AzureProvider {
    fn metadata() -> ProviderMetadata {
        let mut auth_type_key =
            ConfigKey::new("AZURE_OPENAI_AUTH_TYPE", false, false, Some("api_key"));
        auth_type_key.auth_modes = Some(vec![
            AuthModeChoice {
                value: "api_key".to_string(),
                label: "Key Authentication".to_string(),
                description: Some(
                    "Azure OpenAI will use an API key stored securely in Goose configuration."
                        .to_string(),
                ),
                requires_api_key: true,
            },
            AuthModeChoice {
                value: "entra_id".to_string(),
                label: "Entra ID Authentication".to_string(),
                description: Some(
                    "Azure OpenAI will use your Azure Entra ID / default credentials (for example via az login). No API key is required in this mode and any configured API key will be ignored."
                        .to_string(),
                ),
                requires_api_key: false,
            },
        ]);

        ProviderMetadata::new(
            "azure_openai",
            "Azure OpenAI",
            "Models through Azure OpenAI Service using either API key or Azure Entra ID (default credential chain) authentication",
            AZURE_DEFAULT_MODEL,
            AZURE_OPENAI_KNOWN_MODELS.to_vec(),
            AZURE_DOC_URL,
            vec![
                ConfigKey::new("AZURE_OPENAI_ENDPOINT", true, false, None),
                ConfigKey::new(
                    "AZURE_OPENAI_API_VERSION",
                    true,
                    false,
                    Some(AZURE_DEFAULT_API_VERSION),
                ),
                auth_type_key,
                ConfigKey::new("AZURE_OPENAI_API_KEY", false, true, Some("")),
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
        // Use Azure OpenAI Responses API.
        // We follow the OpenAI Responses API contract: the `model` field is set
        // by create_responses_request() from model_config.model_name.
        // On Azure, this model identifier should be configured as the deployment name.
        let payload = create_responses_request(model_config, system, messages, tools)?;
        let mut log = RequestLog::start(model_config, &payload)?;

        let json_response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post(&payload_clone).await
            })
            .await?;

        let responses_api_response: ResponsesApiResponse =
            serde_json::from_value(json_response.clone()).map_err(|e| {
                ProviderError::ExecutionError(format!(
                    "Failed to parse Azure OpenAI responses API response: {}",
                    e
                ))
            })?;

        let message = responses_api_to_message(&responses_api_response)?;
        let usage = get_responses_usage(&responses_api_response);
        let response_model = responses_api_response.model.clone();

        log.write(&json_response, Some(&usage))?;
        Ok((message, ProviderUsage::new(response_model, usage)))
    }
}
