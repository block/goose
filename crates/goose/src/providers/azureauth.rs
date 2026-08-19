use crate::config::Config;
use crate::providers::oauth_device_flow::{
    refresh_device_flow_token, run_device_flow, DeviceFlowConfig, DeviceFlowTokenRefreshError,
    DeviceFlowTokens, RequestEncoding,
};
use crate::subprocess::SubprocessExt;
use chrono::{DateTime, Utc};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const DEFAULT_RESOURCE: &str = "https://cognitiveservices.azure.com";
const TOKEN_EXPIRY_SKEW_SECS: i64 = 30;

/// Represents errors that can occur during Azure authentication.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Error when loading credentials from the filesystem or environment
    #[error("Failed to load credentials: {0}")]
    Credentials(String),

    /// Error during token exchange
    #[error("Token exchange failed: {0}")]
    TokenExchange(String),
}

/// Represents an authentication token with its type and value.
#[derive(Debug, Clone)]
pub struct AuthToken {
    /// The type of the token (e.g., "Bearer")
    pub token_type: String,
    /// The actual token value
    pub token_value: String,
}

/// Represents the types of Azure credentials supported.
#[derive(Debug, Clone)]
pub enum AzureCredentials {
    /// API key based authentication
    ApiKey(String),
    /// Pre-acquired Microsoft Entra ID access token (e.g. AZURE_OPENAI_AD_TOKEN)
    BearerToken(String),
    /// Azure credential chain based authentication
    DefaultCredential,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntraDeviceCodeConfig {
    pub tenant_id: String,
    pub client_id: String,
    pub resource: String,
}

impl EntraDeviceCodeConfig {
    pub fn new(tenant_id: String, client_id: String, resource: String) -> Self {
        Self {
            tenant_id,
            client_id,
            resource,
        }
    }

    fn device_auth_url(&self) -> String {
        format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/devicecode",
            self.tenant_id
        )
    }

    fn token_url(&self) -> String {
        format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        )
    }

    fn scopes(&self) -> String {
        format!(
            "{}/.default openid profile offline_access",
            self.resource.trim_end_matches('/')
        )
    }

    fn secret_key(&self) -> String {
        let identity = format!("{}:{}:{}", self.tenant_id, self.client_id, self.resource);
        let encoded = identity
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        format!("azure_entra_device_code_{encoded}")
    }
}

/// Holds a cached token and its expiration time.
#[derive(Debug, Clone)]
struct CachedToken {
    token: AuthToken,
    expires_at: Instant,
}

/// Response from Azure token endpoint
#[derive(Debug, Clone, Deserialize)]
struct TokenResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "tokenType")]
    token_type: String,
    #[serde(rename = "expires_on")]
    expires_on: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedDeviceCodeTokens {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

/// Azure authentication handler that manages credentials and token caching.
#[derive(Debug)]
pub struct AzureAuth {
    credentials: AzureCredentials,
    resource: String,
    device_code: Option<EntraDeviceCodeConfig>,
    client: reqwest::Client,
    cached_token: Arc<RwLock<Option<CachedToken>>>,
    force_refresh: AtomicBool,
}

impl AzureAuth {
    /// Creates a new Azure authentication handler.
    ///
    /// Initializes the authentication handler by:
    /// 1. Loading credentials from environment
    /// 2. Setting up an HTTP client for token requests
    /// 3. Initializing the token cache
    ///
    /// # Returns
    /// * `Result<Self, AuthError>` - A new AzureAuth instance or an error if initialization fails
    pub fn new(api_key: Option<String>, ad_token: Option<String>) -> Result<Self, AuthError> {
        Self::new_with_resource(api_key, ad_token, DEFAULT_RESOURCE.to_string())
    }

    pub fn new_with_resource(
        api_key: Option<String>,
        ad_token: Option<String>,
        resource: String,
    ) -> Result<Self, AuthError> {
        let credentials = match (ad_token, api_key) {
            (Some(token), _) => AzureCredentials::BearerToken(token),
            (None, Some(key)) => AzureCredentials::ApiKey(key),
            (None, None) => AzureCredentials::DefaultCredential,
        };

        Ok(Self {
            credentials,
            resource,
            device_code: None,
            client: reqwest::Client::new(),
            cached_token: Arc::new(RwLock::new(None)),
            force_refresh: AtomicBool::new(false),
        })
    }

    pub fn new_with_device_code(config: EntraDeviceCodeConfig) -> Result<Self, AuthError> {
        if config.tenant_id.trim().is_empty()
            || config.client_id.trim().is_empty()
            || config.resource.trim().is_empty()
        {
            return Err(AuthError::Credentials(
                "tenant_id, client_id, and resource are required".to_string(),
            ));
        }

        Ok(Self {
            credentials: AzureCredentials::DefaultCredential,
            resource: config.resource.clone(),
            device_code: Some(config),
            client: reqwest::Client::new(),
            cached_token: Arc::new(RwLock::new(None)),
            force_refresh: AtomicBool::new(false),
        })
    }

    /// Returns the type of credentials being used.
    pub fn credential_type(&self) -> &AzureCredentials {
        &self.credentials
    }

    pub async fn invalidate_token(&self) {
        self.force_refresh.store(true, Ordering::Release);
        *self.cached_token.write().await = None;
    }

    /// Retrieves a valid authentication token.
    ///
    /// This method implements an efficient token management strategy:
    /// 1. For API key auth, returns the API key directly
    /// 2. For bearer token auth, returns the pre-acquired token directly
    /// 3. For Azure credential chain:
    ///    a. Checks the cache for a valid token
    ///    b. Returns the cached token if not expired
    ///    c. Obtains a new token if needed or expired
    ///    d. Uses double-checked locking for thread safety
    /// 4. For Entra Device Code auth, restores or refreshes securely stored tokens
    ///    before starting an interactive flow.
    ///
    /// # Returns
    /// * `Result<AuthToken, AuthError>` - A valid authentication token or an error
    pub async fn get_token(&self) -> Result<AuthToken, AuthError> {
        if let Some(config) = &self.device_code {
            return self.get_device_code_token(config).await;
        }

        match &self.credentials {
            AzureCredentials::ApiKey(key) => Ok(AuthToken {
                token_type: "Bearer".to_string(),
                token_value: key.clone(),
            }),
            AzureCredentials::BearerToken(token) => Ok(AuthToken {
                token_type: "Bearer".to_string(),
                token_value: token.clone(),
            }),
            AzureCredentials::DefaultCredential => self.get_default_credential_token().await,
        }
    }

    async fn get_device_code_token(
        &self,
        config: &EntraDeviceCodeConfig,
    ) -> Result<AuthToken, AuthError> {
        if let Some(cached) = self.cached_token.read().await.as_ref() {
            if cached.expires_at > Instant::now() && !self.force_refresh.load(Ordering::Acquire) {
                return Ok(cached.token.clone());
            }
        }

        let mut token_guard = self.cached_token.write().await;
        if let Some(cached) = token_guard.as_ref() {
            if cached.expires_at > Instant::now() && !self.force_refresh.load(Ordering::Acquire) {
                return Ok(cached.token.clone());
            }
        }

        let persisted = Config::global()
            .get_secret::<PersistedDeviceCodeTokens>(&config.secret_key())
            .ok();
        let force_refresh = self.force_refresh.load(Ordering::Acquire);
        if !force_refresh {
            if let Some(tokens) = persisted.as_ref().filter(|tokens| tokens_are_valid(tokens)) {
                let cached = cache_device_token(tokens)?;
                let token = cached.token.clone();
                *token_guard = Some(cached);
                return Ok(token);
            }
        }

        let device_auth_url = config.device_auth_url();
        let token_url = config.token_url();
        let scopes = config.scopes();
        let flow_config = DeviceFlowConfig {
            device_auth_url: Some(&device_auth_url),
            token_url: &token_url,
            client_id: &config.client_id,
            scopes: Some(&scopes),
            extra_headers: HeaderMap::new(),
            encoding: RequestEncoding::Form,
        };

        let old_refresh_token = persisted
            .as_ref()
            .and_then(|tokens| tokens.refresh_token.as_deref());
        let issued = if let Some(refresh_token) = old_refresh_token {
            match refresh_device_flow_token(&self.client, &flow_config, refresh_token).await {
                Ok(tokens) => tokens,
                Err(error) if refresh_requires_login(&error) => {
                    run_device_flow(&self.client, &flow_config)
                        .await
                        .map_err(token_exchange_error)?
                }
                Err(error) => return Err(token_exchange_error(error)),
            }
        } else {
            run_device_flow(&self.client, &flow_config)
                .await
                .map_err(token_exchange_error)?
        };

        let persisted = persisted_tokens(issued, old_refresh_token);
        Config::global()
            .set_secret(&config.secret_key(), &persisted)
            .map_err(|error| AuthError::Credentials(error.to_string()))?;
        let cached = cache_device_token(&persisted)?;
        let token = cached.token.clone();
        *token_guard = Some(cached);
        self.force_refresh.store(false, Ordering::Release);
        Ok(token)
    }

    async fn get_default_credential_token(&self) -> Result<AuthToken, AuthError> {
        // Try read lock first for better concurrency
        if let Some(cached) = self.cached_token.read().await.as_ref() {
            if cached.expires_at > Instant::now() {
                return Ok(cached.token.clone());
            }
        }

        // Take write lock only if needed
        let mut token_guard = self.cached_token.write().await;

        // Double-check expiration after acquiring write lock
        if let Some(cached) = token_guard.as_ref() {
            if cached.expires_at > Instant::now() {
                return Ok(cached.token.clone());
            }
        }

        let az = if cfg!(windows) { "az.cmd" } else { "az" };
        let output = tokio::process::Command::new(az)
            .args(["account", "get-access-token", "--resource", &self.resource])
            .set_no_window()
            .output()
            .await
            .map_err(|e| AuthError::TokenExchange(format!("Failed to execute Azure CLI: {e}")))?;

        if !output.status.success() {
            return Err(AuthError::TokenExchange(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let token_response: TokenResponse = serde_json::from_slice(&output.stdout)
            .map_err(|e| AuthError::TokenExchange(format!("Invalid token response: {e}")))?;
        let auth_token = AuthToken {
            token_type: token_response.token_type,
            token_value: token_response.access_token,
        };
        let expires_at = Instant::now()
            + Duration::from_secs(
                token_response
                    .expires_on
                    .saturating_sub(Utc::now().timestamp() as u64)
                    .saturating_sub(TOKEN_EXPIRY_SKEW_SECS as u64),
            );
        *token_guard = Some(CachedToken {
            token: auth_token.clone(),
            expires_at,
        });
        Ok(auth_token)
    }
}

fn persisted_tokens(
    tokens: DeviceFlowTokens,
    old_refresh_token: Option<&str>,
) -> PersistedDeviceCodeTokens {
    PersistedDeviceCodeTokens {
        access_token: tokens.access_token,
        refresh_token: tokens
            .refresh_token
            .or_else(|| old_refresh_token.map(str::to_string)),
        expires_at: tokens.expires_at,
    }
}

fn tokens_are_valid(tokens: &PersistedDeviceCodeTokens) -> bool {
    tokens.expires_at.is_some_and(|expires_at| {
        expires_at > Utc::now() + chrono::Duration::seconds(TOKEN_EXPIRY_SKEW_SECS)
    })
}

fn cache_device_token(tokens: &PersistedDeviceCodeTokens) -> Result<CachedToken, AuthError> {
    let expires_at = tokens
        .expires_at
        .ok_or_else(|| AuthError::TokenExchange("token response missing expires_in".to_string()))?;
    let remaining = (expires_at - Utc::now() - chrono::Duration::seconds(TOKEN_EXPIRY_SKEW_SECS))
        .to_std()
        .unwrap_or_default();
    Ok(CachedToken {
        token: AuthToken {
            token_type: "Bearer".to_string(),
            token_value: tokens.access_token.clone(),
        },
        expires_at: Instant::now() + remaining,
    })
}

fn refresh_requires_login(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<DeviceFlowTokenRefreshError>()
        .is_some_and(|error| {
            error.status == reqwest::StatusCode::BAD_REQUEST
                || error.status == reqwest::StatusCode::UNAUTHORIZED
        })
}

fn token_exchange_error(error: anyhow::Error) -> AuthError {
    AuthError::TokenExchange(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device_config() -> EntraDeviceCodeConfig {
        EntraDeviceCodeConfig::new(
            "organizations".to_string(),
            "client-id".to_string(),
            "https://example.azure.com/".to_string(),
        )
    }

    #[test]
    fn test_ad_token_takes_precedence_over_api_key() {
        let auth = AzureAuth::new(Some("key".to_string()), Some("token".to_string())).unwrap();
        assert!(matches!(
            auth.credential_type(),
            AzureCredentials::BearerToken(_)
        ));
    }

    #[test]
    fn test_api_key_when_no_ad_token() {
        let auth = AzureAuth::new(Some("key".to_string()), None).unwrap();
        assert!(matches!(
            auth.credential_type(),
            AzureCredentials::ApiKey(_)
        ));
    }

    #[test]
    fn test_default_credential_when_neither() {
        let auth = AzureAuth::new(None, None).unwrap();
        assert!(matches!(
            auth.credential_type(),
            AzureCredentials::DefaultCredential
        ));
    }

    #[tokio::test]
    async fn test_bearer_token_get_token() {
        let auth = AzureAuth::new(None, Some("my-token".to_string())).unwrap();
        let token = auth.get_token().await.unwrap();
        assert_eq!(token.token_type, "Bearer");
        assert_eq!(token.token_value, "my-token");
    }

    #[test]
    fn device_code_configuration_uses_entra_v2_endpoints_and_default_scope() {
        let config = device_config();
        assert_eq!(
            config.device_auth_url(),
            "https://login.microsoftonline.com/organizations/oauth2/v2.0/devicecode"
        );
        assert_eq!(
            config.token_url(),
            "https://login.microsoftonline.com/organizations/oauth2/v2.0/token"
        );
        assert_eq!(
            config.scopes(),
            "https://example.azure.com/.default openid profile offline_access"
        );
    }

    #[test]
    fn device_code_constructor_selects_device_mode_without_changing_header_selection() {
        let auth = AzureAuth::new_with_device_code(device_config()).unwrap();
        assert!(auth.device_code.is_some());
        assert!(matches!(
            auth.credential_type(),
            AzureCredentials::DefaultCredential
        ));
    }

    #[test]
    fn refresh_response_preserves_an_existing_refresh_token() {
        let persisted = persisted_tokens(
            DeviceFlowTokens {
                access_token: "new-access".to_string(),
                refresh_token: None,
                expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            },
            Some("old-refresh"),
        );
        assert_eq!(persisted.refresh_token.as_deref(), Some("old-refresh"));
    }

    #[test]
    fn device_credentials_are_scoped_by_configuration() {
        let first = device_config();
        let mut second = device_config();
        second.resource = "https://other.azure.com".to_string();
        assert_ne!(first.secret_key(), second.secret_key());
        assert!(first.secret_key().starts_with("azure_entra_device_code_"));
    }
}
