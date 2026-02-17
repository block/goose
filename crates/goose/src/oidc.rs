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

/// Configuration for a single OIDC provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcProviderConfig {
    pub issuer: String,
    pub audience: String,
    #[serde(default)]
    pub tenant_claim: Option<String>,
    #[serde(default)]
    pub group_claim: Option<String>,
    #[serde(default)]
    pub required_groups: Vec<String>,
}

/// OIDC discovery document (subset of fields we need).
#[derive(Debug, Clone, Deserialize)]
struct OidcDiscovery {
    issuer: String,
    jwks_uri: String,
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
                required_groups: vec![],
            },
            OidcProviderConfig {
                issuer: "https://login.microsoftonline.com/tenant-id/v2.0".into(),
                audience: "my-app".into(),
                tenant_claim: Some("tid".into()),
                group_claim: Some("groups".into()),
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
}
