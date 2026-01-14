use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::model::ModelConfig;
use crate::providers::base::{Provider, MSG_COUNT_FOR_SESSION_NAME_GENERATION};
use crate::recipe::Recipe;
use crate::session::extension_data::ExtensionData;
use crate::storage::StorageManager;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    #[default]
    User,
    Scheduled,
    SubAgent,
    Hidden,
    Terminal,
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionType::User => write!(f, "user"),
            SessionType::SubAgent => write!(f, "sub_agent"),
            SessionType::Hidden => write!(f, "hidden"),
            SessionType::Scheduled => write!(f, "scheduled"),
            SessionType::Terminal => write!(f, "terminal"),
        }
    }
}

impl std::str::FromStr for SessionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(SessionType::User),
            "sub_agent" => Ok(SessionType::SubAgent),
            "hidden" => Ok(SessionType::Hidden),
            "scheduled" => Ok(SessionType::Scheduled),
            "terminal" => Ok(SessionType::Terminal),
            _ => Err(anyhow::anyhow!("Invalid session type: {}", s)),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Session {
    pub id: String,
    #[schema(value_type = String)]
    pub working_dir: PathBuf,
    #[serde(alias = "description")]
    pub name: String,
    #[serde(default)]
    pub user_set_name: bool,
    #[serde(default)]
    pub session_type: SessionType,
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
    pub provider_name: Option<String>,
    pub model_config: Option<ModelConfig>,
}

pub struct SessionUpdateBuilder {
    pub session_id: String,
    pub name: Option<String>,
    pub user_set_name: Option<bool>,
    pub session_type: Option<SessionType>,
    pub working_dir: Option<PathBuf>,
    pub extension_data: Option<ExtensionData>,
    pub total_tokens: Option<Option<i32>>,
    pub input_tokens: Option<Option<i32>>,
    pub output_tokens: Option<Option<i32>>,
    pub accumulated_total_tokens: Option<Option<i32>>,
    pub accumulated_input_tokens: Option<Option<i32>>,
    pub accumulated_output_tokens: Option<Option<i32>>,
    pub schedule_id: Option<Option<String>>,
    pub recipe: Option<Option<Recipe>>,
    pub user_recipe_values: Option<Option<HashMap<String, String>>>,
    pub provider_name: Option<Option<String>>,
    pub model_config: Option<Option<ModelConfig>>,
}

#[derive(Serialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionInsights {
    pub total_sessions: usize,
    pub total_tokens: i64,
}

impl SessionUpdateBuilder {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            name: None,
            user_set_name: None,
            session_type: None,
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
            provider_name: None,
            model_config: None,
        }
    }

    pub fn user_provided_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into().trim().to_string();
        if !name.is_empty() {
            self.name = Some(name);
            self.user_set_name = Some(true);
        }
        self
    }

    pub fn system_generated_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into().trim().to_string();
        if !name.is_empty() {
            self.name = Some(name);
            self.user_set_name = Some(false);
        }
        self
    }

    pub fn session_type(mut self, session_type: SessionType) -> Self {
        self.session_type = Some(session_type);
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

    pub fn provider_name(mut self, provider_name: impl Into<String>) -> Self {
        self.provider_name = Some(Some(provider_name.into()));
        self
    }

    pub fn model_config(mut self, model_config: ModelConfig) -> Self {
        self.model_config = Some(Some(model_config));
        self
    }

    pub async fn apply(self) -> Result<()> {
        SessionManager::apply_update(self).await
    }
}

pub struct SessionManager;

impl SessionManager {
    pub async fn create_session(
        working_dir: PathBuf,
        name: String,
        session_type: SessionType,
    ) -> Result<Session> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage
            .create_session(working_dir, name, session_type)
            .await
    }

    pub async fn get_session(id: &str, include_messages: bool) -> Result<Session> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.get_session(id, include_messages).await
    }

    pub fn update_session(id: &str) -> SessionUpdateBuilder {
        SessionUpdateBuilder::new(id.to_string())
    }

    async fn apply_update(builder: SessionUpdateBuilder) -> Result<()> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.apply_update(builder).await
    }

    pub async fn add_message(id: &str, message: &Message) -> Result<()> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.add_message(id, message).await
    }

    pub async fn replace_conversation(id: &str, conversation: &Conversation) -> Result<()> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage
            .replace_conversation(id, conversation)
            .await
    }

    pub async fn list_sessions() -> Result<Vec<Session>> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.list_sessions().await
    }

    pub async fn list_sessions_by_types(types: &[SessionType]) -> Result<Vec<Session>> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.list_sessions_by_types(types).await
    }

    pub async fn delete_session(id: &str) -> Result<()> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.delete_session(id).await
    }

    pub async fn get_insights() -> Result<SessionInsights> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.get_insights().await
    }

    pub async fn export_session(id: &str) -> Result<String> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.export_session(id).await
    }

    pub async fn import_session(json: &str) -> Result<Session> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.import_session(json).await
    }

    pub async fn copy_session(session_id: &str, new_name: String) -> Result<Session> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage.copy_session(session_id, new_name).await
    }

    pub async fn truncate_conversation(session_id: &str, timestamp: i64) -> Result<()> {
        let storage_manager = StorageManager::instance().await?;
        let session_storage = storage_manager.session_storage().await?;
        session_storage
            .truncate_conversation(session_id, timestamp)
            .await
    }

    pub async fn maybe_update_name(id: &str, provider: Arc<dyn Provider>) -> Result<()> {
        let session = Self::get_session(id, true).await?;

        if session.user_set_name {
            return Ok(());
        }

        let conversation = session
            .conversation
            .ok_or_else(|| anyhow::anyhow!("No messages found"))?;

        let user_message_count = conversation
            .messages()
            .iter()
            .filter(|m| matches!(m.role, Role::User))
            .count();

        if user_message_count <= MSG_COUNT_FOR_SESSION_NAME_GENERATION {
            let name = provider.generate_session_name(&conversation).await?;
            Self::update_session(id)
                .system_generated_name(name)
                .apply()
                .await
        } else {
            Ok(())
        }
    }

    pub async fn search_chat_history(
        query: &str,
        limit: Option<usize>,
        after_date: Option<chrono::DateTime<chrono::Utc>>,
        before_date: Option<chrono::DateTime<chrono::Utc>>,
        exclude_session_id: Option<String>,
    ) -> Result<crate::session::chat_history_search::ChatRecallResults> {
        use crate::session::chat_history_search::ChatHistorySearch;

        let storage_manager = StorageManager::instance().await?;
        let pool = storage_manager.pool();

        ChatHistorySearch::new(
            pool,
            query,
            limit,
            after_date,
            before_date,
            exclude_session_id,
        )
        .execute()
        .await
    }

    pub async fn import_legacy_sessions(
        session_dir: &PathBuf,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> Result<()> {
        use crate::session::legacy;
        use crate::storage::SessionStorage;
        use tracing::{info, warn};

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

        let session_storage = SessionStorage::from_pool(pool.clone()).await?;
        let mut imported_count = 0;
        let mut failed_count = 0;

        for (session_name, session_path) in sessions {
            match legacy::load_session(&session_name, &session_path) {
                Ok(session) => match session_storage.import_legacy_session(&session).await {
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
}


impl Default for Session {
    fn default() -> Self {
        Self {
            id: String::new(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            name: String::new(),
            user_set_name: false,
            session_type: SessionType::default(),
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
            provider_name: None,
            model_config: None,
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

        let model_config_json: Option<String> = row.try_get("model_config_json").ok().flatten();
        let model_config = model_config_json.and_then(|json| serde_json::from_str(&json).ok());

        let name: String = {
            let name_val: String = row.try_get("name").unwrap_or_default();
            if !name_val.is_empty() {
                name_val
            } else {
                row.try_get("description").unwrap_or_default()
            }
        };

        let user_set_name = row.try_get("user_set_name").unwrap_or(false);

        let session_type_str: String = row
            .try_get("session_type")
            .unwrap_or_else(|_| "user".to_string());
        let session_type = session_type_str.parse().unwrap_or_default();

        Ok(Session {
            id: row.try_get("id")?,
            working_dir: PathBuf::from(row.try_get::<String, _>("working_dir")?),
            name,
            user_set_name,
            session_type,
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
            provider_name: row.try_get("provider_name").ok().flatten(),
            model_config,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{Message, MessageContent};
    use crate::storage::session_storage::SessionStorage;
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
                    .create_session(working_dir.clone(), description, SessionType::User)
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
                            .user_provided_name(format!("Updated session {}", i))
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
            assert!(session.name.starts_with("Updated session"));
        }

        let insights = storage.get_insights().await.unwrap();
        assert_eq!(insights.total_sessions, NUM_CONCURRENT_SESSIONS as usize);
        let expected_tokens = 100 * NUM_CONCURRENT_SESSIONS * (NUM_CONCURRENT_SESSIONS - 1) / 2;
        assert_eq!(insights.total_tokens, expected_tokens as i64);
    }

    #[tokio::test]
    async fn test_export_import_roundtrip() {
        const DESCRIPTION: &str = "Original session";
        const TOTAL_TOKENS: i32 = 500;
        const INPUT_TOKENS: i32 = 300;
        const OUTPUT_TOKENS: i32 = 200;
        const ACCUMULATED_TOKENS: i32 = 1000;
        const USER_MESSAGE: &str = "test message";
        const ASSISTANT_MESSAGE: &str = "test response";

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_export.db");
        let storage = Arc::new(SessionStorage::create(&db_path).await.unwrap());

        let original = storage
            .create_session(
                PathBuf::from("/tmp/test"),
                DESCRIPTION.to_string(),
                SessionType::User,
            )
            .await
            .unwrap();

        storage
            .apply_update(
                SessionUpdateBuilder::new(original.id.clone())
                    .total_tokens(Some(TOTAL_TOKENS))
                    .input_tokens(Some(INPUT_TOKENS))
                    .output_tokens(Some(OUTPUT_TOKENS))
                    .accumulated_total_tokens(Some(ACCUMULATED_TOKENS)),
            )
            .await
            .unwrap();

        storage
            .add_message(
                &original.id,
                &Message {
                    id: None,
                    role: Role::User,
                    created: chrono::Utc::now().timestamp_millis(),
                    content: vec![MessageContent::text(USER_MESSAGE)],
                    metadata: Default::default(),
                },
            )
            .await
            .unwrap();

        storage
            .add_message(
                &original.id,
                &Message {
                    id: None,
                    role: Role::Assistant,
                    created: chrono::Utc::now().timestamp_millis(),
                    content: vec![MessageContent::text(ASSISTANT_MESSAGE)],
                    metadata: Default::default(),
                },
            )
            .await
            .unwrap();

        let exported = storage.export_session(&original.id).await.unwrap();
        let imported = storage.import_session(&exported).await.unwrap();

        assert_ne!(imported.id, original.id);
        assert_eq!(imported.name, DESCRIPTION);
        assert_eq!(imported.working_dir, PathBuf::from("/tmp/test"));
        assert_eq!(imported.total_tokens, Some(TOTAL_TOKENS));
        assert_eq!(imported.input_tokens, Some(INPUT_TOKENS));
        assert_eq!(imported.output_tokens, Some(OUTPUT_TOKENS));
        assert_eq!(imported.accumulated_total_tokens, Some(ACCUMULATED_TOKENS));
        assert_eq!(imported.message_count, 2);

        let conversation = imported.conversation.unwrap();
        assert_eq!(conversation.messages().len(), 2);
        assert_eq!(conversation.messages()[0].role, Role::User);
        assert_eq!(conversation.messages()[1].role, Role::Assistant);
    }

    #[tokio::test]
    async fn test_import_session_with_description_field() {
        const OLD_FORMAT_JSON: &str = r#"{
            "id": "20240101_1",
            "description": "Old format session",
            "user_set_name": true,
            "working_dir": "/tmp/test",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "extension_data": {},
            "message_count": 0
        }"#;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_import.db");
        let storage = Arc::new(SessionStorage::create(&db_path).await.unwrap());

        let imported = storage.import_session(OLD_FORMAT_JSON).await.unwrap();

        assert_eq!(imported.name, "Old format session");
        assert!(imported.user_set_name);
        assert_eq!(imported.working_dir, PathBuf::from("/tmp/test"));
    }
}
