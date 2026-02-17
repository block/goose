use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::identity::UserIdentity;

const DEFAULT_TOKEN_TTL_SECS: u64 = 86400; // 24 hours

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    pub sub: String,
    pub name: String,
    pub auth_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    pub iat: u64,
    pub exp: u64,
    pub jti: String,
}

impl SessionClaims {
    pub fn from_user(user: &UserIdentity, ttl_secs: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            sub: user.id.clone(),
            name: user.name.clone(),
            auth_method: format!("{:?}", user.auth_method),
            tenant: user.tenant.clone(),
            iat: now,
            exp: now + ttl_secs,
            jti: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn into_user_identity(&self) -> UserIdentity {
        let mut user = UserIdentity::guest_stable(&self.sub);
        user.name = self.name.clone();
        user.tenant = self.tenant.clone();
        user
    }
}

#[derive(Clone)]
pub struct SessionTokenStore {
    signing_key: Arc<String>,
    revoked: Arc<RwLock<HashSet<String>>>,
    ttl_secs: u64,
}

impl SessionTokenStore {
    pub fn new(signing_key: impl Into<String>) -> Self {
        Self {
            signing_key: Arc::new(signing_key.into()),
            revoked: Arc::new(RwLock::new(HashSet::new())),
            ttl_secs: DEFAULT_TOKEN_TTL_SECS,
        }
    }

    pub fn ttl(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.ttl_secs)
    }

    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = ttl_secs;
        self
    }

    pub fn issue_token(&self, user: &UserIdentity) -> Result<String, SessionTokenError> {
        let claims = SessionClaims::from_user(user, self.ttl_secs);
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.signing_key.as_bytes()),
        )
        .map_err(|e| SessionTokenError::EncodingFailed(e.to_string()))?;
        Ok(token)
    }

    pub async fn validate_token(&self, token: &str) -> Result<SessionClaims, SessionTokenError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.required_spec_claims = HashSet::from(["exp".into(), "sub".into()]);

        let token_data = decode::<SessionClaims>(
            token,
            &DecodingKey::from_secret(self.signing_key.as_bytes()),
            &validation,
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => SessionTokenError::Expired,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                SessionTokenError::InvalidSignature
            }
            _ => SessionTokenError::ValidationFailed(e.to_string()),
        })?;

        let revoked = self.revoked.read().await;
        if revoked.contains(&token_data.claims.jti) {
            return Err(SessionTokenError::Revoked);
        }

        Ok(token_data.claims)
    }

    pub async fn revoke_token(&self, jti: &str) {
        let mut revoked = self.revoked.write().await;
        revoked.insert(jti.to_string());
    }

    pub async fn revoke_by_token(&self, token: &str) -> Result<(), SessionTokenError> {
        let claims = self.validate_token(token).await?;
        self.revoke_token(&claims.jti).await;
        Ok(())
    }

    pub async fn cleanup_expired(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut revoked = self.revoked.write().await;
        // We can't check expiry of revoked tokens without re-decoding them,
        // so we just cap the revocation list size
        if revoked.len() > 10_000 {
            revoked.clear();
        }
        let _ = now; // suppress unused warning
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SessionTokenError {
    #[error("token encoding failed: {0}")]
    EncodingFailed(String),
    #[error("token expired")]
    Expired,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("token revoked")]
    Revoked,
    #[error("validation failed: {0}")]
    ValidationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_user() -> UserIdentity {
        UserIdentity::guest_stable("test-user-123")
    }

    fn oidc_user() -> UserIdentity {
        UserIdentity::oidc("sub-456", "Alice Smith", "google").with_tenant("acme-corp".to_string())
    }

    #[test]
    fn test_issue_and_validate_sync() {
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!");
        let token = store.issue_token(&test_user()).unwrap();
        assert!(!token.is_empty());
        // Token should have 3 parts (header.payload.signature)
        assert_eq!(token.split('.').count(), 3);
    }

    #[tokio::test]
    async fn test_issue_and_validate() {
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!");
        let user = test_user();
        let token = store.issue_token(&user).unwrap();

        let claims = store.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.name, user.name);
        assert!(claims.tenant.is_none());
    }

    #[tokio::test]
    async fn test_oidc_user_roundtrip() {
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!");
        let user = oidc_user();
        let token = store.issue_token(&user).unwrap();

        let claims = store.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.name, "Alice Smith");
        assert_eq!(claims.tenant, Some("acme-corp".to_string()));
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!");
        let token = store.issue_token(&test_user()).unwrap();

        // Valid before revocation
        assert!(store.validate_token(&token).await.is_ok());

        // Revoke
        store.revoke_by_token(&token).await.unwrap();

        // Invalid after revocation
        let err = store.validate_token(&token).await.unwrap_err();
        assert!(matches!(err, SessionTokenError::Revoked));
    }

    #[tokio::test]
    async fn test_expired_token() {
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!");
        // Manually create a token with exp in the past
        let mut claims = SessionClaims::from_user(&test_user(), 0);
        claims.iat = 1_000_000;
        claims.exp = 1_000_001; // long expired

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"test-secret-key-32chars-long!!!!"),
        )
        .unwrap();

        let err = store.validate_token(&token).await.unwrap_err();
        assert!(matches!(err, SessionTokenError::Expired));
    }

    #[tokio::test]
    async fn test_invalid_signature() {
        let store1 = SessionTokenStore::new("secret-key-one-32chars-long!!!!!");
        let store2 = SessionTokenStore::new("secret-key-two-32chars-long!!!!!");

        let token = store1.issue_token(&test_user()).unwrap();
        let err = store2.validate_token(&token).await.unwrap_err();
        assert!(matches!(err, SessionTokenError::InvalidSignature));
    }

    #[tokio::test]
    async fn test_claims_to_user_identity() {
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!");
        let user = oidc_user();
        let token = store.issue_token(&user).unwrap();

        let claims = store.validate_token(&token).await.unwrap();
        let restored = claims.into_user_identity();
        assert_eq!(restored.id, user.id);
        assert_eq!(restored.name, "Alice Smith");
        assert_eq!(restored.tenant, Some("acme-corp".to_string()));
    }

    #[tokio::test]
    async fn test_custom_ttl() {
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!").with_ttl(3600);
        let token = store.issue_token(&test_user()).unwrap();

        let claims = store.validate_token(&token).await.unwrap();
        assert_eq!(claims.exp - claims.iat, 3600);
    }

    #[test]
    fn test_jti_uniqueness() {
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!");
        let user = test_user();
        let t1 = store.issue_token(&user).unwrap();
        let t2 = store.issue_token(&user).unwrap();
        // Different tokens for same user (different jti)
        assert_ne!(t1, t2);
    }
}
