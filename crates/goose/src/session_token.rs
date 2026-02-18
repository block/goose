use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};

use crate::identity::UserIdentity;

const DEFAULT_TOKEN_TTL_SECS: u64 = 86400; // 24 hours
const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionClaims {
    pub sub: String,
    pub name: String,
    pub auth_method: String,
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
    ttl_secs: u64,
    /// SQLite pool for persistent storage (lazy-initialized)
    pool: Arc<OnceCell<Pool<Sqlite>>>,
    db_path: PathBuf,
    /// In-memory fallback for revoked tokens (used before DB is ready)
    revoked_fallback: Arc<RwLock<HashSet<String>>>,
}

impl SessionTokenStore {
    /// Create a new store with SQLite persistence at the given data directory.
    pub fn new(signing_key: impl Into<String>, data_dir: &Path) -> Self {
        let auth_dir = data_dir.join("auth");
        std::fs::create_dir_all(&auth_dir).ok();
        let db_path = auth_dir.join("tokens.db");

        Self {
            signing_key: Arc::new(signing_key.into()),
            ttl_secs: DEFAULT_TOKEN_TTL_SECS,
            pool: Arc::new(OnceCell::new()),
            db_path,
            revoked_fallback: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Create an in-memory store (for tests or when no persistence is needed).
    pub fn in_memory(signing_key: impl Into<String>) -> Self {
        Self {
            signing_key: Arc::new(signing_key.into()),
            ttl_secs: DEFAULT_TOKEN_TTL_SECS,
            pool: Arc::new(OnceCell::new()),
            db_path: PathBuf::from(":memory:"),
            revoked_fallback: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn ttl(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.ttl_secs)
    }

    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = ttl_secs;
        self
    }

    async fn pool(&self) -> Result<&Pool<Sqlite>, SessionTokenError> {
        self.pool
            .get_or_try_init(|| async {
                let pool = Self::create_pool(&self.db_path);
                Self::run_migrations(&pool).await?;
                Ok::<Pool<Sqlite>, SessionTokenError>(pool)
            })
            .await
    }

    fn create_pool(path: &Path) -> Pool<Sqlite> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        SqlitePoolOptions::new()
            .max_connections(2)
            .connect_lazy_with(options)
    }

    async fn run_migrations(pool: &Pool<Sqlite>) -> Result<(), SessionTokenError> {
        // Check if schema_version table exists
        let has_schema: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version')"#,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| SessionTokenError::StorageError(e.to_string()))?;

        let current_version = if has_schema {
            sqlx::query_scalar::<_, i64>("SELECT version FROM schema_version")
                .fetch_optional(pool)
                .await
                .map_err(|e| SessionTokenError::StorageError(e.to_string()))?
                .unwrap_or(0)
        } else {
            0
        };

        if current_version < SCHEMA_VERSION {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS schema_version (
                    version INTEGER NOT NULL
                );
                "#,
            )
            .execute(pool)
            .await
            .map_err(|e| SessionTokenError::StorageError(e.to_string()))?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS revoked_tokens (
                    jti TEXT PRIMARY KEY,
                    revoked_at INTEGER NOT NULL
                );
                "#,
            )
            .execute(pool)
            .await
            .map_err(|e| SessionTokenError::StorageError(e.to_string()))?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS refresh_tokens (
                    issuer TEXT NOT NULL,
                    subject TEXT NOT NULL,
                    refresh_token TEXT NOT NULL,
                    stored_at INTEGER NOT NULL,
                    PRIMARY KEY (issuer, subject)
                );
                "#,
            )
            .execute(pool)
            .await
            .map_err(|e| SessionTokenError::StorageError(e.to_string()))?;

            if current_version == 0 {
                sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
                    .bind(SCHEMA_VERSION)
                    .execute(pool)
                    .await
                    .map_err(|e| SessionTokenError::StorageError(e.to_string()))?;
            } else {
                sqlx::query("UPDATE schema_version SET version = ?")
                    .bind(SCHEMA_VERSION)
                    .execute(pool)
                    .await
                    .map_err(|e| SessionTokenError::StorageError(e.to_string()))?;
            }
        }

        Ok(())
    }

    // --- JWT Token Operations ---

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

        // Check persistent revocation store
        if self.is_token_revoked(&token_data.claims.jti).await? {
            return Err(SessionTokenError::Revoked);
        }

        Ok(token_data.claims)
    }

    pub async fn revoke_token(&self, jti: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if let Ok(pool) = self.pool().await {
            sqlx::query("INSERT OR REPLACE INTO revoked_tokens (jti, revoked_at) VALUES (?, ?)")
                .bind(jti)
                .bind(now)
                .execute(pool)
                .await
                .ok();
        } else {
            // Fallback to in-memory
            let mut revoked = self.revoked_fallback.write().await;
            revoked.insert(jti.to_string());
        }
    }

    pub async fn revoke_by_token(&self, token: &str) -> Result<(), SessionTokenError> {
        let claims = self.validate_token(token).await?;
        self.revoke_token(&claims.jti).await;
        Ok(())
    }

    async fn is_token_revoked(&self, jti: &str) -> Result<bool, SessionTokenError> {
        // Check DB first
        if let Ok(pool) = self.pool().await {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM revoked_tokens WHERE jti = ?)")
                    .bind(jti)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| SessionTokenError::StorageError(e.to_string()))?;
            return Ok(exists);
        }

        // Fallback to in-memory
        let revoked = self.revoked_fallback.read().await;
        Ok(revoked.contains(jti))
    }

    /// Clean up expired revocation entries (older than 48h — well past any token's TTL).
    pub async fn cleanup_expired(&self) {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            - 172800; // 48 hours

        if let Ok(pool) = self.pool().await {
            sqlx::query("DELETE FROM revoked_tokens WHERE revoked_at < ?")
                .bind(cutoff)
                .execute(pool)
                .await
                .ok();
        }
    }

    // --- Refresh Token Operations ---

    pub async fn store_refresh_token(&self, issuer: &str, subject: &str, refresh_token: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if let Ok(pool) = self.pool().await {
            sqlx::query(
                "INSERT OR REPLACE INTO refresh_tokens (issuer, subject, refresh_token, stored_at) VALUES (?, ?, ?, ?)",
            )
            .bind(issuer)
            .bind(subject)
            .bind(refresh_token)
            .bind(now)
            .execute(pool)
            .await
            .ok();
        }
    }

    pub async fn get_refresh_token(&self, issuer: &str, subject: &str) -> Option<String> {
        if let Ok(pool) = self.pool().await {
            sqlx::query_scalar(
                "SELECT refresh_token FROM refresh_tokens WHERE issuer = ? AND subject = ?",
            )
            .bind(issuer)
            .bind(subject)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
        } else {
            None
        }
    }

    pub async fn remove_refresh_token(&self, issuer: &str, subject: &str) {
        if let Ok(pool) = self.pool().await {
            sqlx::query("DELETE FROM refresh_tokens WHERE issuer = ? AND subject = ?")
                .bind(issuer)
                .bind(subject)
                .execute(pool)
                .await
                .ok();
        }
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
    #[error("storage error: {0}")]
    StorageError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_user() -> UserIdentity {
        UserIdentity::guest_stable("test-user-123")
    }

    fn oidc_user() -> UserIdentity {
        UserIdentity::oidc("sub-456", "Alice Smith", "google").with_tenant("acme-corp".to_string())
    }

    fn create_test_store() -> (SessionTokenStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!", dir.path());
        (store, dir)
    }

    #[test]
    fn test_issue_token_sync() {
        let dir = TempDir::new().unwrap();
        let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!", dir.path());
        let token = store.issue_token(&test_user()).unwrap();
        assert!(!token.is_empty());
        assert_eq!(token.split('.').count(), 3);
    }

    #[tokio::test]
    async fn test_issue_and_validate() {
        let (store, _dir) = create_test_store();
        let user = test_user();
        let token = store.issue_token(&user).unwrap();

        let claims = store.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.name, user.name);
        assert!(claims.tenant.is_none());
    }

    #[tokio::test]
    async fn test_oidc_user_roundtrip() {
        let (store, _dir) = create_test_store();
        let user = oidc_user();
        let token = store.issue_token(&user).unwrap();

        let claims = store.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.name, "Alice Smith");
        assert_eq!(claims.tenant, Some("acme-corp".to_string()));
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let (store, _dir) = create_test_store();
        let token = store.issue_token(&test_user()).unwrap();

        assert!(store.validate_token(&token).await.is_ok());

        store.revoke_by_token(&token).await.unwrap();

        let err = store.validate_token(&token).await.unwrap_err();
        assert!(matches!(err, SessionTokenError::Revoked));
    }

    #[tokio::test]
    async fn test_revoke_persists() {
        let dir = TempDir::new().unwrap();
        let token;

        // Issue and revoke with first store instance
        {
            let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!", dir.path());
            token = store.issue_token(&test_user()).unwrap();
            store.revoke_by_token(&token).await.unwrap();
        }

        // Create a new store pointing to the same DB — revocation should persist
        {
            let store2 = SessionTokenStore::new("test-secret-key-32chars-long!!!!", dir.path());
            let err = store2.validate_token(&token).await.unwrap_err();
            assert!(matches!(err, SessionTokenError::Revoked));
        }
    }

    #[tokio::test]
    async fn test_expired_token() {
        let (store, _dir) = create_test_store();
        let mut claims = SessionClaims::from_user(&test_user(), 0);
        claims.iat = 1_000_000;
        claims.exp = 1_000_001;

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
        let (store, _dir) = create_test_store();
        let claims = SessionClaims::from_user(&test_user(), 3600);

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"wrong-key-that-does-not-match!!!"),
        )
        .unwrap();

        let err = store.validate_token(&token).await.unwrap_err();
        assert!(matches!(err, SessionTokenError::InvalidSignature));
    }

    #[tokio::test]
    async fn test_custom_ttl() {
        let dir = TempDir::new().unwrap();
        let store =
            SessionTokenStore::new("test-secret-key-32chars-long!!!!", dir.path()).with_ttl(7200);
        assert_eq!(store.ttl().as_secs(), 7200);

        let token = store.issue_token(&test_user()).unwrap();
        let claims = store.validate_token(&token).await.unwrap();
        assert!(claims.exp - claims.iat == 7200);
    }

    #[tokio::test]
    async fn test_jti_uniqueness() {
        let (store, _dir) = create_test_store();
        let user = test_user();
        let token1 = store.issue_token(&user).unwrap();
        let token2 = store.issue_token(&user).unwrap();

        let c1 = store.validate_token(&token1).await.unwrap();
        let c2 = store.validate_token(&token2).await.unwrap();
        assert_ne!(c1.jti, c2.jti);
    }

    #[tokio::test]
    async fn test_refresh_token_store_and_retrieve() {
        let (store, _dir) = create_test_store();

        store
            .store_refresh_token("https://accounts.google.com", "user-123", "rt-abc-def")
            .await;

        let rt = store
            .get_refresh_token("https://accounts.google.com", "user-123")
            .await;
        assert_eq!(rt, Some("rt-abc-def".to_string()));

        // Different subject returns None
        let rt2 = store
            .get_refresh_token("https://accounts.google.com", "user-999")
            .await;
        assert_eq!(rt2, None);

        // Remove and verify
        store
            .remove_refresh_token("https://accounts.google.com", "user-123")
            .await;
        let rt3 = store
            .get_refresh_token("https://accounts.google.com", "user-123")
            .await;
        assert_eq!(rt3, None);
    }

    #[tokio::test]
    async fn test_refresh_token_persists() {
        let dir = TempDir::new().unwrap();

        // Store with first instance
        {
            let store = SessionTokenStore::new("test-secret-key-32chars-long!!!!", dir.path());
            store
                .store_refresh_token("https://accounts.google.com", "user-123", "rt-persist")
                .await;
        }

        // Read with new instance — should persist
        {
            let store2 = SessionTokenStore::new("test-secret-key-32chars-long!!!!", dir.path());
            let rt = store2
                .get_refresh_token("https://accounts.google.com", "user-123")
                .await;
            assert_eq!(rt, Some("rt-persist".to_string()));
        }
    }

    #[tokio::test]
    async fn test_cleanup_expired_revocations() {
        let (store, _dir) = create_test_store();

        // Manually insert an old revocation
        if let Ok(pool) = store.pool().await {
            sqlx::query("INSERT INTO revoked_tokens (jti, revoked_at) VALUES (?, ?)")
                .bind("old-jti")
                .bind(1_000_000i64) // very old
                .execute(pool)
                .await
                .unwrap();
        }

        // Should exist before cleanup
        assert!(store.is_token_revoked("old-jti").await.unwrap());

        // Cleanup removes old entries
        store.cleanup_expired().await;

        // Should be gone
        assert!(!store.is_token_revoked("old-jti").await.unwrap());
    }

    #[tokio::test]
    async fn test_in_memory_store() {
        let store = SessionTokenStore::in_memory("test-secret");
        let token = store.issue_token(&test_user()).unwrap();
        let claims = store.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, "test-user-123");
    }
}
