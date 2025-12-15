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
pub const AZURE_DEFAULT_API_VERSION: &str = "2024-10-21";
// For Azure OpenAI, we don't use a static known_models list because
// the available "models" are actually user-configured deployments.
// The fetch_supported_models() method returns deployments from ARM API
// when configured, or falls back to the configured deployment name.
pub const AZURE_OPENAI_KNOWN_MODELS: &[&str] = &[];

#[derive(Debug)]
pub struct AzureProvider {
    api_client: ApiClient,
    endpoint: String,
    deployment_name: String,
    api_version: String,
    model: ModelConfig,
    name: String,
    // Optional: For dynamic deployment listing via ARM API
    subscription_id: Option<String>,
    resource_group: Option<String>,
}

impl Serialize for AzureProvider {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AzureProvider", 2)?;
        state.serialize_field("deployment_name", &self.deployment_name)?;
        state.serialize_field("api_version", &self.api_version)?;
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
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let endpoint: String = config.get_param("AZURE_OPENAI_ENDPOINT")?;
        let deployment_name: String = config.get_param("AZURE_OPENAI_DEPLOYMENT_NAME")?;
        let api_version: String = config
            .get_param("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| AZURE_DEFAULT_API_VERSION.to_string());

        // Optional ARM API configuration for dynamic deployment listing
        let subscription_id: Option<String> = config
            .get_param("AZURE_SUBSCRIPTION_ID")
            .ok()
            .filter(|s: &String| !s.is_empty());
        let resource_group: Option<String> = config
            .get_param("AZURE_RESOURCE_GROUP")
            .ok()
            .filter(|s: &String| !s.is_empty());

        let api_key = config
            .get_secret("AZURE_OPENAI_API_KEY")
            .ok()
            .filter(|key: &String| !key.is_empty());
        let auth = AzureAuth::new(api_key).map_err(|e| match e {
            AuthError::Credentials(msg) => anyhow::anyhow!("Credentials error: {}", msg),
            AuthError::TokenExchange(msg) => anyhow::anyhow!("Token exchange error: {}", msg),
        })?;

        let auth_provider = AzureAuthProvider { auth };
        let api_client = ApiClient::new(
            endpoint.clone(),
            AuthMethod::Custom(Box::new(auth_provider)),
        )?;

        Ok(Self {
            api_client,
            endpoint,
            deployment_name,
            api_version,
            model,
            name: Self::metadata().name,
            subscription_id,
            resource_group,
        })
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        // Build the path for Azure OpenAI
        let path = format!(
            "openai/deployments/{}/chat/completions?api-version={}",
            self.deployment_name, self.api_version
        );

        let response = self.api_client.response_post(&path, payload).await?;
        handle_response_openai_compat(response).await
    }

    /// Fetch deployments from Azure Resource Manager API
    async fn fetch_deployments_from_arm(
        &self,
        subscription_id: &str,
        resource_group: &str,
        account_name: &str,
    ) -> Result<Vec<String>, ProviderError> {
        // Get ARM token using Azure CLI
        let output = tokio::process::Command::new("az")
            .args([
                "account",
                "get-access-token",
                "--resource",
                "https://management.azure.com",
            ])
            .output()
            .await
            .map_err(|e| {
                ProviderError::ExecutionError(format!("Failed to execute Azure CLI: {}", e))
            })?;

        if !output.status.success() {
            return Err(ProviderError::ExecutionError(format!(
                "Azure CLI token request failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let token_response: Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| ProviderError::ExecutionError(format!("Invalid token response: {}", e)))?;

        let access_token = token_response["accessToken"].as_str().ok_or_else(|| {
            ProviderError::ExecutionError("No accessToken in response".to_string())
        })?;

        // Call ARM API to list deployments
        let arm_url = format!(
            "https://management.azure.com/subscriptions/{}/resourceGroups/{}/providers/Microsoft.CognitiveServices/accounts/{}/deployments?api-version=2023-05-01",
            subscription_id, resource_group, account_name
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&arm_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| ProviderError::ExecutionError(format!("ARM API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ExecutionError(format!(
                "ARM API returned {}: {}",
                status, body
            )));
        }

        let json: Value = response.json().await.map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to parse ARM API response: {}", e))
        })?;

        // Extract deployment names from the response
        let deployments = json["value"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|deployment| deployment["name"].as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        Ok(deployments)
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
                ConfigKey::new("AZURE_OPENAI_API_VERSION", false, false, Some("2024-10-21")),
                // API key is optional - Azure credential chain (az login) can be used instead
                ConfigKey::new("AZURE_OPENAI_API_KEY", false, true, Some("")),
                // Optional: For dynamic deployment listing via Azure Resource Manager API
                ConfigKey::new("AZURE_SUBSCRIPTION_ID", false, false, None),
                ConfigKey::new("AZURE_RESOURCE_GROUP", false, false, None),
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

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // For Azure OpenAI, "models" are actually deployments configured in the Azure account
        // If subscription ID and resource group are configured, query the ARM API
        if let (Some(subscription_id), Some(resource_group)) =
            (&self.subscription_id, &self.resource_group)
        {
            // Extract account name from endpoint (e.g., "https://myaccount.openai.azure.com/" -> "myaccount")
            let account_name = self
                .endpoint
                .trim_end_matches('/')
                .replace("https://", "")
                .replace("http://", "")
                .split('.')
                .next()
                .map(|s| s.to_string());

            if let Some(account) = account_name {
                match self
                    .fetch_deployments_from_arm(subscription_id, resource_group, &account)
                    .await
                {
                    Ok(deployments) => {
                        tracing::debug!(
                            "Azure OpenAI: found {} deployments via ARM API",
                            deployments.len()
                        );
                        return Ok(Some(deployments));
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Azure OpenAI: failed to fetch deployments via ARM API: {}, falling back to configured deployment",
                            e
                        );
                    }
                }
            }
        }

        // Fall back to returning just the currently configured deployment name
        tracing::debug!(
            "Azure OpenAI: returning configured deployment '{}' as available model",
            self.deployment_name
        );
        Ok(Some(vec![self.deployment_name.clone()]))
    }
}
