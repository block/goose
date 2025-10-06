use crate::config::paths::Paths;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::{Provider, MSG_COUNT_FOR_SESSION_NAME_GENERATION};
use crate::recipe::Recipe;
use crate::session::extension_data::ExtensionData;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{info, warn};
use utoipa::ToSchema;

const CURRENT_SCHEMA_VERSION: i32 = 2;

static SESSION_STORAGE: OnceCell<Arc<SessionStorage>> = OnceCell::const_new();

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Session {
    pub id: String,
    #[schema(value_type = String)]
    pub working_dir: PathBuf,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub extension_data: ExtensionData,
    pub total_tokens: Option<i32>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub accumulated_total_tokens: Option<i32>,
    pub accumulated_input_tokens: Option<i32>,
    pub accumulated_output_tokens: Option<i32>,
    pub schedule_id: Option<String>,
    pub recipe: Option<Recipe>,
    pub user_recipe_values: Option<HashMap<String, String>>,
    pub conversation: Option<Conversation>,
    pub message_count: usize,
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
    recipe: Option<Option<Recipe>>,
    user_recipe_values: Option<Option<HashMap<String, String>>>,
}

#[derive(Serialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionInsights {
    /// Total number of sessions
    total_sessions: usize,
    /// Total tokens used across all sessions
    total_tokens: i64,
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
            recipe: None,
            user_recipe_values: None,
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

    pub fn recipe(mut self, recipe: Option<Recipe>) -> Self {
        self.recipe = Some(recipe);
        self
    }

    pub fn user_recipe_values(
        mut self,
        user_recipe_values: Option<HashMap<String, String>>,
    ) -> Self {
        self.user_recipe_values = Some(user_recipe_values);
        self
    }

    pub async fn apply(self) -> Result<()> {
        SessionManager::apply_update(self).await
    }
}

pub struct SessionManager;

impl SessionManager {
    pub async fn instance() -> Result<Arc<SessionStorage>> {
        SESSION_STORAGE
            .get_or_try_init(|| async { SessionStorage::new().await.map(Arc::new) })
            .await
            .map(Arc::clone)
    }

    pub async fn create_session(working_dir: PathBuf, description: String) -> Result<Session> {
        Self::instance()
            .await?
            .create_session(working_dir, description)
            .await
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

    pub async fn get_insights() -> Result<SessionInsights> {
        Self::instance().await?.get_insights().await
    }

    pub async fn maybe_update_description(id: &str, provider: Arc<dyn Provider>) -> Result<()> {
        let session = Self::get_session(id, true).await?;
        let conversation = session
            .conversation
            .ok_or_else(|| anyhow::anyhow!("No messages found"))?;

        let user_message_count = conversation
            .messages()
            .iter()
            .filter(|m| matches!(m.role, Role::User))
            .count();

        if user_message_count <= MSG_COUNT_FOR_SESSION_NAME_GENERATION {
            let description = provider.generate_session_name(&conversation).await?;
            Self::update_session(id)
                .description(description)
                .apply()
                .await
        } else {
            Ok(())
        }
    }
}

pub struct SessionStorage {
    pool: Pool<Sqlite>,
}

pub fn ensure_session_dir() -> Result<PathBuf> {
    let session_dir = Paths::data_dir().join("sessions");

    if !session_dir.exists() {
        fs::create_dir_all(&session_dir)?;
    }

    Ok(session_dir)
}

fn role_to_string(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

impl Default for Session {
    fn default() -> Self {
        Self {
            id: String::new(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            description: String::new(),
            created_at: Default::default(),
            updated_at: Default::default(),
            extension_data: ExtensionData::default(),
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            accumulated_total_tokens: None,
            accumulated_input_tokens: None,
            accumulated_output_tokens: None,
            schedule_id: None,
            recipe: None,
            user_recipe_values: None,
            conversation: None,
            message_count: 0,
        }
    }
}

impl Session {
    pub fn without_messages(mut self) -> Self {
        self.conversation = None;
        self
    }
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Session {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let recipe_json: Option<String> = row.try_get("recipe_json")?;
        let recipe = recipe_json.and_then(|json| serde_json::from_str(&json).ok());

        let user_recipe_values_json: Option<String> = row.try_get("user_recipe_values_json")?;
        let user_recipe_values =
            user_recipe_values_json.and_then(|json| serde_json::from_str(&json).ok());

        Ok(Session {
            id: row.try_get("id")?,
            working_dir: PathBuf::from(row.try_get::<String, _>("working_dir")?),
            description: row.try_get("description")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            extension_data: serde_json::from_str(&row.try_get::<String, _>("extension_data")?)
                .unwrap_or_default(),
            total_tokens: row.try_get("total_tokens")?,
            input_tokens: row.try_get("input_tokens")?,
            output_tokens: row.try_get("output_tokens")?,
            accumulated_total_tokens: row.try_get("accumulated_total_tokens")?,
            accumulated_input_tokens: row.try_get("accumulated_input_tokens")?,
            accumulated_output_tokens: row.try_get("accumulated_output_tokens")?,
            schedule_id: row.try_get("schedule_id")?,
            recipe,
            user_recipe_values,
            conversation: None,
            message_count: row.try_get("message_count").unwrap_or(0) as usize,
        })
    }
}

impl SessionStorage {
    async fn new() -> Result<Self> {
        let session_dir = ensure_session_dir()?;
        let db_path = session_dir.join("sessions.db");

        let storage = if db_path.exists() {
            Self::open(&db_path).await?
        } else {
            let storage = Self::create(&db_path).await?;

            if let Err(e) = storage.import_legacy(&session_dir).await {
                warn!("Failed to import some legacy sessions: {}", e);
            }

            storage
        };

        Ok(storage)
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

    async fn open(db_path: &Path) -> Result<Self> {
        let pool = Self::get_pool(db_path, false).await?;

        let storage = Self { pool };
        storage.run_migrations().await?;
        Ok(storage)
    }

    async fn create(db_path: &Path) -> Result<Self> {
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
                recipe_json TEXT,
                user_recipe_values_json TEXT
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
                warn!("No legacy sessions found to import");
                return Ok(());
            }
        };

        if sessions.is_empty() {
            return Ok(());
        }

        let mut imported_count = 0;
        let mut failed_count = 0;

        for (session_name, session_path) in sessions {
            match legacy::load_session(&session_name, &session_path) {
                Ok(session) => match self.import_legacy_session(&session).await {
                    Ok(_) => {
                        imported_count += 1;
                        info!("  ✓ Imported: {}", session_name);
                    }
                    Err(e) => {
                        failed_count += 1;
                        info!("  ✗ Failed to import {}: {}", session_name, e);
                    }
                },
                Err(e) => {
                    failed_count += 1;
                    info!("  ✗ Failed to load {}: {}", session_name, e);
                }
            }
        }

        info!(
            "Import complete: {} successful, {} failed",
            imported_count, failed_count
        );
        Ok(())
    }

    async fn import_legacy_session(&self, session: &Session) -> Result<()> {
        let recipe_json = match &session.recipe {
            Some(recipe) => Some(serde_json::to_string(recipe)?),
            None => None,
        };

        let user_recipe_values_json = match &session.user_recipe_values {
            Some(user_recipe_values) => Some(serde_json::to_string(user_recipe_values)?),
            None => None,
        };

        sqlx::query(
            r#"
        INSERT INTO sessions (
            id, description, working_dir, created_at, updated_at, extension_data,
            total_tokens, input_tokens, output_tokens,
            accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
            schedule_id, recipe_json, user_recipe_values_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&session.id)
        .bind(&session.description)
        .bind(session.working_dir.to_string_lossy().as_ref())
        .bind(session.created_at)
        .bind(session.updated_at)
        .bind(serde_json::to_string(&session.extension_data)?)
        .bind(session.total_tokens)
        .bind(session.input_tokens)
        .bind(session.output_tokens)
        .bind(session.accumulated_total_tokens)
        .bind(session.accumulated_input_tokens)
        .bind(session.accumulated_output_tokens)
        .bind(&session.schedule_id)
        .bind(recipe_json)
        .bind(user_recipe_values_json)
        .execute(&self.pool)
        .await?;

        if let Some(conversation) = &session.conversation {
            self.replace_conversation(&session.id, conversation).await?;
        }
        Ok(())
    }

    async fn run_migrations(&self) -> Result<()> {
        let current_version = self.get_schema_version().await?;

        if current_version < CURRENT_SCHEMA_VERSION {
            info!(
                "Running database migrations from v{} to v{}...",
                current_version, CURRENT_SCHEMA_VERSION
            );

            for version in (current_version + 1)..=CURRENT_SCHEMA_VERSION {
                info!("  Applying migration v{}...", version);
                self.apply_migration(version).await?;
                self.update_schema_version(version).await?;
                info!("  ✓ Migration v{} complete", version);
            }

            info!("All migrations complete");
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
            2 => {
                sqlx::query(
                    r#"
                    ALTER TABLE sessions ADD COLUMN user_recipe_values_json TEXT
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

    async fn create_session(&self, working_dir: PathBuf, description: String) -> Result<Session> {
        let today = chrono::Utc::now().format("%Y%m%d").to_string();
        Ok(sqlx::query_as(
            r#"
                INSERT INTO sessions (id, description, working_dir, extension_data)
                VALUES (
                    ? || '_' || CAST(COALESCE((
                        SELECT MAX(CAST(SUBSTR(id, 10) AS INTEGER))
                        FROM sessions
                        WHERE id LIKE ? || '_%'
                    ), 0) + 1 AS TEXT),
                    ?,
                    ?,
                    '{}'
                )
                RETURNING *
                "#,
        )
        .bind(&today)
        .bind(&today)
        .bind(&description)
        .bind(working_dir.to_string_lossy().as_ref())
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_session(&self, id: &str, include_messages: bool) -> Result<Session> {
        let mut session = sqlx::query_as::<_, Session>(
            r#"
        SELECT id, working_dir, description, created_at, updated_at, extension_data,
               total_tokens, input_tokens, output_tokens,
               accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
               schedule_id, recipe_json, user_recipe_values_json
        FROM sessions
        WHERE id = ?
    "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        if include_messages {
            let conv = self.get_conversation(&session.id).await?;
            session.message_count = conv.messages().len();
            session.conversation = Some(conv);
        } else {
            let count =
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM messages WHERE session_id = ?")
                    .bind(&session.id)
                    .fetch_one(&self.pool)
                    .await? as usize;
            session.message_count = count;
        }

        Ok(session)
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
        add_update!(builder.recipe, "recipe_json");
        add_update!(builder.user_recipe_values, "user_recipe_values_json");

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
        if let Some(recipe) = builder.recipe {
            let recipe_json = recipe.map(|r| serde_json::to_string(&r)).transpose()?;
            q = q.bind(recipe_json);
        }
        if let Some(user_recipe_values) = builder.user_recipe_values {
            let user_recipe_values_json = user_recipe_values
                .map(|urv| serde_json::to_string(&urv))
                .transpose()?;
            q = q.bind(user_recipe_values_json);
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
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&mut *tx)
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
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_sessions(&self) -> Result<Vec<Session>> {
        sqlx::query_as::<_, Session>(
            r#"
        SELECT s.id, s.working_dir, s.description, s.created_at, s.updated_at, s.extension_data,
               s.total_tokens, s.input_tokens, s.output_tokens,
               s.accumulated_total_tokens, s.accumulated_input_tokens, s.accumulated_output_tokens,
               s.schedule_id, s.recipe_json, s.user_recipe_values_json,
               COUNT(m.id) as message_count
        FROM sessions s
        INNER JOIN messages m ON s.id = m.session_id
        GROUP BY s.id
        ORDER BY s.updated_at DESC
    "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
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

    async fn get_insights(&self) -> Result<SessionInsights> {
        let row = sqlx::query_as::<_, (i64, Option<i64>)>(
            r#"
            SELECT COUNT(*) as total_sessions,
                   COALESCE(SUM(COALESCE(accumulated_total_tokens, total_tokens, 0)), 0) as total_tokens
            FROM sessions
            "#,
        )
            .fetch_one(&self.pool)
            .await?;

        Ok(SessionInsights {
            total_sessions: row.0 as usize,
            total_tokens: row.1.unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{Message, MessageContent};
    use tempfile::TempDir;

    const NUM_CONCURRENT_SESSIONS: i32 = 10;

    #[tokio::test]
    async fn test_concurrent_session_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_sessions.db");

        let storage = Arc::new(SessionStorage::create(&db_path).await.unwrap());

        let mut handles = vec![];

        for i in 0..NUM_CONCURRENT_SESSIONS {
            let session_storage = Arc::clone(&storage);
            let handle = tokio::spawn(async move {
                let working_dir = PathBuf::from(format!("/tmp/test_{}", i));
                let description = format!("Test session {}", i);

                let session = session_storage
                    .create_session(working_dir.clone(), description)
                    .await
                    .unwrap();

                session_storage
                    .add_message(
                        &session.id,
                        &Message {
                            id: None,
                            role: Role::User,
                            created: chrono::Utc::now().timestamp_millis(),
                            content: vec![MessageContent::text("hello world")],
                            metadata: Default::default(),
                        },
                    )
                    .await
                    .unwrap();

                session_storage
                    .add_message(
                        &session.id,
                        &Message {
                            id: None,
                            role: Role::Assistant,
                            created: chrono::Utc::now().timestamp_millis(),
                            content: vec![MessageContent::text("sup world?")],
                            metadata: Default::default(),
                        },
                    )
                    .await
                    .unwrap();

                session_storage
                    .apply_update(
                        SessionUpdateBuilder::new(session.id.clone())
                            .description(format!("Updated session {}", i))
                            .total_tokens(Some(100 * i)),
                    )
                    .await
                    .unwrap();

                let updated = session_storage
                    .get_session(&session.id, true)
                    .await
                    .unwrap();
                assert_eq!(updated.message_count, 2);
                assert_eq!(updated.total_tokens, Some(100 * i));

                session.id
            });
            handles.push(handle);
        }

        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        assert_eq!(results.len(), NUM_CONCURRENT_SESSIONS as usize);

        let unique_ids: std::collections::HashSet<_> = results.iter().collect();
        assert_eq!(unique_ids.len(), NUM_CONCURRENT_SESSIONS as usize);

        let sessions = storage.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), NUM_CONCURRENT_SESSIONS as usize);

        for session in &sessions {
            assert_eq!(session.message_count, 2);
            assert!(session.description.starts_with("Updated session"));
        }

        let insights = storage.get_insights().await.unwrap();
        assert_eq!(insights.total_sessions, NUM_CONCURRENT_SESSIONS as usize);
        let expected_tokens = 100 * NUM_CONCURRENT_SESSIONS * (NUM_CONCURRENT_SESSIONS - 1) / 2;
        assert_eq!(insights.total_tokens, expected_tokens as i64);
    }
}
