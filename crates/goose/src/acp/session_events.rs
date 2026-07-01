use crate::session::session_manager::{DB_NAME, SESSIONS_FOLDER};
use agent_client_protocol::schema::v1::SessionNotification;
use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::OnceCell;

const MAX_STORED_EVENTS: i64 = 10_000;
const EVENT_RETENTION_SECONDS: i64 = 10 * 60;
const PRUNE_EVERY_EVENTS: u64 = 128;

pub(crate) struct AcpSessionEvent {
    pub(crate) id: i64,
    pub(crate) notification: SessionNotification,
}

pub(crate) struct AcpSessionEventStore {
    pool: Pool<Sqlite>,
    initialized: OnceCell<()>,
    published_event_count: AtomicU64,
}

impl AcpSessionEventStore {
    pub(crate) fn new(data_dir: PathBuf) -> Self {
        let db_path = data_dir.join(SESSIONS_FOLDER).join(DB_NAME);
        Self {
            pool: Self::create_pool(&db_path),
            initialized: OnceCell::new(),
            published_event_count: AtomicU64::new(0),
        }
    }

    fn create_pool(path: &Path) -> Pool<Sqlite> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create session database directory");
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(30))
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        SqlitePoolOptions::new().connect_lazy_with(options)
    }

    async fn pool(&self) -> Result<&Pool<Sqlite>> {
        self.initialized
            .get_or_try_init(|| async {
                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS acp_session_events (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        source_id TEXT NOT NULL,
                        session_id TEXT NOT NULL,
                        notification_json TEXT NOT NULL,
                        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )
                    "#,
                )
                .execute(&self.pool)
                .await?;

                sqlx::query(
                    r#"
                    CREATE INDEX IF NOT EXISTS idx_acp_session_events_id_source
                    ON acp_session_events(id, source_id)
                    "#,
                )
                .execute(&self.pool)
                .await?;

                sqlx::query(
                    r#"
                    CREATE INDEX IF NOT EXISTS idx_acp_session_events_created_at
                    ON acp_session_events(created_at)
                    "#,
                )
                .execute(&self.pool)
                .await?;

                sqlx::query(
                    r#"
                    CREATE INDEX IF NOT EXISTS idx_acp_session_events_session_id
                    ON acp_session_events(session_id)
                    "#,
                )
                .execute(&self.pool)
                .await?;

                Self::prune_events(&self.pool).await?;

                Ok::<(), anyhow::Error>(())
            })
            .await?;
        Ok(&self.pool)
    }

    async fn session_table_exists(pool: &Pool<Sqlite>) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM sqlite_master
                WHERE type = 'table' AND name = 'sessions'
            )
            "#,
        )
        .fetch_one(pool)
        .await?;
        Ok(exists)
    }

    async fn prune_events(pool: &Pool<Sqlite>) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM acp_session_events
            WHERE created_at < datetime('now', '-' || ? || ' seconds')
            "#,
        )
        .bind(EVENT_RETENTION_SECONDS)
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            DELETE FROM acp_session_events
            WHERE id <= (
                SELECT COALESCE(MAX(id), 0) - ?
                FROM acp_session_events
            )
            "#,
        )
        .bind(MAX_STORED_EVENTS)
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn insert_event(
        pool: &Pool<Sqlite>,
        source_id: &str,
        session_id: &str,
        notification_json: String,
    ) -> Result<u64> {
        let result = if Self::session_table_exists(pool).await? {
            sqlx::query(
                r#"
                INSERT INTO acp_session_events (source_id, session_id, notification_json)
                SELECT ?, ?, ?
                WHERE EXISTS (SELECT 1 FROM sessions WHERE id = ?)
                "#,
            )
            .bind(source_id)
            .bind(session_id)
            .bind(notification_json)
            .bind(session_id)
            .execute(pool)
            .await?
        } else {
            sqlx::query(
                r#"
                INSERT INTO acp_session_events (source_id, session_id, notification_json)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(source_id)
            .bind(session_id)
            .bind(notification_json)
            .execute(pool)
            .await?
        };

        Ok(result.rows_affected())
    }

    fn should_prune_after_publish(&self) -> bool {
        let count = self.published_event_count.fetch_add(1, Ordering::Relaxed) + 1;
        count.is_multiple_of(PRUNE_EVERY_EVENTS)
    }

    pub(crate) async fn latest_event_id(&self) -> Result<i64> {
        let pool = self.pool().await?;
        let id = sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(id) FROM acp_session_events")
            .fetch_one(pool)
            .await?
            .unwrap_or(0);
        Ok(id)
    }

    pub(crate) async fn publish(
        &self,
        source_id: &str,
        notification: &SessionNotification,
    ) -> Result<()> {
        let pool = self.pool().await?;
        let session_id = notification.session_id.0.as_ref();
        let notification_json =
            serde_json::to_string(notification).context("serialize ACP session notification")?;
        let inserted = Self::insert_event(pool, source_id, session_id, notification_json).await?;
        if inserted == 0 {
            return Ok(());
        }

        if self.should_prune_after_publish() {
            Self::prune_events(pool).await?;
        }

        Ok(())
    }

    pub(crate) async fn events_after(
        &self,
        last_seen_id: i64,
        source_id: &str,
        limit: i64,
    ) -> Result<Vec<AcpSessionEvent>> {
        let pool = self.pool().await?;
        let rows = sqlx::query(
            r#"
            SELECT id, notification_json
            FROM acp_session_events
            WHERE id > ? AND source_id != ?
            ORDER BY id ASC
            LIMIT ?
            "#,
        )
        .bind(last_seen_id)
        .bind(source_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let id: i64 = row.try_get("id")?;
                let notification_json: String = row.try_get("notification_json")?;
                let notification = serde_json::from_str(&notification_json)
                    .with_context(|| format!("deserialize ACP session event {id}"))?;
                Ok(AcpSessionEvent { id, notification })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GooseMode;
    use crate::session::{SessionManager, SessionType};
    use agent_client_protocol::schema::v1::{
        ContentBlock, ContentChunk, SessionId, SessionUpdate, TextContent,
    };
    use std::path::PathBuf;

    fn notification(session_id: &str, text: &str) -> SessionNotification {
        SessionNotification::new(
            SessionId::new(session_id.to_string()),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(text.to_string()),
            ))),
        )
    }

    #[tokio::test]
    async fn events_after_excludes_same_source_and_preserves_order() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        let store = AcpSessionEventStore::new(data_dir.clone());
        let session_manager = SessionManager::new(data_dir);
        let session = session_manager
            .create_session(
                PathBuf::from("/tmp/acp-session-event-order"),
                "event order".to_string(),
                SessionType::Acp,
                GooseMode::default(),
            )
            .await
            .unwrap();

        let baseline = store.latest_event_id().await.unwrap();
        store
            .publish("source-a", &notification(&session.id, "first"))
            .await
            .unwrap();
        store
            .publish("source-b", &notification(&session.id, "second"))
            .await
            .unwrap();
        store
            .publish("source-c", &notification(&session.id, "third"))
            .await
            .unwrap();

        let events = store.events_after(baseline, "source-a", 100).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].notification.session_id.0.as_ref(), session.id);
        assert_eq!(events[1].notification.session_id.0.as_ref(), session.id);
        assert!(events[0].id > baseline);
        assert!(events[1].id > events[0].id);

        let value = serde_json::to_value(&events[0].notification).unwrap();
        assert_eq!(
            value["update"]["content"]["text"],
            serde_json::Value::String("second".to_string())
        );
        let value = serde_json::to_value(&events[1].notification).unwrap();
        assert_eq!(
            value["update"]["content"]["text"],
            serde_json::Value::String("third".to_string())
        );
    }

    #[tokio::test]
    async fn events_after_reads_multiple_batches_in_order() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        let store = AcpSessionEventStore::new(data_dir.clone());
        let session_manager = SessionManager::new(data_dir);
        let session = session_manager
            .create_session(
                PathBuf::from("/tmp/acp-session-event-batches"),
                "event batches".to_string(),
                SessionType::Acp,
                GooseMode::default(),
            )
            .await
            .unwrap();

        let baseline = store.latest_event_id().await.unwrap();
        for index in 0..5 {
            store
                .publish(
                    "source-a",
                    &notification(&session.id, &format!("event-{index}")),
                )
                .await
                .unwrap();
        }

        let mut cursor = baseline;
        let mut texts = Vec::new();
        loop {
            let events = store.events_after(cursor, "source-b", 2).await.unwrap();
            if events.is_empty() {
                break;
            }
            cursor = events.last().unwrap().id;
            texts.extend(events.into_iter().map(|event| {
                let value = serde_json::to_value(&event.notification).unwrap();
                value["update"]["content"]["text"]
                    .as_str()
                    .unwrap()
                    .to_string()
            }));
        }

        assert_eq!(
            texts,
            vec!["event-0", "event-1", "event-2", "event-3", "event-4"]
        );
    }

    #[tokio::test]
    async fn delete_session_removes_events_for_session_and_preserves_others() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        let store = AcpSessionEventStore::new(data_dir.clone());
        let session_manager = SessionManager::new(data_dir);
        let deleted_session = session_manager
            .create_session(
                PathBuf::from("/tmp/acp-session-event-delete"),
                "event delete".to_string(),
                SessionType::Acp,
                GooseMode::default(),
            )
            .await
            .unwrap();
        let retained_session = session_manager
            .create_session(
                PathBuf::from("/tmp/acp-session-event-retain"),
                "event retain".to_string(),
                SessionType::Acp,
                GooseMode::default(),
            )
            .await
            .unwrap();

        let baseline = store.latest_event_id().await.unwrap();
        store
            .publish("source-a", &notification(&deleted_session.id, "first"))
            .await
            .unwrap();
        store
            .publish("source-b", &notification(&retained_session.id, "second"))
            .await
            .unwrap();

        session_manager
            .delete_session(&deleted_session.id)
            .await
            .unwrap();

        let events = store.events_after(baseline, "source-c", 100).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].notification.session_id.0.as_ref(),
            retained_session.id
        );
    }

    #[tokio::test]
    async fn publish_skips_events_for_deleted_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        let store = AcpSessionEventStore::new(data_dir.clone());
        let session_manager = SessionManager::new(data_dir);
        let session = session_manager
            .create_session(
                PathBuf::from("/tmp/acp-session-event-deleted-publish"),
                "event deleted publish".to_string(),
                SessionType::Acp,
                GooseMode::default(),
            )
            .await
            .unwrap();

        let baseline = store.latest_event_id().await.unwrap();
        session_manager.delete_session(&session.id).await.unwrap();
        store
            .publish("source-a", &notification(&session.id, "deleted"))
            .await
            .unwrap();

        let events = store.events_after(baseline, "source-b", 100).await.unwrap();
        assert!(events.is_empty());
    }
}
