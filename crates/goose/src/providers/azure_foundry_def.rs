use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use goose_providers::api_client::{AuthMethod, AuthProvider, TlsConfig};
use goose_providers::azure_foundry::{endpoint_kind, AzureFoundryProvider, EndpointKind};
use goose_providers::base::{ProviderDescriptor, ProviderMetadata};

use crate::config::{Config, ExtensionConfig};
use crate::providers::azureauth::{AzureAuth, AzureCredentials, EntraDeviceCodeConfig};
use crate::providers::base::ProviderDef;

const AZURE_PROJECT_ENTRA_RESOURCE: &str = "https://ai.azure.com";
const AZURE_MAAS_ENTRA_RESOURCE: &str = "https://ml.azure.com";

enum AuthHeader {
    ApiKey,
    Bearer,
}

struct AzureFoundryAuthProvider {
    auth: Arc<AzureAuth>,
    header: AuthHeader,
}

#[async_trait]
impl AuthProvider for AzureFoundryAuthProvider {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        let token = self.auth.get_token().await?;
        match &self.header {
            AuthHeader::ApiKey => Ok(("api-key".to_string(), token.token_value)),
            AuthHeader::Bearer => Ok((
                "Authorization".to_string(),
                format!("Bearer {}", token.token_value),
            )),
        }
    }

    async fn refresh_credentials(&self) -> Result<()> {
        self.auth.invalidate_token().await;
        Ok(())
    }
}

pub struct AzureFoundryProviderDef;

impl ProviderDescriptor for AzureFoundryProviderDef {
    fn metadata() -> ProviderMetadata {
        AzureFoundryProvider::metadata()
    }
}

impl ProviderDef for AzureFoundryProviderDef {
    type Provider = AzureFoundryProvider;

    fn from_env(
        _extensions: Vec<ExtensionConfig>,
        tls_config: Option<TlsConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(from_env(tls_config))
    }
}

pub async fn from_env(tls_config: Option<TlsConfig>) -> Result<AzureFoundryProvider> {
    let config = Config::global();
    let endpoint: String = config.get_param("AZURE_FOUNDRY_ENDPOINT")?;
    let api_version = config.get_param("AZURE_FOUNDRY_API_VERSION").ok();
    let maas_model = config
        .get_param::<String>("AZURE_FOUNDRY_MODEL")
        .ok()
        .filter(|model| !model.trim().is_empty());
    let api_key = config
        .get_secret::<String>("AZURE_FOUNDRY_API_KEY")
        .ok()
        .filter(|key| !key.is_empty());
    let ad_token = config
        .get_secret::<String>("AZURE_FOUNDRY_AD_TOKEN")
        .ok()
        .filter(|token| !token.is_empty());
    let tenant_id = config
        .get_param::<String>("AZURE_FOUNDRY_ENTRA_TENANT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let client_id = config
        .get_param::<String>("AZURE_FOUNDRY_ENTRA_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let endpoint_kind = endpoint_kind(&endpoint);
    let resource = if endpoint_kind == EndpointKind::Maas {
        AZURE_MAAS_ENTRA_RESOURCE
    } else {
        AZURE_PROJECT_ENTRA_RESOURCE
    };
    let auth = Arc::new(build_auth(
        api_key, ad_token, tenant_id, client_id, resource,
    )?);
    let auth_method = |header| {
        AuthMethod::Custom(Box::new(AzureFoundryAuthProvider {
            auth: Arc::clone(&auth),
            header,
        }))
    };
    let anthropic_auth = match auth.credential_type() {
        AzureCredentials::ApiKey(key) => AuthMethod::ApiKey {
            header_name: "x-api-key".to_string(),
            key: key.clone(),
        },
        _ => auth_method(AuthHeader::Bearer),
    };
    let api_key_auth_header = || match auth.credential_type() {
        AzureCredentials::ApiKey(_) => AuthHeader::ApiKey,
        _ => AuthHeader::Bearer,
    };
    let chat_auth_header = match auth.credential_type() {
        AzureCredentials::ApiKey(_) if endpoint_kind == EndpointKind::Maas => AuthHeader::Bearer,
        _ => api_key_auth_header(),
    };

    AzureFoundryProvider::create(
        endpoint,
        api_version,
        maas_model,
        auth_method(chat_auth_header),
        auth_method(api_key_auth_header()),
        anthropic_auth,
        auth_method(api_key_auth_header()),
        tls_config,
        Some(crate::session_context::session_id_request_builder()),
    )
}

fn build_auth(
    api_key: Option<String>,
    ad_token: Option<String>,
    tenant_id: Option<String>,
    client_id: Option<String>,
    resource: &str,
) -> Result<AzureAuth> {
    let device_code = match (tenant_id, client_id) {
        (Some(tenant_id), Some(client_id)) => Some(EntraDeviceCodeConfig::new(
            tenant_id,
            client_id,
            resource.to_string(),
        )),
        (None, None) => None,
        _ => anyhow::bail!(
            "AZURE_FOUNDRY_ENTRA_TENANT_ID and AZURE_FOUNDRY_ENTRA_CLIENT_ID must be configured together"
        ),
    };
    if ad_token.is_some() || api_key.is_some() {
        return Ok(AzureAuth::new_with_resource(
            api_key,
            ad_token,
            resource.to_string(),
        )?);
    }
    if let Some(device_code) = device_code {
        return Ok(AzureAuth::new_with_device_code(device_code)?);
    }
    Ok(AzureAuth::new_with_resource(
        None,
        None,
        resource.to_string(),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn header(
        api_key: Option<&str>,
        ad_token: Option<&str>,
        header: AuthHeader,
    ) -> (String, String) {
        let auth = Arc::new(
            AzureAuth::new_with_resource(
                api_key.map(str::to_string),
                ad_token.map(str::to_string),
                AZURE_PROJECT_ENTRA_RESOURCE.to_string(),
            )
            .unwrap(),
        );
        AzureFoundryAuthProvider { auth, header }
            .get_auth_header()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn project_api_key_uses_api_key_header() {
        assert_eq!(
            header(Some("key"), None, AuthHeader::ApiKey).await,
            ("api-key".to_string(), "key".to_string())
        );
    }

    #[tokio::test]
    async fn maas_api_key_uses_bearer_header() {
        assert_eq!(
            header(Some("key"), None, AuthHeader::Bearer).await,
            ("Authorization".to_string(), "Bearer key".to_string())
        );
    }

    #[tokio::test]
    async fn entra_token_uses_bearer_header() {
        assert_eq!(
            header(None, Some("token"), AuthHeader::Bearer).await,
            ("Authorization".to_string(), "Bearer token".to_string())
        );
    }
    #[test]
    fn partial_device_configuration_is_rejected() {
        let error = build_auth(
            None,
            None,
            Some("tenant".to_string()),
            None,
            AZURE_PROJECT_ENTRA_RESOURCE,
        )
        .unwrap_err();
        assert!(error.to_string().contains("must be configured together"));
    }

    #[test]
    fn static_credentials_take_precedence_over_device_configuration() {
        let auth = build_auth(
            Some("key".to_string()),
            Some("token".to_string()),
            Some("tenant".to_string()),
            Some("client".to_string()),
            AZURE_PROJECT_ENTRA_RESOURCE,
        )
        .unwrap();
        assert!(matches!(
            auth.credential_type(),
            AzureCredentials::BearerToken(_)
        ));
    }

    #[test]
    fn complete_device_configuration_selects_interactive_auth() {
        let auth = build_auth(
            None,
            None,
            Some("tenant".to_string()),
            Some("client".to_string()),
            AZURE_PROJECT_ENTRA_RESOURCE,
        )
        .unwrap();
        assert!(matches!(
            auth.credential_type(),
            AzureCredentials::DefaultCredential
        ));
    }
}
