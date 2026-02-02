//! SQLite-based checkpoint storage for durable persistence using sqlx

use super::{Checkpoint, CheckpointSummary, Checkpointer};
use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
use std::path::Path;

/// SQLite-based checkpointer for durable checkpoint storage
///
/// Stores checkpoints in a SQLite database for persistence across restarts.
/// Uses sqlx for async database access.
pub struct SqliteCheckpointer {
    /// Connection pool to the SQLite database
    pool: Pool<Sqlite>,
}

impl SqliteCheckpointer {
    /// Create a new SQLite checkpointer with the given database path
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Create the database file if it doesn't exist
        if !path.exists() {
            std::fs::File::create(path)?;
        }

        let database_url = format!("sqlite:{}", path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .with_context(|| format!("Failed to connect to SQLite database at {:?}", path))?;

        let checkpointer = Self { pool };

        // Initialize schema
        checkpointer.init_schema().await?;

        Ok(checkpointer)
    }

    /// Create an in-memory SQLite database (for testing)
    pub async fn in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        let checkpointer = Self { pool };
        checkpointer.init_schema().await?;
        Ok(checkpointer)
    }

    /// Initialize the database schema
    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS checkpoints (
                checkpoint_id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                parent_id TEXT,
                state TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_checkpoints_thread_id
                ON checkpoints(thread_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_checkpoints_created_at
                ON checkpoints(thread_id, created_at DESC)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Convert a timestamp to DateTime<Utc>
    fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(ts, 0).single().unwrap_or_else(Utc::now)
    }
}

#[async_trait::async_trait]
impl Checkpointer for SqliteCheckpointer {
    async fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        let state_json = serde_json::to_string(&checkpoint.state)?;
        let metadata_json = serde_json::to_string(&checkpoint.metadata)?;
        let created_at = checkpoint.created_at.timestamp();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO checkpoints
                (checkpoint_id, thread_id, parent_id, state, metadata, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&checkpoint.checkpoint_id)
        .bind(&checkpoint.thread_id)
        .bind(&checkpoint.parent_id)
        .bind(&state_json)
        .bind(&metadata_json)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load(&self, thread_id: &str) -> Result<Option<Checkpoint>> {
        let row = sqlx::query(
            r#"
            SELECT checkpoint_id, thread_id, parent_id, state, metadata, created_at
            FROM checkpoints
            WHERE thread_id = ?1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(thread_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let checkpoint_id: String = row.get("checkpoint_id");
                let thread_id: String = row.get("thread_id");
                let parent_id: Option<String> = row.get("parent_id");
                let state: String = row.get("state");
                let metadata: String = row.get("metadata");
                let created_at: i64 = row.get("created_at");

                Ok(Some(Checkpoint {
                    checkpoint_id,
                    thread_id,
                    parent_id,
                    state: serde_json::from_str(&state)?,
                    metadata: serde_json::from_str(&metadata)?,
                    created_at: Self::timestamp_to_datetime(created_at),
                }))
            }
            None => Ok(None),
        }
    }

    async fn load_by_id(&self, checkpoint_id: &str) -> Result<Option<Checkpoint>> {
        let row = sqlx::query(
            r#"
            SELECT checkpoint_id, thread_id, parent_id, state, metadata, created_at
            FROM checkpoints
            WHERE checkpoint_id = ?1
            "#,
        )
        .bind(checkpoint_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let checkpoint_id: String = row.get("checkpoint_id");
                let thread_id: String = row.get("thread_id");
                let parent_id: Option<String> = row.get("parent_id");
                let state: String = row.get("state");
                let metadata: String = row.get("metadata");
                let created_at: i64 = row.get("created_at");

                Ok(Some(Checkpoint {
                    checkpoint_id,
                    thread_id,
                    parent_id,
                    state: serde_json::from_str(&state)?,
                    metadata: serde_json::from_str(&metadata)?,
                    created_at: Self::timestamp_to_datetime(created_at),
                }))
            }
            None => Ok(None),
        }
    }

    async fn list(&self, thread_id: &str) -> Result<Vec<CheckpointSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT checkpoint_id, thread_id, parent_id, metadata, created_at
            FROM checkpoints
            WHERE thread_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(thread_id)
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = Vec::new();
        for row in rows {
            let checkpoint_id: String = row.get("checkpoint_id");
            let thread_id: String = row.get("thread_id");
            let parent_id: Option<String> = row.get("parent_id");
            let metadata: String = row.get("metadata");
            let created_at: i64 = row.get("created_at");

            summaries.push(CheckpointSummary {
                checkpoint_id,
                thread_id,
                parent_id,
                metadata: serde_json::from_str(&metadata)?,
                created_at: Self::timestamp_to_datetime(created_at),
            });
        }

        Ok(summaries)
    }

    async fn delete(&self, checkpoint_id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM checkpoints WHERE checkpoint_id = ?1")
            .bind(checkpoint_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete_thread(&self, thread_id: &str) -> Result<usize> {
        let result = sqlx::query("DELETE FROM checkpoints WHERE thread_id = ?1")
            .bind(thread_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::persistence::CheckpointMetadata;

    #[tokio::test]
    async fn test_sqlite_checkpointer_save_load() {
        let cp = SqliteCheckpointer::in_memory().await.unwrap();

        let checkpoint = Checkpoint::new("thread-1", serde_json::json!({"count": 1}));
        cp.save(&checkpoint).await.unwrap();

        let loaded = cp.load("thread-1").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().thread_id, "thread-1");
    }

    #[tokio::test]
    async fn test_sqlite_checkpointer_load_by_id() {
        let cp = SqliteCheckpointer::in_memory().await.unwrap();

        let checkpoint = Checkpoint::new("thread-1", serde_json::json!({"count": 1}));
        let id = checkpoint.checkpoint_id.clone();
        cp.save(&checkpoint).await.unwrap();

        let loaded = cp.load_by_id(&id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().checkpoint_id, id);
    }

    #[tokio::test]
    async fn test_sqlite_checkpointer_list() {
        let cp = SqliteCheckpointer::in_memory().await.unwrap();

        // Create multiple checkpoints with different timestamps (seconds apart to ensure ordering)
        let base_time = Utc::now();
        for i in 0..5 {
            let mut checkpoint = Checkpoint::new("thread-1", serde_json::json!({"count": i}));
            checkpoint.metadata = CheckpointMetadata::for_step(i, "Test");
            // Use seconds offset since SQLite stores timestamps as seconds
            checkpoint.created_at =
                base_time + chrono::Duration::try_seconds(i as i64).unwrap_or_default();
            cp.save(&checkpoint).await.unwrap();
        }

        let list = cp.list("thread-1").await.unwrap();
        assert_eq!(list.len(), 5);

        // Should be most recent first
        assert_eq!(list[0].metadata.step, Some(4));
        assert_eq!(list[4].metadata.step, Some(0));
    }

    #[tokio::test]
    async fn test_sqlite_checkpointer_delete() {
        let cp = SqliteCheckpointer::in_memory().await.unwrap();

        let checkpoint = Checkpoint::new("thread-1", serde_json::json!({"count": 1}));
        let id = checkpoint.checkpoint_id.clone();
        cp.save(&checkpoint).await.unwrap();

        let deleted = cp.delete(&id).await.unwrap();
        assert!(deleted);

        let loaded = cp.load_by_id(&id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_checkpointer_delete_thread() {
        let cp = SqliteCheckpointer::in_memory().await.unwrap();

        // Create checkpoints for two threads
        for i in 0..5 {
            cp.save(&Checkpoint::new(
                "thread-1",
                serde_json::json!({"count": i}),
            ))
            .await
            .unwrap();
            cp.save(&Checkpoint::new(
                "thread-2",
                serde_json::json!({"count": i}),
            ))
            .await
            .unwrap();
        }

        let deleted = cp.delete_thread("thread-1").await.unwrap();
        assert_eq!(deleted, 5);

        // thread-1 should be empty
        let list1 = cp.list("thread-1").await.unwrap();
        assert_eq!(list1.len(), 0);

        // thread-2 should still exist
        let list2 = cp.list("thread-2").await.unwrap();
        assert_eq!(list2.len(), 5);
    }

    #[tokio::test]
    async fn test_sqlite_checkpointer_parent_chain() {
        let cp = SqliteCheckpointer::in_memory().await.unwrap();

        // Create a chain of checkpoints
        let mut parent_id: Option<String> = None;
        for i in 0..3 {
            let mut checkpoint = Checkpoint::new("thread-1", serde_json::json!({"count": i}));
            if let Some(pid) = parent_id {
                checkpoint = checkpoint.with_parent(pid);
            }
            parent_id = Some(checkpoint.checkpoint_id.clone());
            cp.save(&checkpoint).await.unwrap();
        }

        // Get history from the last checkpoint
        let history = cp.get_history(&parent_id.unwrap()).await.unwrap();
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn test_sqlite_checkpointer_upsert() {
        let cp = SqliteCheckpointer::in_memory().await.unwrap();

        let mut checkpoint = Checkpoint::new("thread-1", serde_json::json!({"count": 1}));
        let id = checkpoint.checkpoint_id.clone();
        cp.save(&checkpoint).await.unwrap();

        // Update the same checkpoint
        checkpoint.state = serde_json::json!({"count": 2});
        cp.save(&checkpoint).await.unwrap();

        let loaded = cp.load_by_id(&id).await.unwrap().unwrap();
        assert_eq!(loaded.state["count"], 2);

        // Should only have one checkpoint
        let list = cp.list("thread-1").await.unwrap();
        assert_eq!(list.len(), 1);
    }
}
