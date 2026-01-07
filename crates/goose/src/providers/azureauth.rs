//! # Azure Entra ID (Azure AD) Authentication Module
//!
//! This module provides Azure Entra ID authentication support for both the OpenAI provider
//! (when using OpenAI endpoints protected by Entra ID) and the Azure OpenAI provider.
//!
//! ## Design Pattern
//!
//! This implementation follows the pattern established by the **OpenAI Python SDK's
//! `azure_ad_token_provider`** parameter, which allows users to provide a callable that
//! returns a valid Azure AD token. This Rust implementation provides the same capability
//! with automatic token caching and refresh.
//!
//! ## Supported Authentication Methods
//!
//! The following authentication methods are supported, in priority order:
//!
//! 1. **Managed Identity** - For Azure-hosted workloads (VMs, App Service, AKS, etc.)
//! 2. **Client Certificate** - Service principal auth with X.509 certificate
//! 3. **Client Secret** - Service principal auth with client secret
//! 4. **API Key** - Traditional API key authentication (Azure OpenAI only)
//! 5. **Default Credential** - Falls back to Azure CLI authentication
//!
//! ## Environment Variables
//!
//! ### For OpenAI Provider (`openai` provider with Entra ID protection)
//!
//! Use the `OPENAI_AZURE_*` prefix for OpenAI endpoints protected by Entra ID:
//!
//! | Variable | Description | Required |
//! |----------|-------------|----------|
//! | `OPENAI_AZURE_TENANT_ID` | Azure AD tenant ID | For client secret/certificate auth |
//! | `OPENAI_AZURE_CLIENT_ID` | Application (client) ID | For client secret/certificate/user-assigned MI |
//! | `OPENAI_AZURE_CLIENT_SECRET` | Client secret value | For client secret auth |
//! | `OPENAI_AZURE_CERTIFICATE_PATH` | Path to PEM certificate file | For certificate auth (file) |
//! | `OPENAI_AZURE_CERTIFICATE` | PEM certificate content | For certificate auth (inline) |
//! | `OPENAI_AZURE_TOKEN_SCOPE` | Custom token scope/resource | Optional (defaults to cognitive services) |
//! | `OPENAI_AZURE_USE_MANAGED_IDENTITY` | Set to "true" or "1" | For managed identity auth |
//!
//! ### For Azure OpenAI Provider (`azure` provider)
//!
//! Use the `AZURE_OPENAI_*` prefix for Azure OpenAI Service:
//!
//! | Variable | Description | Required |
//! |----------|-------------|----------|
//! | `AZURE_OPENAI_TENANT_ID` | Azure AD tenant ID | For client secret/certificate auth |
//! | `AZURE_OPENAI_CLIENT_ID` | Application (client) ID | For client secret/certificate/user-assigned MI |
//! | `AZURE_OPENAI_CLIENT_SECRET` | Client secret value | For client secret auth |
//! | `AZURE_OPENAI_CERTIFICATE_PATH` | Path to PEM certificate file | For certificate auth (file) |
//! | `AZURE_OPENAI_CERTIFICATE` | PEM certificate content | For certificate auth (inline) |
//! | `AZURE_OPENAI_TOKEN_SCOPE` | Custom token scope/resource | Optional (defaults to cognitive services) |
//! | `AZURE_OPENAI_USE_MANAGED_IDENTITY` | Set to "true" or "1" | For managed identity auth |
//! | `AZURE_OPENAI_API_KEY` | API key (alternative to Entra auth) | For API key auth |
//!
//! ## Usage Examples
//!
//! ### Client Secret Authentication
//!
//! ```bash
//! # For OpenAI provider with Entra ID
//! export OPENAI_AZURE_TENANT_ID="your-tenant-id"
//! export OPENAI_AZURE_CLIENT_ID="your-client-id"
//! export OPENAI_AZURE_CLIENT_SECRET="your-client-secret"
//!
//! # For Azure OpenAI provider
//! export AZURE_OPENAI_TENANT_ID="your-tenant-id"
//! export AZURE_OPENAI_CLIENT_ID="your-client-id"
//! export AZURE_OPENAI_CLIENT_SECRET="your-client-secret"
//! ```
//!
//! ### Managed Identity Authentication
//!
//! ```bash
//! # System-assigned managed identity
//! export OPENAI_AZURE_USE_MANAGED_IDENTITY="true"
//!
//! # User-assigned managed identity
//! export OPENAI_AZURE_USE_MANAGED_IDENTITY="true"
//! export OPENAI_AZURE_CLIENT_ID="your-managed-identity-client-id"
//! ```
//!
//! ### Client Certificate Authentication
//!
//! ```bash
//! # From file path
//! export OPENAI_AZURE_TENANT_ID="your-tenant-id"
//! export OPENAI_AZURE_CLIENT_ID="your-client-id"
//! export OPENAI_AZURE_CERTIFICATE_PATH="/path/to/certificate.pem"
//!
//! # Or inline PEM content
//! export OPENAI_AZURE_CERTIFICATE="-----BEGIN CERTIFICATE-----..."
//! ```
//!
//! ## Token Caching and Refresh
//!
//! Tokens are automatically cached and refreshed 30 seconds before expiry. The implementation
//! uses a double-checked locking pattern for thread-safe token caching in async contexts.
//!
//! ## Security
//!
//! All credential types implement custom `Debug` traits that redact sensitive information
//! (secrets, certificates, tokens) to prevent accidental exposure in logs.

use chrono;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use url::Url;

/// Default Azure AD resource for cognitive services (used by Azure OpenAI)
pub const AZURE_COGNITIVE_SERVICES_RESOURCE: &str = "https://cognitiveservices.azure.com";

/// Azure Instance Metadata Service endpoint for managed identity
const IMDS_ENDPOINT: &str = "http://169.254.169.254/metadata/identity/oauth2/token";

/// Formats a resource string as a scope for Azure AD v2.0 endpoints.
/// Azure AD v2.0 requires scopes to end with `/.default` for client credentials flow.
fn format_scope(resource: &str) -> String {
    if resource.ends_with("/.default") {
        resource.to_string()
    } else {
        format!("{}/.default", resource)
    }
}

/// Maps an AuthError to an anyhow::Error with a contextual prefix.
/// This helper is used by both OpenAI and Azure providers to standardize error handling.
pub fn map_auth_error(error: AuthError, context: &str) -> anyhow::Error {
    match error {
        AuthError::Credentials(msg) => anyhow::anyhow!("{} credentials error: {}", context, msg),
        AuthError::TokenExchange(msg) => {
            anyhow::anyhow!("{} token exchange error: {}", context, msg)
        }
    }
}

/// Validates that both tenant_id and client_id are provided for authentication methods
/// that require them (client secret, client certificate).
/// Returns a tuple of (tenant_id, client_id) on success.
pub fn require_tenant_and_client_ids(
    tenant_id: &Option<String>,
    client_id: &Option<String>,
    env_prefix: &str,
) -> Result<(String, String), anyhow::Error> {
    match (tenant_id, client_id) {
        (Some(t), Some(c)) => Ok((t.clone(), c.clone())),
        _ => Err(anyhow::anyhow!(
            "When using service principal authentication, both {}_TENANT_ID and {}_CLIENT_ID must be set.",
            env_prefix,
            env_prefix
        )),
    }
}

/// Configuration for building Azure authentication from environment variables.
/// This struct is used to share auth configuration logic between OpenAI and Azure providers.
#[derive(Debug, Default)]
pub struct AzureAuthConfig {
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub certificate_path: Option<String>,
    pub certificate_pem: Option<String>,
    pub token_scope: Option<String>,
    pub use_managed_identity: bool,
    pub api_key: Option<String>,
}

impl AzureAuthConfig {
    /// Creates a new empty auth configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds an AzureAuth instance based on the configuration.
    /// Priority order: Managed Identity → Certificate → Client Secret → API Key → Default Credential
    ///
    /// # Arguments
    /// * `env_prefix` - Prefix for error messages (e.g., "OPENAI_AZURE" or "AZURE_OPENAI")
    ///
    /// # Returns
    /// * `Result<AzureAuth, anyhow::Error>` - The configured AzureAuth instance
    pub fn build(self, env_prefix: &str) -> Result<AzureAuth, anyhow::Error> {
        if self.use_managed_identity {
            // Managed Identity authentication
            let azure_auth = if let Some(client_id) = &self.client_id {
                AzureAuth::with_user_assigned_managed_identity(client_id.clone(), self.token_scope)
            } else {
                AzureAuth::with_managed_identity(self.token_scope)
            }
            .map_err(|e| map_auth_error(e, "Managed identity"))?;
            Ok(azure_auth)
        } else if let Some(cert_path) = &self.certificate_path {
            // Client Certificate authentication from file
            let (tenant_id, client_id) =
                require_tenant_and_client_ids(&self.tenant_id, &self.client_id, env_prefix)?;
            AzureAuth::with_client_certificate_file(
                tenant_id,
                client_id,
                cert_path,
                self.token_scope,
            )
            .map_err(|e| map_auth_error(e, "Certificate"))
        } else if let Some(cert_pem) = &self.certificate_pem {
            // Client Certificate authentication from PEM content
            let (tenant_id, client_id) =
                require_tenant_and_client_ids(&self.tenant_id, &self.client_id, env_prefix)?;
            AzureAuth::with_client_certificate(
                tenant_id,
                client_id,
                cert_pem.clone(),
                self.token_scope,
            )
            .map_err(|e| map_auth_error(e, "Certificate"))
        } else if let Some(client_secret) = &self.client_secret {
            // Client Secret authentication
            let (tenant_id, client_id) =
                require_tenant_and_client_ids(&self.tenant_id, &self.client_id, env_prefix)?;
            AzureAuth::with_client_secret(
                tenant_id,
                client_id,
                client_secret.clone(),
                self.token_scope,
            )
            .map_err(|e| map_auth_error(e, "Client secret"))
        } else {
            // API Key or Default Credential (Azure CLI)
            AzureAuth::new(self.api_key).map_err(|e| map_auth_error(e, "Azure"))
        }
    }

    /// Returns true if any Entra ID authentication method is configured.
    /// This is useful for determining whether to use Entra auth or fall back to API key.
    pub fn has_entra_auth(&self) -> bool {
        self.use_managed_identity
            || self.certificate_path.is_some()
            || self.certificate_pem.is_some()
            || self.client_secret.is_some()
    }
}

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
#[derive(Clone)]
pub struct AuthToken {
    /// The type of the token (e.g., "Bearer")
    pub token_type: String,
    /// The actual token value
    pub token_value: String,
}

impl std::fmt::Debug for AuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthToken")
            .field("token_type", &self.token_type)
            .field("token_value", &"[redacted]")
            .finish()
    }
}

/// Configuration for client secret (service principal) authentication.
#[derive(Clone)]
pub struct ClientSecretCredential {
    /// Azure AD tenant ID
    pub tenant_id: String,
    /// Application (client) ID
    pub client_id: String,
    /// Client secret value
    pub client_secret: String,
    /// Resource/scope to request token for (defaults to cognitive services)
    pub resource: String,
}

impl std::fmt::Debug for ClientSecretCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientSecretCredential")
            .field("tenant_id", &self.tenant_id)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[redacted]")
            .field("resource", &self.resource)
            .finish()
    }
}

impl ClientSecretCredential {
    /// Creates a new client secret credential configuration.
    pub fn new(
        tenant_id: String,
        client_id: String,
        client_secret: String,
        resource: Option<String>,
    ) -> Self {
        Self {
            tenant_id,
            client_id,
            client_secret,
            resource: resource.unwrap_or_else(|| AZURE_COGNITIVE_SERVICES_RESOURCE.to_string()),
        }
    }
}

/// Configuration for client certificate (service principal) authentication.
#[derive(Clone)]
pub struct ClientCertificateCredential {
    /// Azure AD tenant ID
    pub tenant_id: String,
    /// Application (client) ID
    pub client_id: String,
    /// PEM-encoded certificate (including private key)
    pub certificate_pem: String,
    /// Resource/scope to request token for (defaults to cognitive services)
    pub resource: String,
}

impl std::fmt::Debug for ClientCertificateCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientCertificateCredential")
            .field("tenant_id", &self.tenant_id)
            .field("client_id", &self.client_id)
            .field("certificate_pem", &"[redacted]")
            .field("resource", &self.resource)
            .finish()
    }
}

impl ClientCertificateCredential {
    /// Creates a new client certificate credential configuration.
    pub fn new(
        tenant_id: String,
        client_id: String,
        certificate_pem: String,
        resource: Option<String>,
    ) -> Self {
        Self {
            tenant_id,
            client_id,
            certificate_pem,
            resource: resource.unwrap_or_else(|| AZURE_COGNITIVE_SERVICES_RESOURCE.to_string()),
        }
    }

    /// Loads certificate from a file path.
    pub fn from_file(
        tenant_id: String,
        client_id: String,
        certificate_path: &str,
        resource: Option<String>,
    ) -> Result<Self, AuthError> {
        let certificate_pem = std::fs::read_to_string(certificate_path).map_err(|e| {
            AuthError::Credentials(format!(
                "Failed to read certificate file '{}': {}",
                certificate_path, e
            ))
        })?;
        Ok(Self::new(tenant_id, client_id, certificate_pem, resource))
    }
}

/// Configuration for managed identity authentication.
#[derive(Debug, Clone)]
pub struct ManagedIdentityCredential {
    /// Client ID for user-assigned managed identity (None for system-assigned)
    pub client_id: Option<String>,
    /// Resource/scope to request token for (defaults to cognitive services)
    pub resource: String,
}

impl ManagedIdentityCredential {
    /// Creates a new managed identity credential for system-assigned identity.
    pub fn system_assigned(resource: Option<String>) -> Self {
        Self {
            client_id: None,
            resource: resource.unwrap_or_else(|| AZURE_COGNITIVE_SERVICES_RESOURCE.to_string()),
        }
    }

    /// Creates a new managed identity credential for user-assigned identity.
    pub fn user_assigned(client_id: String, resource: Option<String>) -> Self {
        Self {
            client_id: Some(client_id),
            resource: resource.unwrap_or_else(|| AZURE_COGNITIVE_SERVICES_RESOURCE.to_string()),
        }
    }
}

/// Represents the types of Azure credentials supported.
#[derive(Clone)]
pub enum AzureCredentials {
    /// API key based authentication
    ApiKey(String),
    /// Azure credential chain based authentication (uses Azure CLI)
    DefaultCredential,
    /// Client secret (service principal) based authentication
    ClientSecret(ClientSecretCredential),
    /// Client certificate (service principal) based authentication
    ClientCertificate(ClientCertificateCredential),
    /// Managed identity based authentication (for Azure-hosted environments)
    ManagedIdentity(ManagedIdentityCredential),
}

impl std::fmt::Debug for AzureCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey(_) => f.debug_tuple("ApiKey").field(&"[redacted]").finish(),
            Self::DefaultCredential => write!(f, "DefaultCredential"),
            Self::ClientSecret(cred) => f.debug_tuple("ClientSecret").field(cred).finish(),
            Self::ClientCertificate(cred) => {
                f.debug_tuple("ClientCertificate").field(cred).finish()
            }
            Self::ManagedIdentity(cred) => f.debug_tuple("ManagedIdentity").field(cred).finish(),
        }
    }
}

/// Holds a cached token and its expiration time.
#[derive(Debug, Clone)]
struct CachedToken {
    token: AuthToken,
    expires_at: Instant,
}

/// Response from Azure CLI token command
#[derive(Debug, Clone, Deserialize)]
struct CliTokenResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "tokenType")]
    token_type: String,
    #[serde(rename = "expires_on")]
    expires_on: u64,
}

/// Response from Azure AD OAuth2 token endpoint
#[derive(Debug, Clone, Deserialize)]
struct OAuth2TokenResponse {
    access_token: String,
    token_type: String,
    /// Token lifetime in seconds
    expires_in: u64,
}

/// Response from Azure IMDS token endpoint
#[derive(Debug, Clone, Deserialize)]
struct ImdsTokenResponse {
    access_token: String,
    token_type: String,
    /// Token expiry as Unix timestamp string
    expires_on: String,
}

/// JWT claims for client certificate assertion
#[derive(Debug, Serialize)]
struct CertificateAssertionClaims {
    /// Audience (token endpoint URL)
    aud: String,
    /// Expiration time
    exp: i64,
    /// Issued at time
    iat: i64,
    /// Issuer (client ID)
    iss: String,
    /// JWT ID (unique identifier)
    jti: String,
    /// Not before time
    nbf: i64,
    /// Subject (client ID)
    sub: String,
}

/// Default Azure AD authority URL
const DEFAULT_AUTHORITY: &str = "https://login.microsoftonline.com";

/// Azure authentication handler that manages credentials and token caching.
pub struct AzureAuth {
    credentials: AzureCredentials,
    cached_token: Arc<RwLock<Option<CachedToken>>>,
    http_client: reqwest::Client,
    /// Azure AD authority URL (configurable for testing)
    token_authority: String,
    /// IMDS endpoint URL (configurable for testing)
    imds_endpoint: String,
}

impl std::fmt::Debug for AzureAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzureAuth")
            .field("credentials", &self.credentials)
            .field("cached_token", &"[cached]")
            .field("token_authority", &self.token_authority)
            .field("imds_endpoint", &self.imds_endpoint)
            .finish()
    }
}

impl AzureAuth {
    /// Creates a new AzureAuth instance with the given credentials and default endpoints.
    /// This is a private helper to avoid duplicating HTTP client initialization.
    fn new_with_credentials(credentials: AzureCredentials) -> Result<Self, AuthError> {
        Self::new_with_credentials_and_endpoints(
            credentials,
            DEFAULT_AUTHORITY.to_string(),
            IMDS_ENDPOINT.to_string(),
        )
    }

    /// Creates a new AzureAuth instance with custom endpoints for testing.
    fn new_with_credentials_and_endpoints(
        credentials: AzureCredentials,
        token_authority: String,
        imds_endpoint: String,
    ) -> Result<Self, AuthError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| AuthError::Credentials(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            credentials,
            cached_token: Arc::new(RwLock::new(None)),
            http_client,
            token_authority,
            imds_endpoint,
        })
    }

    /// Creates a new AzureAuth instance with custom endpoints.
    /// This is useful for unit testing with mock servers.
    #[cfg(test)]
    pub fn with_custom_endpoints(
        credentials: AzureCredentials,
        token_authority: String,
        imds_endpoint: String,
    ) -> Result<Self, AuthError> {
        Self::new_with_credentials_and_endpoints(credentials, token_authority, imds_endpoint)
    }

    /// Creates a new Azure authentication handler.
    ///
    /// Initializes the authentication handler by:
    /// 1. Loading credentials from environment
    /// 2. Setting up an HTTP client for token requests
    /// 3. Initializing the token cache
    ///
    /// # Returns
    /// * `Result<Self, AuthError>` - A new AzureAuth instance or an error if initialization fails
    pub fn new(api_key: Option<String>) -> Result<Self, AuthError> {
        let credentials = match api_key {
            Some(key) => AzureCredentials::ApiKey(key),
            None => AzureCredentials::DefaultCredential,
        };
        Self::new_with_credentials(credentials)
    }

    /// Creates a new Azure authentication handler with client secret credentials.
    ///
    /// This method configures authentication using a service principal (application)
    /// with a client secret, suitable for server-to-server authentication scenarios.
    ///
    /// # Arguments
    /// * `tenant_id` - Azure AD tenant ID
    /// * `client_id` - Application (client) ID
    /// * `client_secret` - Client secret value
    /// * `resource` - Optional resource/scope (defaults to cognitive services)
    ///
    /// # Returns
    /// * `Result<Self, AuthError>` - A new AzureAuth instance or an error if initialization fails
    pub fn with_client_secret(
        tenant_id: String,
        client_id: String,
        client_secret: String,
        resource: Option<String>,
    ) -> Result<Self, AuthError> {
        let credentials = AzureCredentials::ClientSecret(ClientSecretCredential::new(
            tenant_id,
            client_id,
            client_secret,
            resource,
        ));
        Self::new_with_credentials(credentials)
    }

    /// Creates a new Azure authentication handler with client certificate credentials.
    ///
    /// This method configures authentication using a service principal (application)
    /// with a client certificate, suitable for secure server-to-server authentication.
    ///
    /// # Arguments
    /// * `tenant_id` - Azure AD tenant ID
    /// * `client_id` - Application (client) ID
    /// * `certificate_pem` - PEM-encoded certificate with private key
    /// * `resource` - Optional resource/scope (defaults to cognitive services)
    ///
    /// # Returns
    /// * `Result<Self, AuthError>` - A new AzureAuth instance or an error if initialization fails
    pub fn with_client_certificate(
        tenant_id: String,
        client_id: String,
        certificate_pem: String,
        resource: Option<String>,
    ) -> Result<Self, AuthError> {
        let credentials = AzureCredentials::ClientCertificate(ClientCertificateCredential::new(
            tenant_id,
            client_id,
            certificate_pem,
            resource,
        ));
        Self::new_with_credentials(credentials)
    }

    /// Creates a new Azure authentication handler with client certificate from file.
    ///
    /// # Arguments
    /// * `tenant_id` - Azure AD tenant ID
    /// * `client_id` - Application (client) ID
    /// * `certificate_path` - Path to PEM file containing certificate and private key
    /// * `resource` - Optional resource/scope (defaults to cognitive services)
    ///
    /// # Returns
    /// * `Result<Self, AuthError>` - A new AzureAuth instance or an error if initialization fails
    pub fn with_client_certificate_file(
        tenant_id: String,
        client_id: String,
        certificate_path: &str,
        resource: Option<String>,
    ) -> Result<Self, AuthError> {
        let cred = ClientCertificateCredential::from_file(
            tenant_id,
            client_id,
            certificate_path,
            resource,
        )?;
        Self::new_with_credentials(AzureCredentials::ClientCertificate(cred))
    }

    /// Creates a new Azure authentication handler for system-assigned managed identity.
    ///
    /// Use this when running in an Azure environment (VM, App Service, AKS, etc.)
    /// with a system-assigned managed identity.
    ///
    /// # Arguments
    /// * `resource` - Optional resource/scope (defaults to cognitive services)
    ///
    /// # Returns
    /// * `Result<Self, AuthError>` - A new AzureAuth instance or an error if initialization fails
    pub fn with_managed_identity(resource: Option<String>) -> Result<Self, AuthError> {
        let credentials =
            AzureCredentials::ManagedIdentity(ManagedIdentityCredential::system_assigned(resource));
        Self::new_with_credentials(credentials)
    }

    /// Creates a new Azure authentication handler for user-assigned managed identity.
    ///
    /// Use this when running in an Azure environment with a user-assigned managed identity.
    ///
    /// # Arguments
    /// * `client_id` - The client ID of the user-assigned managed identity
    /// * `resource` - Optional resource/scope (defaults to cognitive services)
    ///
    /// # Returns
    /// * `Result<Self, AuthError>` - A new AzureAuth instance or an error if initialization fails
    pub fn with_user_assigned_managed_identity(
        client_id: String,
        resource: Option<String>,
    ) -> Result<Self, AuthError> {
        let credentials = AzureCredentials::ManagedIdentity(
            ManagedIdentityCredential::user_assigned(client_id, resource),
        );
        Self::new_with_credentials(credentials)
    }

    /// Returns the type of credentials being used.
    pub fn credential_type(&self) -> &AzureCredentials {
        &self.credentials
    }

    /// Helper method that implements the double-checked locking pattern for token caching.
    /// Accepts an async closure that fetches a new token when the cache is expired.
    async fn get_or_refresh_token<F, Fut>(&self, fetch_token: F) -> Result<AuthToken, AuthError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<(AuthToken, Instant), AuthError>>,
    {
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

        // Fetch new token using the provided closure
        let (auth_token, expires_at) = fetch_token().await?;

        *token_guard = Some(CachedToken {
            token: auth_token.clone(),
            expires_at,
        });

        Ok(auth_token)
    }

    /// Retrieves a valid authentication token.
    ///
    /// This method implements an efficient token management strategy:
    /// 1. For API key auth, returns the API key directly
    /// 2. For Azure credential chain (CLI):
    ///    a. Checks the cache for a valid token
    ///    b. Returns the cached token if not expired
    ///    c. Obtains a new token if needed or expired
    ///    d. Uses double-checked locking for thread safety
    /// 3. For client secret auth:
    ///    a. Uses cached token if valid
    ///    b. Requests new token from Azure AD OAuth2 endpoint if needed
    /// 4. For client certificate auth:
    ///    a. Uses cached token if valid
    ///    b. Creates JWT assertion signed with certificate
    ///    c. Exchanges JWT for access token
    /// 5. For managed identity:
    ///    a. Uses cached token if valid
    ///    b. Requests token from IMDS endpoint
    ///
    /// # Returns
    /// * `Result<AuthToken, AuthError>` - A valid authentication token or an error
    pub async fn get_token(&self) -> Result<AuthToken, AuthError> {
        match &self.credentials {
            AzureCredentials::ApiKey(key) => Ok(AuthToken {
                token_type: "api-key".to_string(),
                token_value: key.clone(),
            }),
            AzureCredentials::DefaultCredential => self.get_default_credential_token().await,
            AzureCredentials::ClientSecret(cred) => self.get_client_secret_token(cred).await,
            AzureCredentials::ClientCertificate(cred) => {
                self.get_client_certificate_token(cred).await
            }
            AzureCredentials::ManagedIdentity(cred) => self.get_managed_identity_token(cred).await,
        }
    }

    async fn get_default_credential_token(&self) -> Result<AuthToken, AuthError> {
        self.get_or_refresh_token(|| async {
            // Get new token using Azure CLI credential
            let output = tokio::process::Command::new("az")
                .args([
                    "account",
                    "get-access-token",
                    "--resource",
                    AZURE_COGNITIVE_SERVICES_RESOURCE,
                ])
                .output()
                .await
                .map_err(|e| {
                    AuthError::TokenExchange(format!("Failed to execute Azure CLI: {}", e))
                })?;

            if !output.status.success() {
                return Err(AuthError::TokenExchange(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }

            let token_response: CliTokenResponse = serde_json::from_slice(&output.stdout)
                .map_err(|e| AuthError::TokenExchange(format!("Invalid token response: {}", e)))?;

            let auth_token = AuthToken {
                token_type: token_response.token_type,
                token_value: token_response.access_token,
            };

            let expires_at = Instant::now()
                + Duration::from_secs(
                    token_response
                        .expires_on
                        .saturating_sub(chrono::Utc::now().timestamp() as u64)
                        .saturating_sub(30),
                );

            Ok((auth_token, expires_at))
        })
        .await
    }

    /// Retrieves a token using client secret credentials via Azure AD OAuth2 endpoint.
    async fn get_client_secret_token(
        &self,
        cred: &ClientSecretCredential,
    ) -> Result<AuthToken, AuthError> {
        let http_client = self.http_client.clone();
        let token_authority = self.token_authority.clone();
        let tenant_id = cred.tenant_id.clone();
        let client_id = cred.client_id.clone();
        let client_secret = cred.client_secret.clone();
        let scope = format_scope(&cred.resource);

        self.get_or_refresh_token(|| async move {
            // Request new token from Azure AD OAuth2 endpoint
            let token_url = format!("{}/{}/oauth2/v2.0/token", token_authority, tenant_id);

            let params = [
                ("grant_type", "client_credentials"),
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("scope", scope.as_str()),
            ];

            let response = http_client
                .post(&token_url)
                .form(&params)
                .send()
                .await
                .map_err(|e| AuthError::TokenExchange(format!("Failed to request token: {}", e)))?;

            if !response.status().is_success() {
                let status = response.status();
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(AuthError::TokenExchange(format!(
                    "Token request failed with status {}: {}",
                    status, error_body
                )));
            }

            let token_response: OAuth2TokenResponse = response
                .json()
                .await
                .map_err(|e| AuthError::TokenExchange(format!("Invalid token response: {}", e)))?;

            let auth_token = AuthToken {
                token_type: token_response.token_type,
                token_value: token_response.access_token,
            };

            // Cache with 30 second buffer before expiry
            let expires_at =
                Instant::now() + Duration::from_secs(token_response.expires_in.saturating_sub(30));

            Ok((auth_token, expires_at))
        })
        .await
    }

    /// Retrieves a token using client certificate credentials via Azure AD OAuth2 endpoint.
    async fn get_client_certificate_token(
        &self,
        cred: &ClientCertificateCredential,
    ) -> Result<AuthToken, AuthError> {
        // Create JWT assertion for client certificate auth (done outside closure to avoid lifetime issues)
        let token_url = format!(
            "{}/{}/oauth2/v2.0/token",
            self.token_authority, cred.tenant_id
        );
        let assertion = self.create_certificate_assertion(cred, &token_url)?;
        let scope = format_scope(&cred.resource);
        let client_id = cred.client_id.clone();
        let http_client = self.http_client.clone();

        self.get_or_refresh_token(|| async move {
            let params = [
                ("grant_type", "client_credentials"),
                ("client_id", client_id.as_str()),
                (
                    "client_assertion_type",
                    "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
                ),
                ("client_assertion", assertion.as_str()),
                ("scope", scope.as_str()),
            ];

            let response = http_client
                .post(&token_url)
                .form(&params)
                .send()
                .await
                .map_err(|e| AuthError::TokenExchange(format!("Failed to request token: {}", e)))?;

            if !response.status().is_success() {
                let status = response.status();
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(AuthError::TokenExchange(format!(
                    "Token request failed with status {}: {}",
                    status, error_body
                )));
            }

            let token_response: OAuth2TokenResponse = response
                .json()
                .await
                .map_err(|e| AuthError::TokenExchange(format!("Invalid token response: {}", e)))?;

            let auth_token = AuthToken {
                token_type: token_response.token_type,
                token_value: token_response.access_token,
            };

            // Cache with 30 second buffer before expiry
            let expires_at =
                Instant::now() + Duration::from_secs(token_response.expires_in.saturating_sub(30));

            Ok((auth_token, expires_at))
        })
        .await
    }

    /// Creates a JWT assertion for client certificate authentication.
    fn create_certificate_assertion(
        &self,
        cred: &ClientCertificateCredential,
        audience: &str,
    ) -> Result<String, AuthError> {
        // Parse the PEM to extract the private key and certificate
        let key = EncodingKey::from_rsa_pem(cred.certificate_pem.as_bytes()).map_err(|e| {
            AuthError::Credentials(format!("Failed to parse certificate private key: {}", e))
        })?;

        // Extract certificate thumbprint (SHA-256 hash of DER-encoded certificate)
        let thumbprint = self.extract_certificate_thumbprint(&cred.certificate_pem)?;

        // Create JWT header with x5t#S256 (certificate thumbprint)
        let mut header = Header::new(Algorithm::RS256);
        header.x5t_s256 = Some(thumbprint);

        // Create JWT claims
        let now = chrono::Utc::now().timestamp();
        let claims = CertificateAssertionClaims {
            aud: audience.to_string(),
            exp: now + 600, // 10 minutes
            iat: now,
            iss: cred.client_id.clone(),
            jti: uuid::Uuid::new_v4().to_string(),
            nbf: now,
            sub: cred.client_id.clone(),
        };

        encode(&header, &claims, &key)
            .map_err(|e| AuthError::Credentials(format!("Failed to create JWT assertion: {}", e)))
    }

    /// Extracts the SHA-256 thumbprint from a PEM certificate.
    fn extract_certificate_thumbprint(&self, pem_content: &str) -> Result<String, AuthError> {
        // Use pem crate for robust certificate parsing
        let pem_entries = pem::parse_many(pem_content)
            .map_err(|e| AuthError::Credentials(format!("Failed to parse PEM content: {}", e)))?;

        // Find the certificate entry
        let cert_pem = pem_entries
            .iter()
            .find(|p| p.tag() == "CERTIFICATE")
            .ok_or_else(|| AuthError::Credentials("No certificate found in PEM".to_string()))?;

        // Get the DER-encoded certificate content
        let der = cert_pem.contents();

        // Calculate SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(der);
        let hash = hasher.finalize();

        // Encode as base64url (without padding)
        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            hash,
        ))
    }

    /// Retrieves a token using managed identity via Azure IMDS endpoint.
    async fn get_managed_identity_token(
        &self,
        cred: &ManagedIdentityCredential,
    ) -> Result<AuthToken, AuthError> {
        // Build IMDS request URL using url crate for proper encoding
        let mut imds_url = Url::parse(&self.imds_endpoint)
            .map_err(|e| AuthError::Credentials(format!("Invalid IMDS endpoint URL: {}", e)))?;

        {
            let mut query_pairs = imds_url.query_pairs_mut();
            query_pairs.append_pair("api-version", "2018-02-01");
            query_pairs.append_pair("resource", &cred.resource);

            // Add client_id for user-assigned managed identity
            if let Some(client_id) = &cred.client_id {
                query_pairs.append_pair("client_id", client_id);
            }
        }

        let url_string = imds_url.to_string();
        let http_client = self.http_client.clone();

        self.get_or_refresh_token(|| async move {
            let response = http_client
                .get(&url_string)
                .header("Metadata", "true")
                .send()
                .await
                .map_err(|e| {
                    AuthError::TokenExchange(format!(
                        "Failed to request token from IMDS (are you running in Azure?): {}",
                        e
                    ))
                })?;

            if !response.status().is_success() {
                let status = response.status();
                let error_body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(AuthError::TokenExchange(format!(
                    "IMDS token request failed with status {} (ensure managed identity is configured): {}",
                    status, error_body
                )));
            }

            let token_response: ImdsTokenResponse = response
                .json()
                .await
                .map_err(|e| AuthError::TokenExchange(format!("Invalid IMDS token response: {}", e)))?;

            let auth_token = AuthToken {
                token_type: token_response.token_type,
                token_value: token_response.access_token,
            };

            // Parse expires_on as Unix timestamp and calculate duration
            let expires_on: u64 = token_response
                .expires_on
                .parse()
                .map_err(|e| AuthError::TokenExchange(format!("Invalid expires_on value: {}", e)))?;

            let expires_at = Instant::now()
                + Duration::from_secs(
                    expires_on
                        .saturating_sub(chrono::Utc::now().timestamp() as u64)
                        .saturating_sub(30),
                );

            Ok((auth_token, expires_at))
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_client_secret_credential_new() {
        let cred = ClientSecretCredential::new(
            "tenant-123".to_string(),
            "client-456".to_string(),
            "secret-789".to_string(),
            None,
        );

        assert_eq!(cred.tenant_id, "tenant-123");
        assert_eq!(cred.client_id, "client-456");
        assert_eq!(cred.client_secret, "secret-789");
        assert_eq!(cred.resource, AZURE_COGNITIVE_SERVICES_RESOURCE);
    }

    #[test]
    fn test_client_secret_credential_custom_resource() {
        let cred = ClientSecretCredential::new(
            "tenant-123".to_string(),
            "client-456".to_string(),
            "secret-789".to_string(),
            Some("https://custom.resource.com".to_string()),
        );

        assert_eq!(cred.resource, "https://custom.resource.com");
    }

    #[test]
    fn test_azure_auth_with_api_key() {
        let auth = AzureAuth::new(Some("test-api-key".to_string())).unwrap();

        match auth.credential_type() {
            AzureCredentials::ApiKey(key) => assert_eq!(key, "test-api-key"),
            _ => panic!("Expected ApiKey credential type"),
        }
    }

    #[test]
    fn test_azure_auth_with_client_secret() {
        let auth = AzureAuth::with_client_secret(
            "tenant-123".to_string(),
            "client-456".to_string(),
            "secret-789".to_string(),
            None,
        )
        .unwrap();

        match auth.credential_type() {
            AzureCredentials::ClientSecret(cred) => {
                assert_eq!(cred.tenant_id, "tenant-123");
                assert_eq!(cred.client_id, "client-456");
                assert_eq!(cred.client_secret, "secret-789");
            }
            _ => panic!("Expected ClientSecret credential type"),
        }
    }

    #[tokio::test]
    async fn test_api_key_returns_token_directly() {
        let auth = AzureAuth::new(Some("test-api-key".to_string())).unwrap();
        let token = auth.get_token().await.unwrap();

        assert_eq!(token.token_type, "api-key");
        assert_eq!(token.token_value, "test-api-key");
    }

    #[tokio::test]
    async fn test_client_secret_token_request_with_mock_server() {
        let mock_server = MockServer::start().await;

        let tenant_id = "test-tenant";
        let client_id = "test-client";
        let client_secret = "test-secret";

        // Mock the token endpoint response
        Mock::given(method("POST"))
            .and(path(format!("/{}/oauth2/v2.0/token", tenant_id)))
            .and(body_string_contains("grant_type=client_credentials"))
            .and(body_string_contains(
                format!("client_id={}", client_id).as_str(),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "mock-access-token",
                "token_type": "Bearer",
                "expires_in": 3600
            })))
            .mount(&mock_server)
            .await;

        // Create credential and use with_custom_endpoints to point to mock server
        let credentials = AzureCredentials::ClientSecret(ClientSecretCredential::new(
            tenant_id.to_string(),
            client_id.to_string(),
            client_secret.to_string(),
            None,
        ));

        let auth = AzureAuth::with_custom_endpoints(
            credentials,
            mock_server.uri(),
            "http://169.254.169.254/metadata/identity/oauth2/token".to_string(),
        )
        .unwrap();

        // Test the full token acquisition flow
        let token = auth.get_token().await.unwrap();

        assert_eq!(token.token_type, "Bearer");
        assert_eq!(token.token_value, "mock-access-token");
    }

    #[tokio::test]
    async fn test_managed_identity_token_request_with_mock_server() {
        use wiremock::matchers::{header, query_param};

        let mock_server = MockServer::start().await;

        // Mock the IMDS endpoint response
        Mock::given(method("GET"))
            .and(query_param("api-version", "2018-02-01"))
            .and(query_param("resource", AZURE_COGNITIVE_SERVICES_RESOURCE))
            .and(header("Metadata", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "mock-managed-identity-token",
                "token_type": "Bearer",
                "expires_on": "9999999999"
            })))
            .mount(&mock_server)
            .await;

        // Create managed identity credential with mock IMDS endpoint
        let credentials =
            AzureCredentials::ManagedIdentity(ManagedIdentityCredential::system_assigned(None));

        let auth = AzureAuth::with_custom_endpoints(
            credentials,
            "https://login.microsoftonline.com".to_string(),
            mock_server.uri(),
        )
        .unwrap();

        // Test the full token acquisition flow
        let token = auth.get_token().await.unwrap();

        assert_eq!(token.token_type, "Bearer");
        assert_eq!(token.token_value, "mock-managed-identity-token");
    }

    #[test]
    fn test_default_credential_type() {
        let auth = AzureAuth::new(None).unwrap();

        match auth.credential_type() {
            AzureCredentials::DefaultCredential => {}
            _ => panic!("Expected DefaultCredential type"),
        }
    }

    #[test]
    fn test_managed_identity_system_assigned() {
        let cred = ManagedIdentityCredential::system_assigned(None);

        assert!(cred.client_id.is_none());
        assert_eq!(cred.resource, AZURE_COGNITIVE_SERVICES_RESOURCE);
    }

    #[test]
    fn test_managed_identity_user_assigned() {
        let cred = ManagedIdentityCredential::user_assigned(
            "my-identity-client-id".to_string(),
            Some("https://custom.resource.com".to_string()),
        );

        assert_eq!(cred.client_id, Some("my-identity-client-id".to_string()));
        assert_eq!(cred.resource, "https://custom.resource.com");
    }

    #[test]
    fn test_azure_auth_with_managed_identity() {
        let auth = AzureAuth::with_managed_identity(None).unwrap();

        match auth.credential_type() {
            AzureCredentials::ManagedIdentity(cred) => {
                assert!(cred.client_id.is_none());
            }
            _ => panic!("Expected ManagedIdentity credential type"),
        }
    }

    #[test]
    fn test_azure_auth_with_user_assigned_managed_identity() {
        let auth =
            AzureAuth::with_user_assigned_managed_identity("my-identity-id".to_string(), None)
                .unwrap();

        match auth.credential_type() {
            AzureCredentials::ManagedIdentity(cred) => {
                assert_eq!(cred.client_id, Some("my-identity-id".to_string()));
            }
            _ => panic!("Expected ManagedIdentity credential type"),
        }
    }

    #[test]
    fn test_client_certificate_credential_new() {
        let cred = ClientCertificateCredential::new(
            "tenant-123".to_string(),
            "client-456".to_string(),
            "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----".to_string(),
            None,
        );

        assert_eq!(cred.tenant_id, "tenant-123");
        assert_eq!(cred.client_id, "client-456");
        assert!(cred.certificate_pem.contains("BEGIN CERTIFICATE"));
        assert_eq!(cred.resource, AZURE_COGNITIVE_SERVICES_RESOURCE);
    }

    #[test]
    fn test_azure_auth_with_client_certificate() {
        // We can't fully test this without a valid certificate,
        // but we can verify the struct is created correctly
        let result = AzureAuth::with_client_certificate(
            "tenant-123".to_string(),
            "client-456".to_string(),
            "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----".to_string(),
            None,
        );

        // It should succeed in creating the struct (even if the cert is invalid)
        assert!(result.is_ok());

        let auth = result.unwrap();
        match auth.credential_type() {
            AzureCredentials::ClientCertificate(cred) => {
                assert_eq!(cred.tenant_id, "tenant-123");
                assert_eq!(cred.client_id, "client-456");
            }
            _ => panic!("Expected ClientCertificate credential type"),
        }
    }
}
