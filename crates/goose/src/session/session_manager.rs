use crate::config::APP_STRATEGY;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::session::extension_data::ExtensionData;
use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;
use uuid::Uuid;

const CURRENT_SCHEMA_VERSION: i32 = 1;

static SESSION_STORAGE: OnceCell<Arc<SessionStorage>> = OnceCell::const_new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub working_dir: PathBuf,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub extension_data: ExtensionData,
    pub total_tokens: Option<i32>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub accumulated_total_tokens: Option<i32>,
    pub accumulated_input_tokens: Option<i32>,
    pub accumulated_output_tokens: Option<i32>,
    pub schedule_id: Option<String>,
    pub recipe_json: Option<String>,
    pub conversation: Option<Conversation>,
}

pub struct SessionUpdateBuilder {
    session_id: String,
    description: Option<String>,
    working_dir: Option<PathBuf>,
    extension_data: Option<ExtensionData>,
    total_tokens: Option<Option<i32>>,
    input_tokens: Option<Option<i32>>,
    output_tokens: Option<Option<i32>>,
    accumulated_total_tokens: Option<Option<i32>>,
    accumulated_input_tokens: Option<Option<i32>>,
    accumulated_output_tokens: Option<Option<i32>>,
    schedule_id: Option<Option<String>>,
    recipe_json: Option<Option<String>>,
}

impl SessionUpdateBuilder {
    fn new(session_id: String) -> Self {
        Self {
            session_id,
            description: None,
            working_dir: None,
            extension_data: None,
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            accumulated_total_tokens: None,
            accumulated_input_tokens: None,
            accumulated_output_tokens: None,
            schedule_id: None,
            recipe_json: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn working_dir(mut self, working_dir: PathBuf) -> Self {
        self.working_dir = Some(working_dir);
        self
    }

    pub fn extension_data(mut self, data: ExtensionData) -> Self {
        self.extension_data = Some(data);
        self
    }

    pub fn total_tokens(mut self, tokens: Option<i32>) -> Self {
        self.total_tokens = Some(tokens);
        self
    }

    pub fn input_tokens(mut self, tokens: Option<i32>) -> Self {
        self.input_tokens = Some(tokens);
        self
    }

    pub fn output_tokens(mut self, tokens: Option<i32>) -> Self {
        self.output_tokens = Some(tokens);
        self
    }

    pub fn accumulated_total_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_total_tokens = Some(tokens);
        self
    }

    pub fn accumulated_input_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_input_tokens = Some(tokens);
        self
    }

    pub fn accumulated_output_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_output_tokens = Some(tokens);
        self
    }

    pub fn schedule_id(mut self, schedule_id: Option<String>) -> Self {
        self.schedule_id = Some(schedule_id);
        self
    }

    pub fn recipe_json(mut self, recipe_json: Option<String>) -> Self {
        self.recipe_json = Some(recipe_json);
        self
    }

    pub async fn apply(self) -> Result<()> {
        SessionManager::apply_update(self).await
    }
}

pub struct SessionManager;

impl SessionManager {
    async fn instance() -> Result<Arc<SessionStorage>> {
        match SESSION_STORAGE.get() {
            Some(storage) => Ok(storage.clone()),
            None => {
                let storage = Arc::new(SessionStorage::new().await?);
                match SESSION_STORAGE.set(storage.clone()) {
                    Ok(_) => Ok(storage),
                    Err(_) => Ok(SESSION_STORAGE.get().unwrap().clone()),
                }
            }
        }
    }

    pub async fn create_session(working_dir: PathBuf, description: String) -> Result<Session> {
        let session_id = Uuid::new_v4().to_string();
        Self::instance()
            .await?
            .create_session(session_id.clone(), working_dir, description)
            .await?;
        Self::get_session(&session_id, false).await
    }

    pub async fn get_session(id: &str, include_messages: bool) -> Result<Session> {
        Self::instance()
            .await?
            .get_session(id, include_messages)
            .await
    }

    pub fn update_session(id: &str) -> SessionUpdateBuilder {
        SessionUpdateBuilder::new(id.to_string())
    }

    async fn apply_update(builder: SessionUpdateBuilder) -> Result<()> {
        Self::instance().await?.apply_update(builder).await
    }

    pub async fn add_message(id: &str, message: &Message) -> Result<()> {
        Self::instance().await?.add_message(id, message).await
    }

    pub async fn replace_conversation(id: &str, conversation: &Conversation) -> Result<()> {
        Self::instance()
            .await?
            .replace_conversation(id, conversation)
            .await
    }

    pub async fn list_sessions() -> Result<Vec<Session>> {
        Self::instance().await?.list_sessions().await
    }

    pub async fn delete_session(id: &str) -> Result<()> {
        Self::instance().await?.delete_session(id).await
    }

    pub async fn generate_description(id: &str, provider: Arc<dyn Provider>) -> Result<()> {
        let session = Self::get_session(id, true).await?;
        let conversation = session
            .conversation
            .ok_or_else(|| anyhow::anyhow!("No messages found"))?;
        let description = provider.generate_session_name(&conversation).await?;
        Self::update_session(id)
            .description(description)
            .apply()
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

type SessionRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    Option<String>,
    Option<String>,
);

impl Default for Session {
    fn default() -> Self {
        Self {
            id: String::new(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            description: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            extension_data: ExtensionData::default(),
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            accumulated_total_tokens: None,
            accumulated_input_tokens: None,
            accumulated_output_tokens: None,
            schedule_id: None,
            recipe_json: None,
            conversation: None,
        }
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

            if let Err(e) = storage.import_legacy(&session_dir).await {
                println!("Warning: Failed to import some legacy sessions: {}", e);
            }

            storage
        };

        Ok(storage)
    }

    async fn open(db_path: &PathBuf) -> Result<Self> {
        let database_url = format!("sqlite://{}", db_path.to_string_lossy());
        let pool = sqlx::SqlitePool::connect(&database_url).await?;

        let storage = Self { pool };
        storage.run_migrations().await?;
        Ok(storage)
    }

    async fn create(db_path: &PathBuf) -> Result<Self> {
        let database_url = format!("sqlite://{}", db_path.to_string_lossy());
        let pool = sqlx::SqlitePool::connect(&database_url).await?;

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
            .bind(CURRENT_SCHEMA_VERSION)
            .execute(&pool)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                description TEXT NOT NULL DEFAULT '',
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
                recipe_json TEXT
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
        Ok(())
    }

    async fn run_migrations(&self) -> Result<()> {
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
            return Ok(0);
        }

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

        sqlx::query(
            r#"
            INSERT INTO sessions (
                id, description, working_dir,
                extension_data, total_tokens, input_tokens, output_tokens,
                accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
                schedule_id, recipe_json,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        "#,
        )
        .bind(session_name)
        .bind(&metadata.description)
        .bind(metadata.working_dir.to_string_lossy().as_ref())
        .bind(serde_json::to_string(&metadata.extension_data)?)
        .bind(metadata.total_tokens)
        .bind(metadata.input_tokens)
        .bind(metadata.output_tokens)
        .bind(metadata.accumulated_total_tokens)
        .bind(metadata.accumulated_input_tokens)
        .bind(metadata.accumulated_output_tokens)
        .bind(metadata.schedule_id)
        .bind(metadata.recipe_json)
        .execute(&self.pool)
        .await?;

        for message in conversation.iter() {
            sqlx::query(
                r#"
                INSERT INTO messages (session_id, role, content_json, created_timestamp, timestamp)
                VALUES (?, ?, ?, ?, datetime('now'))
            "#,
            )
            .bind(session_name)
            .bind(role_to_string(&message.role))
            .bind(serde_json::to_string(&message.content)?)
            .bind(message.created)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    fn row_to_session(row: SessionRow, conversation: Option<Conversation>) -> Session {
        let (
            id,
            working_dir,
            description,
            created_at,
            updated_at,
            extension_data,
            total_tokens,
            input_tokens,
            output_tokens,
            accumulated_total_tokens,
            accumulated_input_tokens,
            accumulated_output_tokens,
            schedule_id,
            recipe_json,
        ) = row;

        Session {
            id,
            working_dir: PathBuf::from(working_dir),
            description,
            created_at,
            updated_at,
            extension_data: serde_json::from_str(&extension_data).unwrap_or_default(),
            total_tokens,
            input_tokens,
            output_tokens,
            accumulated_total_tokens,
            accumulated_input_tokens,
            accumulated_output_tokens,
            schedule_id,
            recipe_json,
            conversation,
        }
    }

    async fn create_session(
        &self,
        session_id: String,
        working_dir: PathBuf,
        description: String,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, description, working_dir, extension_data)
            VALUES (?, ?, ?, '{}')
        "#,
        )
        .bind(&session_id)
        .bind(&description)
        .bind(working_dir.to_string_lossy().as_ref())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_session(&self, id: &str, include_messages: bool) -> Result<Session> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, working_dir, description, created_at, updated_at, extension_data,
                   total_tokens, input_tokens, output_tokens,
                   accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
                   schedule_id, recipe_json
            FROM sessions
            WHERE id = ?
        "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let conversation = if include_messages {
            Some(self.get_conversation(&row.0).await?)
        } else {
            None
        };

        Ok(Self::row_to_session(row, conversation))
    }

    async fn apply_update(&self, builder: SessionUpdateBuilder) -> Result<()> {
        let mut updates = Vec::new();
        let mut query = String::from("UPDATE sessions SET ");

        macro_rules! add_update {
            ($field:expr, $name:expr) => {
                if $field.is_some() {
                    if !updates.is_empty() {
                        query.push_str(", ");
                    }
                    updates.push($name);
                    query.push_str($name);
                    query.push_str(" = ?");
                }
            };
        }

        add_update!(builder.description, "description");
        add_update!(builder.working_dir, "working_dir");
        add_update!(builder.extension_data, "extension_data");
        add_update!(builder.total_tokens, "total_tokens");
        add_update!(builder.input_tokens, "input_tokens");
        add_update!(builder.output_tokens, "output_tokens");
        add_update!(builder.accumulated_total_tokens, "accumulated_total_tokens");
        add_update!(builder.accumulated_input_tokens, "accumulated_input_tokens");
        add_update!(
            builder.accumulated_output_tokens,
            "accumulated_output_tokens"
        );
        add_update!(builder.schedule_id, "schedule_id");
        add_update!(builder.recipe_json, "recipe_json");

        if updates.is_empty() {
            return Ok(());
        }

        if !updates.is_empty() {
            query.push_str(", ");
        }
        query.push_str("updated_at = datetime('now') WHERE id = ?");

        let mut q = sqlx::query(&query);

        if let Some(desc) = builder.description {
            q = q.bind(desc);
        }
        if let Some(wd) = builder.working_dir {
            q = q.bind(wd.to_string_lossy().to_string());
        }
        if let Some(ed) = builder.extension_data {
            q = q.bind(serde_json::to_string(&ed)?);
        }
        if let Some(tt) = builder.total_tokens {
            q = q.bind(tt);
        }
        if let Some(it) = builder.input_tokens {
            q = q.bind(it);
        }
        if let Some(ot) = builder.output_tokens {
            q = q.bind(ot);
        }
        if let Some(att) = builder.accumulated_total_tokens {
            q = q.bind(att);
        }
        if let Some(ait) = builder.accumulated_input_tokens {
            q = q.bind(ait);
        }
        if let Some(aot) = builder.accumulated_output_tokens {
            q = q.bind(aot);
        }
        if let Some(sid) = builder.schedule_id {
            q = q.bind(sid);
        }
        if let Some(rj) = builder.recipe_json {
            q = q.bind(rj);
        }

        q = q.bind(&builder.session_id);
        q.execute(&self.pool).await?;

        Ok(())
    }

    async fn get_conversation(&self, session_id: &str) -> Result<Conversation> {
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT role, content_json, created_timestamp FROM messages WHERE session_id = ? ORDER BY timestamp",
        )
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for (role_str, content_json, created_timestamp) in rows {
            let role = match role_str.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => continue,
            };

            let content = serde_json::from_str(&content_json)?;
            let message = Message::new(role, created_timestamp, content);
            messages.push(message);
        }

        Ok(Conversation::new_unvalidated(messages))
    }

    async fn add_message(&self, session_id: &str, message: &Message) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO messages (session_id, role, content_json, created_timestamp)
            VALUES (?, ?, ?, ?)
        "#,
        )
        .bind(session_id)
        .bind(role_to_string(&message.role))
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

    async fn replace_conversation(
        &self,
        session_id: &str,
        conversation: &Conversation,
    ) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        for message in conversation.messages() {
            sqlx::query(
                r#"
                INSERT INTO messages (session_id, role, content_json, created_timestamp)
                VALUES (?, ?, ?, ?)
            "#,
            )
            .bind(session_id)
            .bind(role_to_string(&message.role))
            .bind(serde_json::to_string(&message.content)?)
            .bind(message.created)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn list_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT id, working_dir, description, created_at, updated_at, extension_data,
                   total_tokens, input_tokens, output_tokens,
                   accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
                   schedule_id, recipe_json
            FROM sessions
            ORDER BY updated_at DESC
        "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Self::row_to_session(row, None))
            .collect())
    }

    async fn delete_session(&self, session_id: &str) -> Result<()> {
        let exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ?)")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await?;

        if !exists {
            return Err(anyhow::anyhow!("Session not found"));
        }

        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
