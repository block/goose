pub use crate::session::migrations::CURRENT_SCHEMA_VERSION;
pub use crate::session::model::{Session, SessionInsights, SessionType};
pub use crate::session::storage::{SessionStorage, DB_NAME, SESSIONS_FOLDER};
pub use crate::session::update_builder::SessionUpdateBuilder;

use crate::config::GooseMode;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::{Provider, MSG_COUNT_FOR_SESSION_NAME_GENERATION};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rmcp::model::Role;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

static SESSION_STORAGE: LazyLock<Arc<SessionStorage>> =
    LazyLock::new(|| Arc::new(SessionStorage::new(crate::config::paths::Paths::data_dir())));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionListCursor {
    pub updated_at: DateTime<Utc>,
    pub session_id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionListPage {
    pub(crate) sessions: Vec<Session>,
    pub(crate) next_cursor: Option<SessionListCursor>,
}

#[derive(Debug, Default)]
pub(crate) struct SessionListQuery<'a> {
    pub(crate) types: Option<&'a [SessionType]>,
    pub(crate) working_dir: Option<&'a Path>,
    pub(crate) cursor: Option<&'a SessionListCursor>,
    pub(crate) limit: Option<usize>,
    pub(crate) require_messages: bool,
}

#[derive(Debug, Clone)]
pub struct SessionNameUpdate {
    pub session_id: String,
    pub name: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub message_count: usize,
    pub user_set_name: bool,
}

pub struct SessionManager {
    storage: Arc<SessionStorage>,
}

impl SessionManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            storage: Arc::new(SessionStorage::new(data_dir)),
        }
    }

    pub fn instance() -> Self {
        Self {
            storage: Arc::clone(&SESSION_STORAGE),
        }
    }

    pub fn storage(&self) -> &Arc<SessionStorage> {
        &self.storage
    }

    pub async fn create_session(
        &self,
        working_dir: PathBuf,
        name: String,
        session_type: SessionType,
        goose_mode: GooseMode,
    ) -> Result<Session> {
        self.storage
            .create_session(working_dir, name, session_type, goose_mode)
            .await
    }

    pub async fn get_session(&self, id: &str, include_messages: bool) -> Result<Session> {
        self.storage.get_session(id, include_messages).await
    }

    pub fn update(&self, id: &str) -> SessionUpdateBuilder<'_> {
        SessionUpdateBuilder::new(self, id.to_string())
    }

    pub(crate) async fn apply_update_inner(&self, builder: SessionUpdateBuilder<'_>) -> Result<()> {
        self.storage.apply_update(builder).await
    }

    pub async fn add_message(&self, id: &str, message: &Message) -> Result<()> {
        self.storage.add_message(id, message).await
    }

    pub async fn replace_conversation(&self, id: &str, conversation: &Conversation) -> Result<()> {
        self.storage.replace_conversation(id, conversation).await
    }

    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        self.storage.list_sessions().await
    }

    pub async fn list_sessions_by_types(&self, types: &[SessionType]) -> Result<Vec<Session>> {
        self.storage.list_sessions_by_types(Some(types)).await
    }

    pub(crate) async fn list_nonempty_sessions_by_types_paged(
        &self,
        types: &[SessionType],
        working_dir: Option<&Path>,
        cursor: Option<&SessionListCursor>,
        page_size: usize,
    ) -> Result<SessionListPage> {
        self.storage
            .list_nonempty_sessions_by_types_paged(types, working_dir, cursor, page_size)
            .await
    }

    pub async fn list_all_sessions(&self) -> Result<Vec<Session>> {
        self.storage.list_sessions_by_types(None).await
    }

    pub async fn delete_session(&self, id: &str) -> Result<()> {
        self.storage.delete_session(id).await
    }

    pub async fn get_insights(&self) -> Result<SessionInsights> {
        self.storage
            .get_insights(&[SessionType::User, SessionType::Scheduled])
            .await
    }

    pub async fn export_session(&self, id: &str) -> Result<String> {
        self.storage.export_session(id).await
    }

    pub async fn import_session(
        &self,
        json: &str,
        session_type_override: Option<SessionType>,
    ) -> Result<Session> {
        self.storage
            .import_session(self, json, session_type_override)
            .await
    }

    pub async fn copy_session(&self, session_id: &str, new_name: String) -> Result<Session> {
        self.storage.copy_session(self, session_id, new_name).await
    }

    pub async fn truncate_conversation(&self, session_id: &str, timestamp: i64) -> Result<()> {
        self.storage
            .truncate_conversation(session_id, timestamp)
            .await
    }

    pub async fn maybe_update_name(
        &self,
        id: &str,
        provider: Arc<dyn Provider>,
    ) -> Result<Option<SessionNameUpdate>> {
        let session = self.get_session(id, true).await?;

        if session.user_set_name {
            return Ok(None);
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
            let name = provider.generate_session_name(id, &conversation).await?;
            self.update(id)
                .system_generated_name(name.clone())
                .apply()
                .await?;

            let session = self.get_session(id, false).await?;
            return Ok(Some(SessionNameUpdate {
                session_id: id.to_string(),
                name,
                updated_at: session.updated_at,
                message_count: session.message_count,
                user_set_name: session.user_set_name,
            }));
        }
        Ok(None)
    }

    pub async fn search_chat_history(
        &self,
        query: &str,
        limit: Option<usize>,
        after_date: Option<chrono::DateTime<chrono::Utc>>,
        before_date: Option<chrono::DateTime<chrono::Utc>>,
        exclude_session_id: Option<String>,
        session_types: Vec<SessionType>,
    ) -> Result<crate::session::chat_history_search::ChatRecallResults> {
        self.storage
            .search_chat_history(
                query,
                limit,
                after_date,
                before_date,
                exclude_session_id,
                session_types,
            )
            .await
    }

    pub async fn update_message_metadata<F>(id: &str, message_id: &str, f: F) -> Result<()>
    where
        F: FnOnce(
            crate::conversation::message::MessageMetadata,
        ) -> crate::conversation::message::MessageMetadata,
    {
        Self::instance()
            .storage
            .update_message_metadata(id, message_id, f)
            .await
    }

    /// Patch `tool_meta` on a specific `ToolRequest` within a stored message.
    /// Used to persist LLM-generated tool titles and chain summaries so they
    /// survive session reload. Merge-based: existing keys not in `patch` are
    /// preserved. No-op if the message or tool_call_id is not found.
    pub async fn update_tool_request_meta(
        &self,
        session_id: &str,
        message_id: &str,
        tool_call_id: &str,
        patch: serde_json::Value,
    ) -> Result<()> {
        self.storage
            .update_tool_request_meta(session_id, message_id, tool_call_id, patch)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{Message, MessageContent};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::{Pool, Sqlite};
    use tempfile::TempDir;
    use test_case::test_case;

    const NUM_CONCURRENT_SESSIONS: i32 = 10;

    async fn create_session_for_list(
        sm: &SessionManager,
        working_dir: &str,
        has_message: bool,
    ) -> String {
        let session = sm
            .create_session(
                PathBuf::from(working_dir),
                format!("Session in {working_dir}"),
                SessionType::User,
                GooseMode::default(),
            )
            .await
            .unwrap();

        if has_message {
            sm.add_message(&session.id, &Message::user().with_text("message"))
                .await
                .unwrap();
        }

        session.id
    }

    async fn set_sessions_updated_at(
        sm: &SessionManager,
        session_ids: &[String],
        updated_at: &str,
    ) {
        let pool = sm.storage().pool().await.unwrap();
        let updated_at = chrono::DateTime::parse_from_rfc3339(updated_at).unwrap();
        let timestamp = updated_at.format("%Y-%m-%d %H:%M:%S").to_string();

        for session_id in session_ids {
            sqlx::query("UPDATE sessions SET updated_at = ? WHERE id = ?")
                .bind(&timestamp)
                .bind(session_id)
                .execute(pool)
                .await
                .unwrap();
        }
    }

    async fn expected_session_list_ids(sm: &SessionManager, session_ids: &[String]) -> Vec<String> {
        let mut sessions = Vec::new();
        for session_id in session_ids {
            sessions.push(sm.get_session(session_id, false).await.unwrap());
        }
        sessions.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        sessions.into_iter().map(|session| session.id).collect()
    }

    async fn assert_session_list_page(
        sm: &SessionManager,
        cursor: Option<&SessionListCursor>,
        working_dir: Option<&str>,
        page_size: usize,
        expected_ids: &[String],
        expected_next_cursor: bool,
    ) -> Option<SessionListCursor> {
        let page = sm
            .list_nonempty_sessions_by_types_paged(
                &[SessionType::User],
                working_dir.map(Path::new),
                cursor,
                page_size,
            )
            .await
            .unwrap();
        let ids = page
            .sessions
            .iter()
            .map(|session| session.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(ids.as_slice(), expected_ids);
        assert_eq!(page.next_cursor.is_some(), expected_next_cursor);
        page.next_cursor
    }

    async fn run_lock_upgrade_attempt(
        pool: Pool<Sqlite>,
        session_id: String,
        begin_statement: &'static str,
        worker_id: i32,
        barrier: Option<Arc<tokio::sync::Barrier>>,
    ) -> anyhow::Result<()> {
        let mut tx = pool.begin_with(begin_statement).await?;

        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions WHERE id = ?")
            .bind(&session_id)
            .fetch_one(&mut *tx)
            .await?;

        if let Some(barrier) = barrier {
            barrier.wait().await;
        }

        sqlx::query("UPDATE sessions SET total_tokens = ? WHERE id = ?")
            .bind(worker_id)
            .bind(&session_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn run_lock_upgrade_race(
        pool: Pool<Sqlite>,
        session_id: String,
        begin_statement: &'static str,
        use_barrier: bool,
    ) -> Vec<anyhow::Result<()>> {
        let barrier = if use_barrier {
            Some(Arc::new(tokio::sync::Barrier::new(2)))
        } else {
            None
        };
        let mut handles = Vec::new();

        for worker_id in 0..2 {
            let pool = pool.clone();
            let session_id = session_id.clone();
            let barrier = barrier.clone();
            handles.push(tokio::spawn(async move {
                run_lock_upgrade_attempt(pool, session_id, begin_statement, worker_id, barrier)
                    .await
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.expect("lock-upgrade task panicked"));
        }
        results
    }

    #[tokio::test]
    async fn test_begin_immediate_prevents_lock_upgrade_deadlock() {
        let temp_dir = TempDir::new().unwrap();
        let session_manager = SessionManager::new(temp_dir.path().to_path_buf());

        let session = session_manager
            .create_session(
                PathBuf::from("/tmp/lock-upgrade-test"),
                "Lock Upgrade Session".to_string(),
                SessionType::User,
                GooseMode::default(),
            )
            .await
            .unwrap();

        let pool = session_manager.storage().pool.clone();

        let results = run_lock_upgrade_race(pool.clone(), session.id.clone(), "BEGIN", true).await;
        assert!(
            results.iter().any(Result::is_err),
            "BEGIN (DEFERRED) should cause SQLITE_BUSY when two tasks try to upgrade SHARED → RESERVED"
        );

        let results = run_lock_upgrade_race(pool, session.id, "BEGIN IMMEDIATE", false).await;
        assert!(
            results.iter().all(Result::is_ok),
            "BEGIN IMMEDIATE should serialize contention without SQLITE_BUSY: {:?}",
            results
                .iter()
                .filter_map(|r| r.as_ref().err().map(ToString::to_string))
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn test_session_list_paged_first_second_and_final_page() {
        let temp_dir = TempDir::new().unwrap();
        let sm = SessionManager::new(temp_dir.path().to_path_buf());
        let mut expected_ids = Vec::new();
        for _ in 0..5 {
            expected_ids.push(create_session_for_list(&sm, "/tmp/session-list", true).await);
        }
        let expected_ids = expected_session_list_ids(&sm, &expected_ids).await;

        let cursor = assert_session_list_page(&sm, None, None, 2, &expected_ids[0..2], true).await;
        let cursor =
            assert_session_list_page(&sm, cursor.as_ref(), None, 2, &expected_ids[2..4], true)
                .await;
        assert_session_list_page(&sm, cursor.as_ref(), None, 2, &expected_ids[4..5], false).await;
    }

    #[tokio::test]
    async fn test_session_list_paged_uses_id_tiebreaker_for_duplicate_updated_at() {
        let temp_dir = TempDir::new().unwrap();
        let sm = SessionManager::new(temp_dir.path().to_path_buf());
        let mut expected_ids = Vec::new();
        for _ in 0..3 {
            expected_ids.push(create_session_for_list(&sm, "/tmp/session-list", true).await);
        }
        set_sessions_updated_at(&sm, &expected_ids, "2024-01-01T00:00:00Z").await;
        let expected_ids = expected_session_list_ids(&sm, &expected_ids).await;

        let cursor = assert_session_list_page(&sm, None, None, 2, &expected_ids[0..2], true).await;
        assert_session_list_page(&sm, cursor.as_ref(), None, 2, &expected_ids[2..3], false).await;
    }

    #[tokio::test]
    async fn test_session_list_paged_filters_empty_and_cwd_before_pagination() {
        let temp_dir = TempDir::new().unwrap();
        let sm = SessionManager::new(temp_dir.path().to_path_buf());
        let expected_ids = vec![
            create_session_for_list(&sm, "/tmp/session-list/a", true).await,
            create_session_for_list(&sm, "/tmp/session-list/a", true).await,
        ];
        create_session_for_list(&sm, "/tmp/session-list/a", false).await;
        create_session_for_list(&sm, "/tmp/session-list/b", true).await;
        let expected_ids = expected_session_list_ids(&sm, &expected_ids).await;

        let cursor = assert_session_list_page(
            &sm,
            None,
            Some("/tmp/session-list/a"),
            1,
            &expected_ids[0..1],
            true,
        )
        .await;
        assert_session_list_page(
            &sm,
            cursor.as_ref(),
            Some("/tmp/session-list/a"),
            1,
            &expected_ids[1..2],
            false,
        )
        .await;
    }

    #[tokio::test]
    async fn test_concurrent_session_creation() {
        let temp_dir = TempDir::new().unwrap();
        let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));

        let mut handles = vec![];

        for i in 0..NUM_CONCURRENT_SESSIONS {
            let sm = Arc::clone(&session_manager);
            let handle = tokio::spawn(async move {
                let working_dir = PathBuf::from(format!("/tmp/test_{}", i));
                let description = format!("Test session {}", i);

                let session = sm
                    .create_session(
                        working_dir.clone(),
                        description,
                        SessionType::User,
                        GooseMode::default(),
                    )
                    .await
                    .unwrap();

                sm.add_message(
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

                sm.add_message(
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

                sm.update(&session.id)
                    .user_provided_name(format!("Updated session {}", i))
                    .total_tokens(Some(100 * i))
                    .apply()
                    .await
                    .unwrap();

                let updated = sm.get_session(&session.id, true).await.unwrap();
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

        let sessions = session_manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), NUM_CONCURRENT_SESSIONS as usize);

        for session in &sessions {
            assert_eq!(session.message_count, 2);
            assert!(session.name.starts_with("Updated session"));
        }

        let insights = session_manager.get_insights().await.unwrap();
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
        let sm = SessionManager::new(temp_dir.path().to_path_buf());

        let original = sm
            .create_session(
                PathBuf::from("/tmp/test"),
                DESCRIPTION.to_string(),
                SessionType::User,
                GooseMode::default(),
            )
            .await
            .unwrap();

        sm.update(&original.id)
            .total_tokens(Some(TOTAL_TOKENS))
            .input_tokens(Some(INPUT_TOKENS))
            .output_tokens(Some(OUTPUT_TOKENS))
            .accumulated_total_tokens(Some(ACCUMULATED_TOKENS))
            .apply()
            .await
            .unwrap();

        sm.add_message(
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

        sm.add_message(
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

        let exported = sm.export_session(&original.id).await.unwrap();
        let imported = sm.import_session(&exported, None).await.unwrap();

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
    async fn test_list_sessions_filters_by_type() {
        let temp_dir = TempDir::new().unwrap();
        let sm = SessionManager::new(temp_dir.path().to_path_buf());

        let user_session = sm
            .create_session(
                PathBuf::from("/tmp/test"),
                "User session".to_string(),
                SessionType::User,
                GooseMode::default(),
            )
            .await
            .unwrap();

        sm.add_message(
            &user_session.id,
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

        let acp_session = sm
            .create_session(
                PathBuf::from("/tmp/test"),
                "ACP session".to_string(),
                SessionType::Acp,
                GooseMode::default(),
            )
            .await
            .unwrap();

        sm.add_message(
            &acp_session.id,
            &Message {
                id: None,
                role: Role::User,
                created: chrono::Utc::now().timestamp_millis(),
                content: vec![MessageContent::text("hello acp")],
                metadata: Default::default(),
            },
        )
        .await
        .unwrap();

        let default_sessions = sm.list_sessions().await.unwrap();
        assert_eq!(default_sessions.len(), 1);
        assert_eq!(default_sessions[0].name, "User session");

        let acp_sessions = sm
            .list_sessions_by_types(&[SessionType::Acp])
            .await
            .unwrap();
        assert_eq!(acp_sessions.len(), 1);
        assert_eq!(acp_sessions[0].name, "ACP session");
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
        let sm = SessionManager::new(temp_dir.path().to_path_buf());

        let imported = sm.import_session(OLD_FORMAT_JSON, None).await.unwrap();

        assert_eq!(imported.name, "Old format session");
        assert!(imported.user_set_name);
        assert_eq!(imported.working_dir, PathBuf::from("/tmp/test"));
    }

    #[test_case(GooseMode::Approve)]
    #[test_case(GooseMode::SmartApprove)]
    #[test_case(GooseMode::Chat)]
    #[tokio::test]
    async fn test_goose_mode_persists(mode: GooseMode) {
        let temp_dir = TempDir::new().unwrap();
        let sm = SessionManager::new(temp_dir.path().to_path_buf());

        let session = sm
            .create_session(
                temp_dir.path().to_path_buf(),
                "test".into(),
                SessionType::User,
                mode,
            )
            .await
            .unwrap();

        let reloaded = sm.get_session(&session.id, false).await.unwrap();
        assert_eq!(reloaded.goose_mode, mode);
    }

    #[tokio::test]
    async fn test_goose_mode_update() {
        let temp_dir = TempDir::new().unwrap();
        let sm = SessionManager::new(temp_dir.path().to_path_buf());

        let session = sm
            .create_session(
                temp_dir.path().to_path_buf(),
                "test".into(),
                SessionType::User,
                GooseMode::default(),
            )
            .await
            .unwrap();

        sm.update(&session.id)
            .goose_mode(GooseMode::Approve)
            .apply()
            .await
            .unwrap();

        let reloaded = sm.get_session(&session.id, false).await.unwrap();
        assert_eq!(reloaded.goose_mode, GooseMode::Approve);
    }

    #[tokio::test]
    async fn test_goose_mode_malformed_defaults_to_auto() {
        let temp_dir = TempDir::new().unwrap();
        let sm = SessionManager::new(temp_dir.path().to_path_buf());

        let session = sm
            .create_session(
                temp_dir.path().to_path_buf(),
                "test".into(),
                SessionType::User,
                GooseMode::Approve,
            )
            .await
            .unwrap();

        let pool = &sm.storage().pool;
        sqlx::query("UPDATE sessions SET goose_mode = 'garbage' WHERE id = ?")
            .bind(&session.id)
            .execute(pool)
            .await
            .unwrap();

        let reloaded = sm.get_session(&session.id, false).await.unwrap();
        assert_eq!(reloaded.goose_mode, GooseMode::default());
    }

    #[tokio::test]
    async fn test_acp_session_migration() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join(SESSIONS_FOLDER).join(DB_NAME);

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        let pool = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&db_path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();

        SessionStorage::create_schema(&pool).await.unwrap();

        // Demote the schema back to v8 to simulate a database
        // that has never seen migration 9.
        sqlx::query("UPDATE schema_version SET version = 8")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO sessions (id, name, user_set_name, session_type, working_dir, extension_data, goose_mode)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("user_id")
        .bind("User Session")
        .bind(false)
        .bind("user")
        .bind("/tmp")
        .bind("{}")
        .bind("auto")
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO sessions (id, name, user_set_name, session_type, working_dir, extension_data, goose_mode)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("acp_id")
        .bind("ACP Session")
        .bind(false)
        .bind("user")
        .bind("/tmp")
        .bind("{}")
        .bind("auto")
        .execute(&pool)
        .await
        .unwrap();

        pool.close().await;

        let sm = SessionManager::new(temp_dir.path().to_path_buf());
        sm.storage().pool().await.unwrap(); // Triggers migration

        let user_session = sm.storage().get_session("user_id", false).await.unwrap();
        assert_eq!(user_session.session_type, SessionType::User);

        let acp_session = sm.storage().get_session("acp_id", false).await.unwrap();
        assert_eq!(acp_session.session_type, SessionType::Acp);
    }
}
