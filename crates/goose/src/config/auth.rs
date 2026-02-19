use serde::{Deserialize, Serialize};

use crate::oidc::{OidcProviderConfig, OidcProviderPreset};

/// Auth configuration section in config.yaml.
///
/// Supports two modes:
/// 1. **Preset**: `provider` + `client_id` + optional `tenant`
/// 2. **Custom OIDC**: full `oidc` block with issuer/audience
///
/// # Examples
///
/// ```yaml
/// auth:
///   provider: azure
///   tenant: my-tenant-id
///   client_id: my-client-id
/// ```
///
/// ```yaml
/// auth:
///   oidc:
///     issuer: https://accounts.google.com
///     audience: my-client-id
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Preset provider name: google, azure, github, gitlab, auth0, okta, aws
    #[serde(default)]
    pub provider: Option<String>,

    /// Tenant/domain for the provider (Azure tenant ID, GitLab host, etc.)
    #[serde(default)]
    pub tenant: Option<String>,

    /// OAuth2/OIDC client ID (audience)
    #[serde(default)]
    pub client_id: Option<String>,

    /// OAuth2/OIDC client secret (for confidential clients)
    #[serde(default)]
    pub client_secret: Option<String>,

    /// Custom OIDC configuration (alternative to preset)
    #[serde(default)]
    pub oidc: Option<CustomOidcConfig>,
}

/// Custom OIDC provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomOidcConfig {
    pub issuer: String,
    pub audience: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub tenant_claim: Option<String>,
    #[serde(default)]
    pub group_claim: Option<String>,
    #[serde(default)]
    pub required_groups: Vec<String>,
}

impl AuthConfig {
    /// Convert this config into an `OidcProviderConfig` for the validator.
    ///
    /// Returns `None` if the config is empty or incomplete.
    pub fn to_oidc_provider_config(&self) -> Option<OidcProviderConfig> {
        // Custom OIDC takes precedence
        if let Some(oidc) = &self.oidc {
            return Some(OidcProviderConfig {
                issuer: oidc.issuer.clone(),
                audience: oidc.audience.clone(),
                client_secret: oidc.client_secret.clone(),
                tenant_claim: oidc.tenant_claim.clone(),
                group_claim: oidc.group_claim.clone(),
                required_groups: oidc.required_groups.clone(),
            });
        }

        // Preset provider
        let provider_name = self.provider.as_deref()?;
        let client_id = self.client_id.as_deref()?;

        let preset = match provider_name.to_lowercase().as_str() {
            "google" => OidcProviderPreset::Google,
            "azure" | "azure_ad" | "azuread" | "microsoft" | "entra" => OidcProviderPreset::Azure,
            "github" => OidcProviderPreset::GitHub,
            "gitlab" => OidcProviderPreset::GitLab,
            "aws" | "cognito" => OidcProviderPreset::Aws,
            "auth0" => OidcProviderPreset::Auth0,
            "okta" => OidcProviderPreset::Okta,
            _ => {
                tracing::warn!(provider = provider_name, "Unknown auth provider preset");
                return None;
            }
        };

        let discovery_url = preset.discovery_url(self.tenant.as_deref());

        // Extract issuer from discovery URL (strip /.well-known/openid-configuration)
        let issuer = discovery_url
            .strip_suffix("/.well-known/openid-configuration")
            .unwrap_or(&discovery_url)
            .to_string();

        Some(OidcProviderConfig {
            issuer,
            audience: client_id.to_string(),
            client_secret: self.client_secret.clone(),
            tenant_claim: None,
            group_claim: None,
            required_groups: vec![],
        })
    }
}

/// Load auth config from goose config.yaml.
///
/// Reads the `auth` key from the config file. Returns `None` if not configured.
pub fn load_auth_config() -> Option<AuthConfig> {
    let config = crate::config::Config::global();
    config.get_param::<AuthConfig>("auth").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_preset() {
        let config = AuthConfig {
            provider: Some("azure".to_string()),
            tenant: Some("my-tenant".to_string()),
            client_id: Some("my-client".to_string()),
            client_secret: None,
            oidc: None,
        };

        let oidc = config.to_oidc_provider_config().unwrap();
        assert_eq!(
            oidc.issuer,
            "https://login.microsoftonline.com/my-tenant/v2.0"
        );
        assert_eq!(oidc.audience, "my-client");
    }

    #[test]
    fn test_google_preset() {
        let config = AuthConfig {
            provider: Some("google".to_string()),
            tenant: None,
            client_id: Some("my-client".to_string()),
            client_secret: None,
            oidc: None,
        };

        let oidc = config.to_oidc_provider_config().unwrap();
        assert_eq!(oidc.issuer, "https://accounts.google.com");
        assert_eq!(oidc.audience, "my-client");
    }

    #[test]
    fn test_custom_oidc() {
        let config = AuthConfig {
            provider: None,
            tenant: None,
            client_id: None,
            client_secret: None,
            oidc: Some(CustomOidcConfig {
                issuer: "https://my-idp.example.com".to_string(),
                audience: "my-app".to_string(),
                client_secret: Some("secret".to_string()),
                tenant_claim: Some("tid".to_string()),
                group_claim: Some("groups".to_string()),
                required_groups: vec!["admin".to_string()],
            }),
        };

        let oidc = config.to_oidc_provider_config().unwrap();
        assert_eq!(oidc.issuer, "https://my-idp.example.com");
        assert_eq!(oidc.audience, "my-app");
        assert_eq!(oidc.client_secret.as_deref(), Some("secret"));
        assert_eq!(oidc.tenant_claim.as_deref(), Some("tid"));
        assert_eq!(oidc.group_claim.as_deref(), Some("groups"));
        assert_eq!(oidc.required_groups, vec!["admin"]);
    }

    #[test]
    fn test_custom_oidc_takes_precedence() {
        let config = AuthConfig {
            provider: Some("azure".to_string()),
            tenant: Some("my-tenant".to_string()),
            client_id: Some("my-client".to_string()),
            client_secret: None,
            oidc: Some(CustomOidcConfig {
                issuer: "https://custom.example.com".to_string(),
                audience: "custom-app".to_string(),
                client_secret: None,
                tenant_claim: None,
                group_claim: None,
                required_groups: vec![],
            }),
        };

        let oidc = config.to_oidc_provider_config().unwrap();
        assert_eq!(oidc.issuer, "https://custom.example.com");
        assert_eq!(oidc.audience, "custom-app");
    }

    #[test]
    fn test_empty_config_returns_none() {
        let config = AuthConfig {
            provider: None,
            tenant: None,
            client_id: None,
            client_secret: None,
            oidc: None,
        };

        assert!(config.to_oidc_provider_config().is_none());
    }

    #[test]
    fn test_provider_aliases() {
        for name in &["azure", "azure_ad", "azuread", "microsoft", "entra"] {
            let config = AuthConfig {
                provider: Some(name.to_string()),
                tenant: Some("t".to_string()),
                client_id: Some("c".to_string()),
                client_secret: None,
                oidc: None,
            };
            let oidc = config.to_oidc_provider_config().unwrap();
            assert!(
                oidc.issuer.contains("microsoftonline.com"),
                "Failed for alias: {name}"
            );
        }
    }
}
