use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Claims extracted from a validated JWT token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedClaims {
    pub subject: String,
    pub issuer: String,
    pub audience: Vec<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub tenant: Option<String>,
    pub expires_at: Option<u64>,
    pub groups: Vec<String>,
}

/// Well-known OIDC provider presets for easy CLI login.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OidcProviderPreset {
    Google,
    Azure,
    GitHub,
    GitLab,
    Aws,
    Auth0,
    Okta,
}

impl OidcProviderPreset {
    /// OIDC discovery base URL for this provider.
    ///
    /// For providers with tenant-specific URLs (Azure, Auth0, Okta, AWS, GitLab),
    /// a `tenant` parameter customizes the URL. For single-tenant providers
    /// (Google, GitHub), it's ignored.
    pub fn discovery_url(&self, tenant: Option<&str>) -> String {
        match self {
            Self::Google => {
                "https://accounts.google.com/.well-known/openid-configuration".to_string()
            }
            Self::Azure => {
                let tid = tenant.unwrap_or("common");
                format!(
                    "https://login.microsoftonline.com/{tid}/v2.0/.well-known/openid-configuration"
                )
            }
            Self::GitHub => {
                // GitHub OIDC (Actions tokens) — for user login, GitHub uses OAuth2 not OIDC
                "https://token.actions.githubusercontent.com/.well-known/openid-configuration"
                    .to_string()
            }
            Self::GitLab => {
                let host = tenant.unwrap_or("gitlab.com");
                format!("https://{host}/.well-known/openid-configuration")
            }
            Self::Aws => {
                // AWS Cognito — requires pool ID as tenant
                let region_pool = tenant.unwrap_or("us-east-1_example");
                let (region, _pool_id) = region_pool.split_once('_').unwrap_or((region_pool, ""));
                format!("https://cognito-idp.{region}.amazonaws.com/{region_pool}/.well-known/openid-configuration")
            }
            Self::Auth0 => {
                let domain = tenant.unwrap_or("dev-example.auth0.com");
                format!("https://{domain}/.well-known/openid-configuration")
            }
            Self::Okta => {
                let domain = tenant.unwrap_or("dev-example.okta.com");
                format!("https://{domain}/.well-known/openid-configuration")
            }
        }
    }

    /// OAuth2 scopes typically needed for this provider.
    pub fn default_scopes(&self) -> &'static str {
        match self {
            Self::Google => "openid email profile",
            Self::Azure => "openid email profile",
            Self::GitHub => "openid",
            Self::GitLab => "openid email profile",
            Self::Aws => "openid email profile",
            Self::Auth0 => "openid email profile",
            Self::Okta => "openid email profile",
        }
    }

    /// Whether this provider supports standard OIDC authorization code flow.
    /// GitHub user login uses OAuth2 (not OIDC), so it needs a different flow.
    pub fn supports_oidc_code_flow(&self) -> bool {
        !matches!(self, Self::GitHub)
    }

    /// OAuth2 authorization URL for providers that don't use OIDC discovery
    /// (e.g., GitHub user login).
    pub fn oauth2_authorize_url(&self) -> Option<&'static str> {
        match self {
            Self::GitHub => Some("https://github.com/login/oauth/authorize"),
            _ => None,
        }
    }

    /// OAuth2 token URL for providers that don't use OIDC discovery.
    pub fn oauth2_token_url(&self) -> Option<&'static str> {
        match self {
            Self::GitHub => Some("https://github.com/login/oauth/access_token"),
            _ => None,
        }
    }

    /// User info URL for providers that return user data via a separate endpoint
    /// (GitHub doesn't include user info in the token itself).
    pub fn userinfo_url(&self) -> Option<&'static str> {
        match self {
            Self::GitHub => Some("https://api.github.com/user"),
            Self::GitLab => Some("https://gitlab.com/oauth/userinfo"),
            _ => None,
        }
    }

    /// Parse a provider name string into a preset.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "google" => Some(Self::Google),
            "azure" | "microsoft" | "entra" => Some(Self::Azure),
            "github" | "gh" => Some(Self::GitHub),
            "gitlab" | "gl" => Some(Self::GitLab),
            "aws" | "cognito" => Some(Self::Aws),
            "auth0" => Some(Self::Auth0),
            "okta" => Some(Self::Okta),
            _ => None,
        }
    }

    /// List all available presets.
    pub fn all() -> &'static [Self] {
        &[
            Self::Google,
            Self::Azure,
            Self::GitHub,
            Self::GitLab,
            Self::Aws,
            Self::Auth0,
            Self::Okta,
        ]
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Azure => "Microsoft Azure AD / Entra ID",
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
            Self::Aws => "AWS Cognito",
            Self::Auth0 => "Auth0",
            Self::Okta => "Okta",
        }
    }
}

impl std::fmt::Display for OidcProviderPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Configuration for a single OIDC provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcProviderConfig {
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

/// OIDC discovery document (subset of fields we need).
/// Result of an OIDC token exchange or refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenSet {
    pub id_token: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

/// Raw token response from the OIDC provider's token endpoint.
#[derive(Deserialize)]
struct OidcTokenResponse {
    id_token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct OidcDiscovery {
    issuer: String,
    jwks_uri: String,
    #[serde(default)]
    pub token_endpoint: Option<String>,
    #[serde(default)]
    pub authorization_endpoint: Option<String>,
}

/// JWKS key set from the provider.
#[derive(Debug, Clone, Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

/// A single JWK key.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct JwkKey {
    kid: Option<String>,
    kty: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    key_use: Option<String>,
    n: Option<String>,
    e: Option<String>,
    x: Option<String>,
    y: Option<String>,
    crv: Option<String>,
}

/// Raw JWT claims we decode (superset of standard + common custom claims).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawClaims {
    sub: Option<String>,
    iss: Option<String>,
    aud: Option<serde_json::Value>,
    exp: Option<u64>,
    email: Option<String>,
    name: Option<String>,
    preferred_username: Option<String>,
    groups: Option<Vec<String>>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

/// Cached JWKS keys with expiry.
struct CachedJwks {
    keys: Vec<JwkKey>,
    fetched_at: Instant,
}

/// OIDC validator that handles discovery, JWKS caching, and JWT validation.
pub struct OidcValidator {
    providers: RwLock<HashMap<String, OidcProviderConfig>>,
    jwks_cache: Arc<RwLock<HashMap<String, CachedJwks>>>,
    discovery_cache: Arc<RwLock<HashMap<String, OidcDiscovery>>>,
    http_client: reqwest::Client,
    jwks_ttl: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum OidcError {
    #[error("unknown issuer: {0}")]
    UnknownIssuer(String),
    #[error("discovery failed for {issuer}: {source}")]
    DiscoveryFailed {
        issuer: String,
        source: reqwest::Error,
    },
    #[error("discovery issuer mismatch: expected {expected}, got {actual}")]
    IssuerMismatch { expected: String, actual: String },
    #[error("JWKS fetch failed for {issuer}: {source}")]
    JwksFetchFailed {
        issuer: String,
        source: reqwest::Error,
    },
    #[error("no matching key found for kid={kid:?}")]
    KeyNotFound { kid: Option<String> },
    #[error("unsupported key type: {kty}")]
    UnsupportedKeyType { kty: String },
    #[error("JWT validation failed: {0}")]
    ValidationFailed(#[from] jsonwebtoken::errors::Error),
    #[error("missing subject claim")]
    MissingSubject,
    #[error("group authorization failed: user not in required groups")]
    GroupAuthorizationFailed,
    #[error("missing token endpoint for issuer: {0}")]
    MissingTokenEndpoint(String),
    #[error("token exchange failed for {issuer}: {message}")]
    TokenExchangeFailed { issuer: String, message: String },
}

impl OidcValidator {
    pub fn new(providers: Vec<OidcProviderConfig>) -> Self {
        let provider_map: HashMap<String, OidcProviderConfig> = providers
            .into_iter()
            .map(|p| (p.issuer.clone(), p))
            .collect();

        Self {
            providers: RwLock::new(provider_map),
            jwks_cache: Arc::new(RwLock::new(HashMap::new())),
            discovery_cache: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            jwks_ttl: Duration::from_secs(3600),
        }
    }

    pub async fn has_providers(&self) -> bool {
        !self.providers.read().await.is_empty()
    }

    pub async fn list_providers(&self) -> Vec<OidcProviderConfig> {
        self.providers.read().await.values().cloned().collect()
    }

    pub async fn add_provider(&self, config: OidcProviderConfig) {
        let issuer = config.issuer.clone();
        self.providers.write().await.insert(issuer.clone(), config);
        // Clear cached discovery/JWKS for this issuer so they're re-fetched
        self.discovery_cache.write().await.remove(&issuer);
        self.jwks_cache.write().await.remove(&issuer);
    }

    pub async fn remove_provider(&self, issuer: &str) {
        self.providers.write().await.remove(issuer);
        self.discovery_cache.write().await.remove(issuer);
        self.jwks_cache.write().await.remove(issuer);
    }

    /// Get the authorization and token endpoints for a provider via OIDC discovery.
    pub async fn discover_endpoints(
        &self,
        issuer: &str,
    ) -> Result<(Option<String>, Option<String>), OidcError> {
        let discovery = self.discover(issuer).await?;
        Ok((
            discovery.authorization_endpoint.clone(),
            discovery.token_endpoint.clone(),
        ))
    }

    /// Exchange an authorization code for tokens using the provider's token endpoint.
    pub async fn exchange_code(
        &self,
        issuer: &str,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OidcTokenSet, OidcError> {
        let providers = self.providers.read().await;
        let config = providers
            .get(issuer)
            .ok_or_else(|| OidcError::UnknownIssuer(issuer.to_string()))?;

        let client_id = config.audience.clone();
        let client_secret = config.client_secret.clone();
        drop(providers);

        let discovery = self.discover(issuer).await?;
        let token_endpoint = discovery
            .token_endpoint
            .as_ref()
            .ok_or_else(|| OidcError::MissingTokenEndpoint(issuer.to_string()))?;

        let mut params = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", redirect_uri.to_string()),
            ("client_id", client_id),
        ];
        if let Some(secret) = client_secret {
            params.push(("client_secret", secret));
        }

        let resp = self
            .http_client
            .post(token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| OidcError::TokenExchangeFailed {
                issuer: issuer.to_string(),
                message: e.to_string(),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(OidcError::TokenExchangeFailed {
                issuer: issuer.to_string(),
                message: format!("HTTP {}: {}", status, body),
            });
        }

        let token_resp: OidcTokenResponse =
            resp.json()
                .await
                .map_err(|e| OidcError::TokenExchangeFailed {
                    issuer: issuer.to_string(),
                    message: format!("failed to parse token response: {}", e),
                })?;

        let id_token = token_resp
            .id_token
            .ok_or_else(|| OidcError::TokenExchangeFailed {
                issuer: issuer.to_string(),
                message: "provider did not return an id_token".to_string(),
            })?;

        Ok(OidcTokenSet {
            id_token: Some(id_token),
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token,
            expires_in: token_resp.expires_in,
        })
    }

    /// Refresh tokens using a refresh_token grant.
    ///
    /// Returns a new token set. The provider may or may not issue a new refresh_token.
    pub async fn refresh_oidc_token(
        &self,
        issuer: &str,
        refresh_token: &str,
    ) -> Result<OidcTokenSet, OidcError> {
        let providers = self.providers.read().await;
        let config = providers
            .get(issuer)
            .ok_or_else(|| OidcError::UnknownIssuer(issuer.to_string()))?;

        let client_id = config.audience.clone();
        let client_secret = config.client_secret.clone();
        drop(providers);

        let discovery = self.discover(issuer).await?;
        let token_endpoint = discovery
            .token_endpoint
            .as_ref()
            .ok_or_else(|| OidcError::MissingTokenEndpoint(issuer.to_string()))?;

        let mut params = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token.to_string()),
            ("client_id", client_id),
        ];
        if let Some(secret) = client_secret {
            params.push(("client_secret", secret));
        }

        let resp = self
            .http_client
            .post(token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| OidcError::TokenExchangeFailed {
                issuer: issuer.to_string(),
                message: format!("refresh failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(OidcError::TokenExchangeFailed {
                issuer: issuer.to_string(),
                message: format!("refresh HTTP {}: {}", status, body),
            });
        }

        let token_resp: OidcTokenResponse =
            resp.json()
                .await
                .map_err(|e| OidcError::TokenExchangeFailed {
                    issuer: issuer.to_string(),
                    message: format!("failed to parse refresh response: {}", e),
                })?;

        Ok(OidcTokenSet {
            id_token: token_resp.id_token,
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token,
            expires_in: token_resp.expires_in,
        })
    }

    /// Validate a Bearer token and return validated claims.
    pub async fn validate_token(&self, token: &str) -> Result<ValidatedClaims, OidcError> {
        let header = decode_header(token)?;
        let kid = header.kid.clone();

        // Peek at unvalidated claims to find the issuer
        let issuer = self.extract_issuer_unvalidated(token)?;

        let providers = self.providers.read().await;
        let config = providers
            .get(&issuer)
            .ok_or_else(|| OidcError::UnknownIssuer(issuer.clone()))?;

        // Clone config fields we need before dropping the lock
        let audience = config.audience.clone();
        let tenant_claim = config.tenant_claim.clone();
        let group_claim = config.group_claim.clone();
        let required_groups = config.required_groups.clone();
        drop(providers);

        // Fetch or use cached JWKS
        let jwks = self.get_jwks(&issuer).await?;

        // Find the matching key
        let key = find_key(&jwks, &kid)?;

        // Build decoding key
        let decoding_key = build_decoding_key(&key)?;

        // Build validation
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[&issuer]);
        validation.set_audience(&[&audience]);
        validation.validate_exp = true;

        // Decode and validate
        let token_data = decode::<RawClaims>(token, &decoding_key, &validation)?;
        let raw = token_data.claims;

        let subject = raw.sub.ok_or(OidcError::MissingSubject)?;

        let aud = match raw.aud {
            Some(serde_json::Value::String(s)) => vec![s],
            Some(serde_json::Value::Array(arr)) => arr
                .into_iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            _ => vec![],
        };

        let name = raw.name.or(raw.preferred_username).or(raw.email.clone());

        // Extract tenant from custom claim if configured
        let tenant = tenant_claim.as_ref().and_then(|claim| {
            raw.extra
                .get(claim)
                .and_then(|v| v.as_str().map(String::from))
        });

        // Extract groups from custom claim if configured
        let groups = if let Some(gc) = &group_claim {
            raw.extra
                .get(gc)
                .and_then(|v| {
                    v.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                })
                .unwrap_or_default()
        } else {
            raw.groups.unwrap_or_default()
        };

        // Check required groups if configured
        if !required_groups.is_empty() {
            let has_required = required_groups
                .iter()
                .any(|required| groups.contains(required));
            if !has_required {
                return Err(OidcError::GroupAuthorizationFailed);
            }
        }

        Ok(ValidatedClaims {
            subject,
            issuer: issuer.clone(),
            audience: aud,
            email: raw.email,
            name,
            tenant,
            expires_at: raw.exp,
            groups,
        })
    }

    /// Extract issuer from unvalidated token (just base64 decode the payload).
    fn extract_issuer_unvalidated(&self, token: &str) -> Result<String, OidcError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(OidcError::ValidationFailed(
                jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken),
            ));
        }

        use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};
        let payload = URL_SAFE_NO_PAD.decode(parts[1]).map_err(|_| {
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken)
        })?;

        #[derive(Deserialize)]
        struct IssuerOnly {
            iss: Option<String>,
        }

        let claims: IssuerOnly = serde_json::from_slice(&payload).map_err(|_| {
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken)
        })?;

        claims
            .iss
            .ok_or_else(|| OidcError::UnknownIssuer("(missing iss claim)".into()))
    }

    /// Fetch JWKS keys, using cache if still fresh.
    async fn get_jwks(&self, issuer: &str) -> Result<Vec<JwkKey>, OidcError> {
        // Check cache
        {
            let cache = self.jwks_cache.read().await;
            if let Some(cached) = cache.get(issuer) {
                if cached.fetched_at.elapsed() < self.jwks_ttl {
                    return Ok(cached.keys.clone());
                }
            }
        }

        // Discover JWKS URI
        let discovery = self.discover(issuer).await?;

        // Fetch JWKS
        let resp = self
            .http_client
            .get(&discovery.jwks_uri)
            .send()
            .await
            .map_err(|e| OidcError::JwksFetchFailed {
                issuer: issuer.to_string(),
                source: e,
            })?;

        let jwks: JwksResponse = resp.json().await.map_err(|e| OidcError::JwksFetchFailed {
            issuer: issuer.to_string(),
            source: e,
        })?;

        // Cache
        let keys = jwks.keys.clone();
        {
            let mut cache = self.jwks_cache.write().await;
            cache.insert(
                issuer.to_string(),
                CachedJwks {
                    keys: jwks.keys,
                    fetched_at: Instant::now(),
                },
            );
        }

        Ok(keys)
    }

    /// Fetch OIDC discovery document.
    async fn discover(&self, issuer: &str) -> Result<OidcDiscovery, OidcError> {
        // Check cache
        {
            let cache = self.discovery_cache.read().await;
            if let Some(doc) = cache.get(issuer) {
                return Ok(doc.clone());
            }
        }

        let url = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );

        let resp =
            self.http_client
                .get(&url)
                .send()
                .await
                .map_err(|e| OidcError::DiscoveryFailed {
                    issuer: issuer.to_string(),
                    source: e,
                })?;

        let doc: OidcDiscovery = resp.json().await.map_err(|e| OidcError::DiscoveryFailed {
            issuer: issuer.to_string(),
            source: e,
        })?;

        // Validate issuer matches
        if doc.issuer != issuer {
            return Err(OidcError::IssuerMismatch {
                expected: issuer.to_string(),
                actual: doc.issuer,
            });
        }

        // Cache
        {
            let mut cache = self.discovery_cache.write().await;
            cache.insert(issuer.to_string(), doc.clone());
        }

        Ok(doc)
    }
}

fn find_key(keys: &[JwkKey], kid: &Option<String>) -> Result<JwkKey, OidcError> {
    // If kid is specified, match by kid
    if let Some(kid) = kid {
        keys.iter()
            .find(|k| k.kid.as_deref() == Some(kid))
            .cloned()
            .ok_or_else(|| OidcError::KeyNotFound {
                kid: Some(kid.clone()),
            })
    } else {
        // No kid — use first signing key
        keys.iter()
            .find(|k| k.key_use.as_deref() == Some("sig") || k.key_use.is_none())
            .cloned()
            .ok_or(OidcError::KeyNotFound { kid: None })
    }
}

fn build_decoding_key(key: &JwkKey) -> Result<DecodingKey, OidcError> {
    match key.kty.as_str() {
        "RSA" => {
            let n = key
                .n
                .as_deref()
                .ok_or_else(|| OidcError::UnsupportedKeyType {
                    kty: "RSA (missing n)".into(),
                })?;
            let e = key
                .e
                .as_deref()
                .ok_or_else(|| OidcError::UnsupportedKeyType {
                    kty: "RSA (missing e)".into(),
                })?;
            Ok(DecodingKey::from_rsa_components(n, e)?)
        }
        "EC" => {
            let x = key
                .x
                .as_deref()
                .ok_or_else(|| OidcError::UnsupportedKeyType {
                    kty: "EC (missing x)".into(),
                })?;
            let y = key
                .y
                .as_deref()
                .ok_or_else(|| OidcError::UnsupportedKeyType {
                    kty: "EC (missing y)".into(),
                })?;
            Ok(DecodingKey::from_ec_components(x, y)?)
        }
        other => Err(OidcError::UnsupportedKeyType {
            kty: other.to_string(),
        }),
    }
}

/// Convert validated OIDC claims to a UserIdentity.
impl ValidatedClaims {
    pub fn into_user_identity(self) -> crate::identity::UserIdentity {
        let display_name = self
            .name
            .clone()
            .or(self.email.clone())
            .unwrap_or_else(|| self.subject.clone());
        let mut user = crate::identity::UserIdentity::oidc(
            self.subject.clone(),
            display_name,
            self.issuer.clone(),
        );
        if let Some(tenant) = self.tenant {
            user = user.with_tenant(tenant);
        }
        user
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validator_no_providers() {
        let validator = OidcValidator::new(vec![]);
        assert!(!validator.has_providers().await);
    }

    #[tokio::test]
    async fn test_validator_with_providers() {
        let validator = OidcValidator::new(vec![
            OidcProviderConfig {
                issuer: "https://accounts.google.com".into(),
                audience: "my-app".into(),
                tenant_claim: None,
                group_claim: None,
                client_secret: None,
                required_groups: vec![],
            },
            OidcProviderConfig {
                issuer: "https://login.microsoftonline.com/tenant-id/v2.0".into(),
                audience: "my-app".into(),
                tenant_claim: Some("tid".into()),
                group_claim: Some("groups".into()),
                client_secret: None,
                required_groups: vec!["goose-users".into()],
            },
        ]);
        assert!(validator.has_providers().await);
    }

    #[tokio::test]
    async fn test_add_and_remove_provider() {
        let validator = OidcValidator::new(vec![]);
        assert!(!validator.has_providers().await);

        validator
            .add_provider(OidcProviderConfig {
                issuer: "https://accounts.google.com".into(),
                audience: "my-app".into(),
                tenant_claim: None,
                group_claim: None,
                client_secret: None,
                required_groups: vec![],
            })
            .await;
        assert!(validator.has_providers().await);
        assert_eq!(validator.list_providers().await.len(), 1);

        validator
            .remove_provider("https://accounts.google.com")
            .await;
        assert!(!validator.has_providers().await);
        assert!(validator.list_providers().await.is_empty());
    }

    #[test]
    fn test_extract_issuer_unvalidated() {
        use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};

        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let payload =
            URL_SAFE_NO_PAD.encode(r#"{"iss":"https://accounts.google.com","sub":"user123"}"#);
        let sig = URL_SAFE_NO_PAD.encode("fake-signature");
        let token = format!("{}.{}.{}", header, payload, sig);

        let validator = OidcValidator::new(vec![OidcProviderConfig {
            issuer: "https://accounts.google.com".into(),
            audience: "test".into(),
            tenant_claim: None,
            group_claim: None,
            client_secret: None,
            required_groups: vec![],
        }]);

        let issuer = validator.extract_issuer_unvalidated(&token).unwrap();
        assert_eq!(issuer, "https://accounts.google.com");
    }

    #[test]
    fn test_extract_issuer_invalid_token() {
        let validator = OidcValidator::new(vec![]);
        assert!(validator.extract_issuer_unvalidated("not-a-jwt").is_err());
    }

    #[test]
    fn test_extract_issuer_missing_iss() {
        use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};

        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#);
        let payload = URL_SAFE_NO_PAD.encode(r#"{"sub":"user123"}"#);
        let sig = URL_SAFE_NO_PAD.encode("sig");
        let token = format!("{}.{}.{}", header, payload, sig);

        let validator = OidcValidator::new(vec![]);
        assert!(validator.extract_issuer_unvalidated(&token).is_err());
    }

    #[test]
    fn test_find_key_by_kid() {
        let keys = vec![
            JwkKey {
                kid: Some("key-1".into()),
                kty: "RSA".into(),
                alg: Some("RS256".into()),
                key_use: Some("sig".into()),
                n: Some("modulus".into()),
                e: Some("exponent".into()),
                x: None,
                y: None,
                crv: None,
            },
            JwkKey {
                kid: Some("key-2".into()),
                kty: "RSA".into(),
                alg: Some("RS256".into()),
                key_use: Some("sig".into()),
                n: Some("modulus2".into()),
                e: Some("exponent2".into()),
                x: None,
                y: None,
                crv: None,
            },
        ];

        let found = find_key(&keys, &Some("key-2".into())).unwrap();
        assert_eq!(found.kid.unwrap(), "key-2");
    }

    #[test]
    fn test_find_key_no_kid_uses_first_sig() {
        let keys = vec![JwkKey {
            kid: None,
            kty: "RSA".into(),
            alg: None,
            key_use: Some("sig".into()),
            n: Some("n".into()),
            e: Some("e".into()),
            x: None,
            y: None,
            crv: None,
        }];

        assert!(find_key(&keys, &None).is_ok());
    }

    #[test]
    fn test_find_key_missing_kid() {
        let keys = vec![JwkKey {
            kid: Some("other".into()),
            kty: "RSA".into(),
            alg: None,
            key_use: Some("sig".into()),
            n: Some("n".into()),
            e: Some("e".into()),
            x: None,
            y: None,
            crv: None,
        }];

        assert!(find_key(&keys, &Some("missing".into())).is_err());
    }

    #[test]
    fn test_validated_claims_to_user_identity() {
        let claims = ValidatedClaims {
            subject: "sub-123".into(),
            issuer: "https://accounts.google.com".into(),
            audience: vec!["my-app".into()],
            email: Some("user@example.com".into()),
            name: Some("Test User".into()),
            tenant: Some("acme-corp".into()),
            expires_at: Some(9999999999),
            groups: vec!["admin".into()],
        };

        let user = claims.into_user_identity();
        assert_eq!(user.id, "oidc-https://accounts.google.com-sub-123");
        assert_eq!(user.name, "Test User");
        assert_eq!(user.tenant, Some("acme-corp".into()));
        assert!(!user.is_guest());
    }

    #[test]
    fn test_validated_claims_fallback_to_email() {
        let claims = ValidatedClaims {
            subject: "sub-456".into(),
            issuer: "https://accounts.google.com".into(),
            audience: vec![],
            email: Some("fallback@example.com".into()),
            name: None,
            tenant: None,
            expires_at: None,
            groups: vec![],
        };

        let user = claims.into_user_identity();
        assert_eq!(user.name, "fallback@example.com");
    }

    #[test]
    fn test_validated_claims_fallback_to_subject() {
        let claims = ValidatedClaims {
            subject: "sub-789".into(),
            issuer: "https://accounts.google.com".into(),
            audience: vec![],
            email: None,
            name: None,
            tenant: None,
            expires_at: None,
            groups: vec![],
        };

        let user = claims.into_user_identity();
        assert_eq!(user.name, "sub-789");
    }

    #[test]
    fn test_provider_config_serde() {
        let json = r#"{
            "issuer": "https://accounts.google.com",
            "audience": "my-app",
            "tenant_claim": "org_id",
            "group_claim": "groups",
            "required_groups": ["goose-users"]
        }"#;

        let config: OidcProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.issuer, "https://accounts.google.com");
        assert_eq!(config.tenant_claim, Some("org_id".into()));
        assert_eq!(config.required_groups, vec!["goose-users"]);
    }

    #[test]
    fn test_provider_config_minimal() {
        let json = r#"{
            "issuer": "https://accounts.google.com",
            "audience": "my-app"
        }"#;

        let config: OidcProviderConfig = serde_json::from_str(json).unwrap();
        assert!(config.tenant_claim.is_none());
        assert!(config.group_claim.is_none());
        assert!(config.required_groups.is_empty());
    }

    #[test]
    fn test_oidc_error_display() {
        let err = OidcError::UnknownIssuer("https://evil.com".into());
        assert!(err.to_string().contains("unknown issuer"));

        let err = OidcError::MissingSubject;
        assert!(err.to_string().contains("missing subject"));

        let err = OidcError::GroupAuthorizationFailed;
        assert!(err.to_string().contains("group authorization"));
    }

    #[test]
    fn test_provider_preset_from_name() {
        assert_eq!(
            OidcProviderPreset::from_name("google"),
            Some(OidcProviderPreset::Google)
        );
        assert_eq!(
            OidcProviderPreset::from_name("GitHub"),
            Some(OidcProviderPreset::GitHub)
        );
        assert_eq!(
            OidcProviderPreset::from_name("gh"),
            Some(OidcProviderPreset::GitHub)
        );
        assert_eq!(
            OidcProviderPreset::from_name("gitlab"),
            Some(OidcProviderPreset::GitLab)
        );
        assert_eq!(
            OidcProviderPreset::from_name("gl"),
            Some(OidcProviderPreset::GitLab)
        );
        assert_eq!(
            OidcProviderPreset::from_name("azure"),
            Some(OidcProviderPreset::Azure)
        );
        assert_eq!(
            OidcProviderPreset::from_name("microsoft"),
            Some(OidcProviderPreset::Azure)
        );
        assert_eq!(
            OidcProviderPreset::from_name("aws"),
            Some(OidcProviderPreset::Aws)
        );
        assert_eq!(
            OidcProviderPreset::from_name("cognito"),
            Some(OidcProviderPreset::Aws)
        );
        assert_eq!(
            OidcProviderPreset::from_name("auth0"),
            Some(OidcProviderPreset::Auth0)
        );
        assert_eq!(
            OidcProviderPreset::from_name("okta"),
            Some(OidcProviderPreset::Okta)
        );
        assert_eq!(OidcProviderPreset::from_name("unknown"), None);
    }

    #[test]
    fn test_provider_preset_discovery_urls() {
        let url = OidcProviderPreset::Google.discovery_url(None);
        assert!(url.contains("accounts.google.com"));

        let url = OidcProviderPreset::Azure.discovery_url(Some("my-tenant-id"));
        assert!(url.contains("my-tenant-id"));
        assert!(url.contains("login.microsoftonline.com"));

        let url = OidcProviderPreset::Azure.discovery_url(None);
        assert!(url.contains("common"));

        let url = OidcProviderPreset::GitLab.discovery_url(None);
        assert!(url.contains("gitlab.com"));

        let url = OidcProviderPreset::GitLab.discovery_url(Some("gitlab.acme.com"));
        assert!(url.contains("gitlab.acme.com"));

        let url = OidcProviderPreset::Aws.discovery_url(Some("us-west-2_abc123"));
        assert!(url.contains("cognito-idp.us-west-2.amazonaws.com"));
        assert!(url.contains("us-west-2_abc123"));

        let url = OidcProviderPreset::Auth0.discovery_url(Some("mycompany.auth0.com"));
        assert!(url.contains("mycompany.auth0.com"));

        let url = OidcProviderPreset::Okta.discovery_url(Some("dev-123.okta.com"));
        assert!(url.contains("dev-123.okta.com"));
    }

    #[test]
    fn test_provider_preset_github_uses_oauth2() {
        assert!(!OidcProviderPreset::GitHub.supports_oidc_code_flow());
        assert!(OidcProviderPreset::GitHub.oauth2_authorize_url().is_some());
        assert!(OidcProviderPreset::GitHub.oauth2_token_url().is_some());
    }

    #[test]
    fn test_provider_preset_standard_oidc() {
        for preset in [
            OidcProviderPreset::Google,
            OidcProviderPreset::Azure,
            OidcProviderPreset::GitLab,
            OidcProviderPreset::Aws,
            OidcProviderPreset::Auth0,
            OidcProviderPreset::Okta,
        ] {
            assert!(
                preset.supports_oidc_code_flow(),
                "{} should support OIDC",
                preset
            );
            assert!(
                preset.oauth2_authorize_url().is_none(),
                "{} shouldn't need OAuth2 fallback",
                preset
            );
        }
    }

    #[test]
    fn test_provider_preset_all() {
        let all = OidcProviderPreset::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_provider_preset_display() {
        assert_eq!(OidcProviderPreset::Google.to_string(), "Google");
        assert_eq!(
            OidcProviderPreset::Azure.to_string(),
            "Microsoft Azure AD / Entra ID"
        );
        assert_eq!(OidcProviderPreset::GitHub.to_string(), "GitHub");
        assert_eq!(OidcProviderPreset::GitLab.to_string(), "GitLab");
        assert_eq!(OidcProviderPreset::Aws.to_string(), "AWS Cognito");
    }

    #[test]
    fn test_provider_preset_serde() {
        let preset = OidcProviderPreset::Google;
        let json = serde_json::to_string(&preset).unwrap();
        assert_eq!(json, "\"google\"");
        let back: OidcProviderPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(back, OidcProviderPreset::Google);
    }

    #[test]
    fn test_provider_preset_scopes() {
        assert!(OidcProviderPreset::Google
            .default_scopes()
            .contains("openid"));
        assert!(OidcProviderPreset::Google
            .default_scopes()
            .contains("email"));
        // GitHub only needs openid for Actions tokens
        assert_eq!(OidcProviderPreset::GitHub.default_scopes(), "openid");
    }
}
