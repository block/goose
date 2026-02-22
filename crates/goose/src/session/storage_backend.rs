use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::conversation::message::{Message, MessageMetadata};
use crate::conversation::Conversation;
use crate::model::ModelConfig;
use crate::recipe::Recipe;
use crate::session::chat_history_search::ChatRecallResults;
use crate::session::extension_data::ExtensionData;
use crate::session::session_manager::{Session, SessionInsights, SessionType};

/// Parameters for updating a session.
/// Only `Some` fields are applied; `None` fields are left unchanged.
/// This struct is the storage-layer equivalent of `SessionUpdateBuilder`.
#[derive(Default)]
pub struct SessionUpdate {
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

/// Trait for pluggable session storage backends.
///
/// Implementors: `SqliteSessionStorage` (existing, refactored) and `MongoDbSessionStorage` (new).
///
/// Higher-level operations like `export_session`, `import_session`, and `copy_session` are
/// composed from these primitives at the `SessionManager` level.
///
/// The `update_message_metadata` closure pattern is handled by `SessionManager` calling
/// `get_message_metadata` then `set_message_metadata`.
#[async_trait]
pub trait SessionStorageBackend: Send + Sync {
    // --- Session CRUD ---

    /// Create a new session. The backend is responsible for generating a unique ID.
    async fn create_session(
        &self,
        working_dir: PathBuf,
        name: String,
        session_type: SessionType,
    ) -> Result<Session>;

    /// Get a session by ID. If `include_messages` is true, populate `session.conversation`.
    async fn get_session(&self, id: &str, include_messages: bool) -> Result<Session>;

    /// Apply a partial update to a session. Only fields set in `update` are changed.
    async fn apply_update(&self, session_id: &str, update: SessionUpdate) -> Result<()>;

    /// Delete a session and all its messages.
    async fn delete_session(&self, id: &str) -> Result<()>;

    // --- Session listing ---

    /// List sessions filtered by type(s), ordered by updated_at DESC.
    /// Only sessions that have at least one message are returned.
    async fn list_sessions_by_types(&self, types: &[SessionType]) -> Result<Vec<Session>>;

    // --- Message operations ---

    /// Add a single message to a session. Updates the session's `updated_at` timestamp.
    async fn add_message(&self, session_id: &str, message: &Message) -> Result<()>;

    /// Replace all messages in a session with the given conversation.
    async fn replace_conversation(
        &self,
        session_id: &str,
        conversation: &Conversation,
    ) -> Result<()>;

    /// Delete messages with `created_timestamp >= timestamp`.
    async fn truncate_conversation(&self, session_id: &str, timestamp: i64) -> Result<()>;

    /// Get the current metadata for a specific message.
    async fn get_message_metadata(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<MessageMetadata>;

    /// Set the metadata for a specific message.
    async fn set_message_metadata(
        &self,
        session_id: &str,
        message_id: &str,
        metadata: MessageMetadata,
    ) -> Result<()>;

    // --- Queries ---

    /// Get aggregate insights (total sessions, total tokens).
    async fn get_insights(&self) -> Result<SessionInsights>;

    /// Search chat history across sessions.
    async fn search_chat_history(
        &self,
        query: &str,
        limit: Option<usize>,
        after_date: Option<DateTime<Utc>>,
        before_date: Option<DateTime<Utc>>,
        exclude_session_id: Option<String>,
    ) -> Result<ChatRecallResults>;

    // --- Health ---

    /// Check that the backend is reachable and operational.
    async fn health_check(&self) -> Result<()>;
}
