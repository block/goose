use anyhow::Result;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::OnceCell;

use super::migrations::run_migrations;
use super::session_storage::SessionStorage;
use super::DB_NAME;

static STORAGE_MANAGER: OnceCell<Arc<StorageManager>> = OnceCell::const_new();

pub struct StorageManager {
    pool: Pool<Sqlite>,
}

impl StorageManager {
    pub async fn instance() -> Result<Arc<StorageManager>> {
        STORAGE_MANAGER
            .get_or_try_init(|| async { Self::new().await.map(Arc::new) })
            .await
            .map(Arc::clone)
    }

    async fn new() -> Result<Self> {
        let session_dir = crate::storage::session_storage::ensure_session_dir()?;
        let db_path = session_dir.join(DB_NAME);

        let is_new_database = !db_path.exists();

        let pool = if is_new_database {
            Self::create_database(&db_path).await?
        } else {
            Self::open_database(&db_path).await?
        };

        let manager = Self { pool: pool.clone() };

        // Import legacy sessions if this is a new database
        if is_new_database {
            if let Err(e) = crate::session::SessionManager::import_legacy_sessions(&session_dir, &pool).await {
                tracing::warn!("Failed to import some legacy sessions: {}", e);
            }
        }

        Ok(manager)
    }

    async fn open_database(db_path: &Path) -> Result<Pool<Sqlite>> {
        let pool = Self::get_pool(db_path, false).await?;
        run_migrations(&pool).await?;
        Ok(pool)
    }

    async fn create_database(db_path: &Path) -> Result<Pool<Sqlite>> {
        let pool = Self::get_pool(db_path, true).await?;

        sqlx::query(
            r#"
            CREATE TABLE schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
            .bind(super::migrations::CURRENT_SCHEMA_VERSION)
            .execute(&pool)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                user_set_name BOOLEAN DEFAULT FALSE,
                session_type TEXT NOT NULL DEFAULT 'user',
                working_dir TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                extension_data TEXT DEFAULT '{}',
                total_tokens INTEGER,
                input_tokens INTEGER,
                output_tokens INTEGER,
                accumulated_total_tokens INTEGER,
                accumulated_input_tokens INTEGER,
                accumulated_output_tokens INTEGER,
                schedule_id TEXT,
                recipe_json TEXT,
                user_recipe_values_json TEXT,
                provider_name TEXT,
                model_config_json TEXT
            )
        "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES sessions(id),
                role TEXT NOT NULL,
                content_json TEXT NOT NULL,
                created_timestamp INTEGER NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                tokens INTEGER,
                metadata_json TEXT
            )
        "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query("CREATE INDEX idx_messages_session ON messages(session_id)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX idx_messages_timestamp ON messages(timestamp)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX idx_sessions_updated ON sessions(updated_at DESC)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX idx_sessions_type ON sessions(session_type)")
            .execute(&pool)
            .await?;

        Ok(pool)
    }

    async fn get_pool(db_path: &Path, create_if_missing: bool) -> Result<Pool<Sqlite>> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(create_if_missing)
            .busy_timeout(std::time::Duration::from_secs(5))
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        sqlx::SqlitePool::connect_with(options).await.map_err(|e| {
            anyhow::anyhow!(
                "Failed to open SQLite database at '{}': {}",
                db_path.display(),
                e
            )
        })
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn session_storage(&self) -> Result<SessionStorage> {
        SessionStorage::from_pool(self.pool.clone()).await
    }
}
