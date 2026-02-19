use std::collections::HashMap;
use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use tokio::sync::{OnceCell, RwLock};

use super::{PairingState, PlatformUser};

pub struct PairingStore {
    pairings: RwLock<HashMap<PlatformUser, PairingState>>,
    pool: Pool<Sqlite>,
    initialized: OnceCell<()>,
}

impl PairingStore {
    pub fn new(db_path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .busy_timeout(std::time::Duration::from_secs(30))
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new().connect_lazy_with(options);

        Ok(Self {
            pairings: RwLock::new(HashMap::new()),
            pool,
            initialized: OnceCell::new(),
        })
    }

    async fn ensure_initialized(&self) -> anyhow::Result<&Pool<Sqlite>> {
        self.initialized
            .get_or_try_init(|| async {
                self.create_schema().await?;
                self.load_all().await?;
                Ok::<(), anyhow::Error>(())
            })
            .await?;
        Ok(&self.pool)
    }

    async fn create_schema(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS gateway_pairings (
                platform TEXT NOT NULL,
                user_id TEXT NOT NULL,
                display_name TEXT,
                state TEXT NOT NULL,
                session_id TEXT,
                code TEXT,
                expires_at INTEGER,
                paired_at INTEGER,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (platform, user_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS gateway_pending_codes (
                code TEXT PRIMARY KEY,
                gateway_type TEXT NOT NULL,
                expires_at INTEGER NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load_all(&self) -> anyhow::Result<()> {
        let rows = sqlx::query_as::<_, PairingRow>(
            "SELECT platform, user_id, display_name, state, session_id, code, expires_at, paired_at FROM gateway_pairings",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut pairings = self.pairings.write().await;
        for row in rows {
            let user = PlatformUser {
                platform: row.platform.clone(),
                user_id: row.user_id.clone(),
                display_name: row.display_name.clone(),
            };
            let state = row.into_pairing_state();
            pairings.insert(user, state);
        }
        Ok(())
    }

    pub async fn get(&self, user: &PlatformUser) -> anyhow::Result<PairingState> {
        self.ensure_initialized().await?;
        let pairings = self.pairings.read().await;
        Ok(pairings
            .get(user)
            .cloned()
            .unwrap_or(PairingState::Unpaired))
    }

    pub async fn set(&self, user: &PlatformUser, state: PairingState) -> anyhow::Result<()> {
        let pool = self.ensure_initialized().await?;

        let (state_str, session_id, code, expires_at, paired_at) = match &state {
            PairingState::Unpaired => ("unpaired", None, None, None, None),
            PairingState::PendingCode { code, expires_at } => (
                "pending_code",
                None,
                Some(code.clone()),
                Some(*expires_at),
                None,
            ),
            PairingState::Paired {
                session_id,
                paired_at,
            } => (
                "paired",
                Some(session_id.clone()),
                None,
                None,
                Some(*paired_at),
            ),
        };

        sqlx::query(
            r#"
            INSERT INTO gateway_pairings (platform, user_id, display_name, state, session_id, code, expires_at, paired_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(platform, user_id) DO UPDATE SET
                display_name = excluded.display_name,
                state = excluded.state,
                session_id = excluded.session_id,
                code = excluded.code,
                expires_at = excluded.expires_at,
                paired_at = excluded.paired_at,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&user.platform)
        .bind(&user.user_id)
        .bind(&user.display_name)
        .bind(state_str)
        .bind(session_id)
        .bind(code)
        .bind(expires_at)
        .bind(paired_at)
        .execute(pool)
        .await?;

        self.pairings.write().await.insert(user.clone(), state);
        Ok(())
    }

    pub async fn remove(&self, user: &PlatformUser) -> anyhow::Result<()> {
        let pool = self.ensure_initialized().await?;

        sqlx::query("DELETE FROM gateway_pairings WHERE platform = ? AND user_id = ?")
            .bind(&user.platform)
            .bind(&user.user_id)
            .execute(pool)
            .await?;

        self.pairings.write().await.remove(user);
        Ok(())
    }

    pub async fn store_pending_code(
        &self,
        code: &str,
        gateway_type: &str,
        expires_at: i64,
    ) -> anyhow::Result<()> {
        let pool = self.ensure_initialized().await?;

        sqlx::query(
            r#"
            INSERT INTO gateway_pending_codes (code, gateway_type, expires_at)
            VALUES (?, ?, ?)
            ON CONFLICT(code) DO UPDATE SET
                gateway_type = excluded.gateway_type,
                expires_at = excluded.expires_at
            "#,
        )
        .bind(code)
        .bind(gateway_type)
        .bind(expires_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn consume_pending_code(&self, code: &str) -> anyhow::Result<Option<String>> {
        let pool = self.ensure_initialized().await?;

        let row = sqlx::query_as::<_, PendingCodeRow>(
            "SELECT code, gateway_type, expires_at FROM gateway_pending_codes WHERE code = ?",
        )
        .bind(code)
        .fetch_optional(pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let now = chrono::Utc::now().timestamp();
        if now > row.expires_at {
            sqlx::query("DELETE FROM gateway_pending_codes WHERE code = ?")
                .bind(code)
                .execute(pool)
                .await?;
            return Ok(None);
        }

        sqlx::query("DELETE FROM gateway_pending_codes WHERE code = ?")
            .bind(code)
            .execute(pool)
            .await?;

        Ok(Some(row.gateway_type))
    }

    #[allow(dead_code)]
    pub async fn cleanup_expired_codes(&self) -> anyhow::Result<()> {
        let pool = self.ensure_initialized().await?;
        let now = chrono::Utc::now().timestamp();

        sqlx::query("DELETE FROM gateway_pending_codes WHERE expires_at < ?")
            .bind(now)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub fn generate_code() -> String {
        use rand::Rng;
        let chars: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::thread_rng();
        (0..6)
            .map(|_| chars[rng.gen_range(0..chars.len())] as char)
            .collect()
    }

    pub async fn remove_all_for_platform(&self, platform: &str) -> anyhow::Result<usize> {
        let pool = self.ensure_initialized().await?;

        let result = sqlx::query("DELETE FROM gateway_pairings WHERE platform = ?")
            .bind(platform)
            .execute(pool)
            .await?;

        let mut pairings = self.pairings.write().await;
        pairings.retain(|user, _| user.platform != platform);

        Ok(result.rows_affected() as usize)
    }

    pub async fn list_paired_users(
        &self,
        gateway_type: &str,
    ) -> anyhow::Result<Vec<(PlatformUser, String, i64)>> {
        self.ensure_initialized().await?;
        let pairings = self.pairings.read().await;
        let mut result = Vec::new();
        for (user, state) in pairings.iter() {
            if user.platform == gateway_type {
                if let PairingState::Paired {
                    session_id,
                    paired_at,
                } = state
                {
                    result.push((user.clone(), session_id.clone(), *paired_at));
                }
            }
        }
        Ok(result)
    }
}

#[derive(sqlx::FromRow)]
struct PairingRow {
    platform: String,
    user_id: String,
    display_name: Option<String>,
    state: String,
    session_id: Option<String>,
    code: Option<String>,
    expires_at: Option<i64>,
    paired_at: Option<i64>,
}

impl PairingRow {
    fn into_pairing_state(self) -> PairingState {
        match self.state.as_str() {
            "pending_code" => PairingState::PendingCode {
                code: self.code.unwrap_or_default(),
                expires_at: self.expires_at.unwrap_or(0),
            },
            "paired" => PairingState::Paired {
                session_id: self.session_id.unwrap_or_default(),
                paired_at: self.paired_at.unwrap_or(0),
            },
            _ => PairingState::Unpaired,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PendingCodeRow {
    #[allow(dead_code)]
    code: String,
    gateway_type: String,
    expires_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_user(platform: &str, id: &str) -> PlatformUser {
        PlatformUser {
            platform: platform.to_string(),
            user_id: id.to_string(),
            display_name: None,
        }
    }

    #[tokio::test]
    async fn test_pairing_lifecycle() {
        let tmp = TempDir::new().unwrap();
        let store = PairingStore::new(&tmp.path().join("test.db")).unwrap();
        let user = test_user("telegram", "12345");

        let state = store.get(&user).await.unwrap();
        assert!(matches!(state, PairingState::Unpaired));

        store
            .set(
                &user,
                PairingState::PendingCode {
                    code: "ABC123".to_string(),
                    expires_at: chrono::Utc::now().timestamp() + 300,
                },
            )
            .await
            .unwrap();

        let state = store.get(&user).await.unwrap();
        assert!(matches!(state, PairingState::PendingCode { .. }));

        store
            .set(
                &user,
                PairingState::Paired {
                    session_id: "session-1".to_string(),
                    paired_at: chrono::Utc::now().timestamp(),
                },
            )
            .await
            .unwrap();

        let state = store.get(&user).await.unwrap();
        assert!(matches!(state, PairingState::Paired { .. }));
    }

    #[tokio::test]
    async fn test_pending_code_flow() {
        let tmp = TempDir::new().unwrap();
        let store = PairingStore::new(&tmp.path().join("test.db")).unwrap();

        let expires = chrono::Utc::now().timestamp() + 300;
        store
            .store_pending_code("XYZW99", "telegram", expires)
            .await
            .unwrap();

        let gw = store.consume_pending_code("XYZW99").await.unwrap();
        assert_eq!(gw, Some("telegram".to_string()));

        let gw = store.consume_pending_code("XYZW99").await.unwrap();
        assert_eq!(gw, None);
    }

    #[tokio::test]
    async fn test_expired_code() {
        let tmp = TempDir::new().unwrap();
        let store = PairingStore::new(&tmp.path().join("test.db")).unwrap();

        let expired = chrono::Utc::now().timestamp() - 10;
        store
            .store_pending_code("OLD123", "telegram", expired)
            .await
            .unwrap();

        let gw = store.consume_pending_code("OLD123").await.unwrap();
        assert_eq!(gw, None);
    }

    #[test]
    fn test_code_generation() {
        let code = PairingStore::generate_code();
        assert_eq!(code.len(), 6);
        assert!(code
            .chars()
            .all(|c| "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(c)));
    }

    #[tokio::test]
    async fn test_persistence_across_instances() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("persist.db");
        let user = test_user("discord", "user42");

        {
            let store = PairingStore::new(&db_path).unwrap();
            store
                .set(
                    &user,
                    PairingState::Paired {
                        session_id: "s-42".to_string(),
                        paired_at: 1000,
                    },
                )
                .await
                .unwrap();
        }

        let store2 = PairingStore::new(&db_path).unwrap();
        let state = store2.get(&user).await.unwrap();
        match state {
            PairingState::Paired { session_id, .. } => assert_eq!(session_id, "s-42"),
            other => panic!("Expected Paired, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_remove_all_for_platform() {
        let tmp = TempDir::new().unwrap();
        let store = PairingStore::new(&tmp.path().join("test.db")).unwrap();

        let tg1 = test_user("telegram", "111");
        let tg2 = test_user("telegram", "222");
        let discord = test_user("discord", "333");

        for user in [&tg1, &tg2, &discord] {
            store
                .set(
                    user,
                    PairingState::Paired {
                        session_id: format!("s-{}", user.user_id),
                        paired_at: 1000,
                    },
                )
                .await
                .unwrap();
        }

        let removed = store.remove_all_for_platform("telegram").await.unwrap();
        assert_eq!(removed, 2);

        assert!(matches!(
            store.get(&tg1).await.unwrap(),
            PairingState::Unpaired
        ));
        assert!(matches!(
            store.get(&tg2).await.unwrap(),
            PairingState::Unpaired
        ));
        assert!(matches!(
            store.get(&discord).await.unwrap(),
            PairingState::Paired { .. }
        ));
    }
}
