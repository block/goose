use crate::config::APP_STRATEGY;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::session::extension_data::ExtensionData;
use crate::session::{SessionInfo, SessionMetadata};
use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};
use sqlx::{Pool, Sqlite};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

const CURRENT_SCHEMA_VERSION: i32 = 1;

static SESSION_STORAGE: OnceCell<Arc<SessionStorage>> = OnceCell::const_new();

pub struct SessionManager;

impl SessionManager {
    /// Get the singleton instance (initializes on first access)
    async fn instance() -> Result<Arc<SessionStorage>> {
        match SESSION_STORAGE.get() {
            Some(storage) => Ok(storage.clone()),
            None => {
                let storage = Arc::new(SessionStorage::new().await?);
                match SESSION_STORAGE.set(storage.clone()) {
                    Ok(_) => Ok(storage),
                    Err(_) => {
                        // Another task beat us to initialization, use theirs
                        Ok(SESSION_STORAGE.get().unwrap().clone())
                    }
                }
            }
        }
    }

    pub async fn add_message(session_id: &str, message: &Message) -> Result<()> {
        Self::instance()
            .await?
            .add_message(session_id, message)
            .await
    }

    pub async fn get_session_metadata(session_id: &str) -> Result<SessionMetadata> {
        Self::instance()
            .await?
            .get_session_metadata(session_id)
            .await
    }

    pub async fn update_session_metadata(
        session_id: &str,
        metadata: SessionMetadata,
    ) -> Result<()> {
        Self::instance()
            .await?
            .update_session_metadata(session_id, metadata)
            .await
    }

    pub async fn get_conversation(session_id: &str) -> Result<Conversation> {
        Self::instance().await?.get_conversation(session_id).await
    }

    pub async fn list_sessions() -> Result<Vec<SessionInfo>> {
        Self::instance().await?.list_sessions().await
    }

    pub async fn update_session_description(session_id: &str, description: String) -> Result<()> {
        Self::instance()
            .await?
            .update_session_description(session_id, description)
            .await
    }

    pub async fn generate_session_description(
        session_id: &str,
        provider: Arc<dyn Provider>,
    ) -> Result<()> {
        Self::instance()
            .await?
            .generate_session_description(session_id, provider)
            .await
    }
}

pub struct SessionStorage {
    pool: Pool<Sqlite>,
}

pub fn ensure_session_dir() -> Result<PathBuf> {
    let data_dir = choose_app_strategy(APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .data_dir()
        .join("sessions");

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)?;
    }

    Ok(data_dir)
}

fn role_to_string(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

impl SessionStorage {
    async fn new() -> Result<Self> {
        let session_dir = ensure_session_dir()?;
        let db_path = session_dir.join("sessions.db");

        let storage = if db_path.exists() {
            Self::open(&db_path).await?
        } else {
            println!("Creating new session database...");
            let storage = Self::create(&db_path).await?;

            // Import legacy sessions if they exist
            if let Err(e) = storage.import_legacy(&session_dir).await {
                println!("Warning: Failed to import some legacy sessions: {}", e);
            }

            storage
        };

        Ok(storage)
    }

    async fn open(db_path: &PathBuf) -> Result<Self> {
        let database_url = format!("sqlite://{}", db_path.display());
        let pool = sqlx::SqlitePool::connect(&database_url).await?;

        // Check and run migrations
        let storage = Self { pool };
        storage.run_migrations().await?;
        Ok(storage)
    }

    async fn create(db_path: &PathBuf) -> Result<Self> {
        let database_url = format!("sqlite://{}", db_path.display());
        let pool = sqlx::SqlitePool::connect(&database_url).await?;

        // Create schema version table first
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

        // Insert current schema version
        sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
            .bind(CURRENT_SCHEMA_VERSION)
            .execute(&pool)
            .await?;

        // Create initial schema (v1)
        sqlx::query(
            r#"
            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                description TEXT NOT NULL DEFAULT '',
                working_dir TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
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
                tokens INTEGER
            )
        "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE session_metadata (
                session_id TEXT PRIMARY KEY REFERENCES sessions(id),
                extension_data TEXT DEFAULT '{}',
                total_tokens INTEGER,
                input_tokens INTEGER,
                output_tokens INTEGER,
                accumulated_total_tokens INTEGER,
                accumulated_input_tokens INTEGER,
                accumulated_output_tokens INTEGER,
                schedule_id TEXT,
                recipe_json TEXT
            )
        "#,
        )
        .execute(&pool)
        .await?;

        // Indexes for performance
        sqlx::query("CREATE INDEX idx_messages_session ON messages(session_id)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX idx_messages_timestamp ON messages(timestamp)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX idx_sessions_updated ON sessions(updated_at DESC)")
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    async fn import_legacy(&self, session_dir: &PathBuf) -> Result<()> {
        use crate::session::legacy;

        let sessions = match legacy::list_sessions(session_dir) {
            Ok(sessions) => sessions,
            Err(_) => {
                println!("No legacy sessions found to import");
                return Ok(());
            }
        };

        if sessions.is_empty() {
            return Ok(());
        }

        println!("Importing {} legacy sessions...", sessions.len());

        let mut imported_count = 0;
        let mut failed_count = 0;

        for (session_name, session_path) in sessions {
            match self
                .import_single_session(&session_name, &session_path)
                .await
            {
                Ok(_) => {
                    imported_count += 1;
                    println!("  ✓ Imported: {}", session_name);
                }
                Err(e) => {
                    failed_count += 1;
                    println!("  ✗ Failed to import {}: {}", session_name, e);
                }
            }
        }

        println!(
            "Import complete: {} successful, {} failed",
            imported_count, failed_count
        );

        // Move legacy files to backup directory
        if imported_count > 0 {
            self.backup_legacy_files(session_dir).await?;
        }

        Ok(())
    }

    async fn run_migrations(&self) -> Result<()> {
        // Get current schema version
        let current_version = self.get_schema_version().await?;

        if current_version < CURRENT_SCHEMA_VERSION {
            println!(
                "Running database migrations from v{} to v{}...",
                current_version, CURRENT_SCHEMA_VERSION
            );

            for version in (current_version + 1)..=CURRENT_SCHEMA_VERSION {
                println!("  Applying migration v{}...", version);
                self.apply_migration(version).await?;
                self.update_schema_version(version).await?;
                println!("  ✓ Migration v{} complete", version);
            }

            println!("All migrations complete");
        }

        Ok(())
    }

    async fn get_schema_version(&self) -> Result<i32> {
        // Check if schema_version table exists
        let table_exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT name FROM sqlite_master 
                WHERE type='table' AND name='schema_version'
            )
        "#,
        )
        .fetch_one(&self.pool)
        .await?;

        if !table_exists {
            // Legacy database without version tracking - assume v0
            return Ok(0);
        }

        // Get latest version
        let version = sqlx::query_scalar::<_, i32>("SELECT MAX(version) FROM schema_version")
            .fetch_one(&self.pool)
            .await?;

        Ok(version)
    }

    async fn update_schema_version(&self, version: i32) -> Result<()> {
        sqlx::query("INSERT INTO schema_version (version) VALUES (?)")
            .bind(version)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn apply_migration(&self, version: i32) -> Result<()> {
        match version {
            1 => {
                // Migration v1: Create schema_version table if upgrading from legacy
                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS schema_version (
                        version INTEGER PRIMARY KEY,
                        applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )
                "#,
                )
                .execute(&self.pool)
                .await?;
            }
            2 => {
                // Example future migration: Add full-text search
                sqlx::query(
                    r#"
                    CREATE VIRTUAL TABLE messages_fts USING fts5(
                        session_id,
                        content,
                        content=messages,
                        content_rowid=id
                    )
                "#,
                )
                .execute(&self.pool)
                .await?;

                // Populate FTS table
                sqlx::query(
                    r#"
                    INSERT INTO messages_fts(session_id, content)
                    SELECT session_id, content_json FROM messages
                "#,
                )
                .execute(&self.pool)
                .await?;
            }
            3 => {
                // Example future migration: Add user management
                sqlx::query(
                    r#"
                    CREATE TABLE users (
                        id TEXT PRIMARY KEY,
                        email TEXT UNIQUE NOT NULL,
                        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )
                "#,
                )
                .execute(&self.pool)
                .await?;

                sqlx::query(
                    r#"
                    ALTER TABLE sessions ADD COLUMN user_id TEXT REFERENCES users(id)
                "#,
                )
                .execute(&self.pool)
                .await?;
            }
            4 => {
                // Example: Add session tags
                sqlx::query(
                    r#"
                    CREATE TABLE session_tags (
                        session_id TEXT REFERENCES sessions(id),
                        tag TEXT NOT NULL,
                        PRIMARY KEY (session_id, tag)
                    )
                "#,
                )
                .execute(&self.pool)
                .await?;
            }
            _ => {
                anyhow::bail!("Unknown migration version: {}", version);
            }
        }

        Ok(())
    }

    async fn import_single_session(
        &self,
        session_name: &str,
        session_path: &PathBuf,
    ) -> Result<()> {
        use crate::session::legacy;

        let conversation = legacy::read_messages(session_path)?;
        let metadata = legacy::read_metadata(session_path).await?;

        // Insert session
        sqlx::query(
            r#"
            INSERT INTO sessions (id, description, working_dir, created_at, updated_at)
            VALUES (?, ?, ?, datetime('now'), datetime('now'))
        "#,
        )
        .bind(session_name)
        .bind(&metadata.description)
        .bind(metadata.working_dir.to_string_lossy().as_ref())
        .execute(&self.pool)
        .await?;

        // Insert messages
        for message in conversation.iter() {
            sqlx::query(
                r#"
                INSERT INTO messages (session_id, role_to_string(role), content_json, created_timestamp, timestamp)
                VALUES (?, ?, ?, ?, datetime('now'))
            "#,
            )
            .bind(session_name)
            .bind(message.role.to_string())
            .bind(serde_json::to_string(&message.content)?)
            .bind(message.created)
            .execute(&self.pool)
            .await?;
        }

        // Insert metadata
        sqlx::query(
            r#"
            INSERT INTO session_metadata (
                session_id, extension_data, total_tokens, input_tokens, output_tokens,
                accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
                schedule_id, recipe_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(session_name)
        .bind(serde_json::to_string(&metadata.extension_data)?)
        .bind(metadata.total_tokens)
        .bind(metadata.input_tokens)
        .bind(metadata.output_tokens)
        .bind(metadata.accumulated_total_tokens)
        .bind(metadata.accumulated_input_tokens)
        .bind(metadata.accumulated_output_tokens)
        .bind(metadata.schedule_id)
        .bind(
            metadata
                .recipe
                .as_ref()
                .map(|r| serde_json::to_string(r).ok())
                .flatten(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn backup_legacy_files(&self, session_dir: &PathBuf) -> Result<()> {
        use crate::session::legacy;

        let backup_dir = session_dir.join("legacy_backup");
        std::fs::create_dir_all(&backup_dir)?;

        let sessions = legacy::list_sessions()?;
        for (session_name, session_path) in sessions {
            let backup_path = backup_dir.join(format!("{}.jsonl", session_name));
            if let Err(e) = std::fs::rename(&session_path, &backup_path) {
                println!("Warning: Could not backup {}: {}", session_name, e);
            }
        }

        println!("Legacy files backed up to: {:?}", backup_dir);
        Ok(())
    }

    pub async fn add_message(&self, session_id: &str, message: &Message) -> Result<()> {
        self.ensure_session_exists(session_id).await?;

        sqlx::query(
            r#"
            INSERT INTO messages (session_id, role_to_string(role), content_json, created_timestamp)
            VALUES (?, ?, ?, ?)
        "#,
        )
        .bind(session_id)
        .bind(message.role.to_string())
        .bind(serde_json::to_string(&message.content)?)
        .bind(message.created)
        .execute(&self.pool)
        .await?;

        sqlx::query("UPDATE sessions SET updated_at = datetime('now') WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_conversation(&self, session_id: &str) -> Result<Conversation> {
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT role, content_json, created_timestamp FROM messages WHERE session_id = ? ORDER BY timestamp",
        )
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for (role_str, content_json, created_timestamp) in rows {
            let role = match role_str.as_str() {
                "user" => rmcp::model::Role::User,
                "assistant" => rmcp::model::Role::Assistant,
                _ => continue,
            };

            let content = serde_json::from_str(&content_json)?;
            let message = Message::new(role, created_timestamp, content);
            messages.push(message);
        }

        Ok(Conversation::new_unvalidated(messages))
    }

    pub async fn get_session_metadata(&self, session_id: &str) -> Result<SessionMetadata> {
        let row =
            sqlx::query_as::<_, (String, String, Option<String>, Option<i32>, Option<String>)>(
                r#"
            SELECT s.working_dir, s.description, sm.extension_data, sm.total_tokens, sm.schedule_id
            FROM sessions s
            LEFT JOIN session_metadata sm ON s.id = sm.session_id
            WHERE s.id = ?
        "#,
            )
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some((working_dir, description, extension_data, total_tokens, schedule_id)) => {
                Ok(SessionMetadata {
                    working_dir: PathBuf::from(working_dir),
                    description,
                    schedule_id,
                    message_count: 0, // Could compute if needed
                    total_tokens,
                    extension_data: extension_data
                        .map(|s| serde_json::from_str(&s).unwrap_or_default())
                        .unwrap_or_default(),
                    // ... other fields
                })
            }
            None => Err(anyhow::anyhow!("Session not found")),
        }
    }

    pub async fn update_session_metadata(
        &self,
        session_id: &str,
        metadata: SessionMetadata,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET description = ?, working_dir = ?, updated_at = datetime('now')
            WHERE id = ?
        "#,
        )
        .bind(&metadata.description)
        .bind(metadata.working_dir.to_string_lossy().as_ref())
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO session_metadata
            (session_id, extension_data, total_tokens, input_tokens, output_tokens,
             accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
             schedule_id, recipe_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(session_id)
        .bind(serde_json::to_string(&metadata.extension_data)?)
        .bind(metadata.total_tokens)
        .bind(metadata.input_tokens)
        .bind(metadata.output_tokens)
        .bind(metadata.accumulated_total_tokens)
        .bind(metadata.accumulated_input_tokens)
        .bind(metadata.accumulated_output_tokens)
        .bind(metadata.schedule_id)
        .bind(
            metadata
                .recipe
                .as_ref()
                .map(|r| serde_json::to_string(r).ok())
                .flatten(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let rows = sqlx::query_as::<_, (String, String, String, String)>(
            r#"
            SELECT s.id, s.description, s.working_dir, s.updated_at
            FROM sessions s
            ORDER BY s.updated_at DESC
        "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for (id, description, working_dir, updated_at) in rows {
            // Get message count for this session
            let message_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE session_id = ?")
                    .bind(&id)
                    .fetch_one(&self.pool)
                    .await?;

            sessions.push(SessionInfo {
                id: id.clone(),
                path: format!("sqlite://sessions.db#{}", id),
                modified: updated_at,
                metadata: SessionMetadata {
                    working_dir: PathBuf::from(working_dir),
                    description,
                    schedule_id: None,
                    message_count: message_count as usize,
                    total_tokens: None,
                    input_tokens: None,
                    output_tokens: None,
                    accumulated_total_tokens: None,
                    accumulated_input_tokens: None,
                    accumulated_output_tokens: None,
                    extension_data: ExtensionData::new(),
                    recipe: None,
                },
            });
        }

        Ok(sessions)
    }

    pub async fn update_session_description(
        &self,
        session_id: &str,
        description: String,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE sessions SET description = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(description)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn generate_session_description(
        &self,
        session_id: &str,
        provider: Arc<dyn Provider>,
    ) -> Result<()> {
        let conversation = self.get_conversation(session_id).await?;
        let description = provider.generate_session_name(&conversation).await?;
        self.update_session_description(session_id, description)
            .await
    }

    async fn ensure_session_exists(&self, session_id: &str) -> Result<()> {
        let exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ?)")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await?;

        if !exists {
            let working_dir = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .to_string_lossy();

            sqlx::query(
                r#"
                INSERT INTO sessions (id, description, working_dir)
                VALUES (?, '', ?)
            "#,
            )
            .bind(session_id)
            .bind(working_dir.as_ref())
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO session_metadata (session_id, extension_data)
                VALUES (?, '{}')
            "#,
            )
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }
}
