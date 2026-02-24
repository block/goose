use anyhow::Result;
use async_trait::async_trait;

use super::api_client::{ApiClient, AuthMethod, AuthProvider};
use super::azureauth::{AuthError, AzureAuth};
use super::base::{ConfigKey, ProviderDef, ProviderMetadata};
use super::openai_compatible::OpenAiCompatibleProvider;
use crate::model::ModelConfig;
use futures::future::BoxFuture;

const AZURE_PROVIDER_NAME: &str = "azure_openai";
pub const AZURE_DEFAULT_MODEL: &str = "gpt-4o";
pub const AZURE_DOC_URL: &str =
    "https://learn.microsoft.com/en-us/azure/ai-services/openai/concepts/models";
pub const AZURE_DEFAULT_API_VERSION: &str = "2024-10-21";

pub struct AzureProvider;

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

impl ProviderDef for AzureProvider {
    type Provider = OpenAiCompatibleProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::from_canonical(
            AZURE_PROVIDER_NAME,
            "Azure OpenAI",
            "Models through Azure OpenAI Service (uses Azure credential chain by default)",
            "gpt-4o",
            vec![],
            AZURE_DOC_URL,
            vec![
                ConfigKey::new("AZURE_OPENAI_ENDPOINT", true, false, None, true),
                ConfigKey::new("AZURE_OPENAI_DEPLOYMENT_NAME", true, false, None, true),
                ConfigKey::new(
                    "AZURE_OPENAI_API_VERSION",
                    true,
                    false,
                    Some("2024-10-21"),
                    false,
                ),
                ConfigKey::new("AZURE_OPENAI_API_KEY", false, true, Some(""), true),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(async move {
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
            let auth = AzureAuth::new(api_key).map_err(|e| match e {
                AuthError::Credentials(msg) => anyhow::anyhow!("Credentials error: {}", msg),
                AuthError::TokenExchange(msg) => anyhow::anyhow!("Token exchange error: {}", msg),
            })?;

            let auth_provider = AzureAuthProvider { auth };
            let host = format!("{}/openai", endpoint.trim_end_matches('/'));
            let api_client = ApiClient::new(host, AuthMethod::Custom(Box::new(auth_provider)))?
                .with_query(vec![("api-version".to_string(), api_version)]);

            Ok(OpenAiCompatibleProvider::new(
                AZURE_PROVIDER_NAME.to_string(),
                api_client,
                model,
                format!("deployments/{}/", deployment_name),
            ))
        })
    }
}
