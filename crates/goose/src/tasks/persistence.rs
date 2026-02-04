//! Task Persistence for cross-session task management

#![allow(dead_code)]

use super::{Task, TaskId, TaskStatus};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Trait for task persistence backends
#[async_trait]
pub trait TaskPersistence: Send + Sync {
    async fn save(&self, task: &Task) -> Result<()>;
    async fn load(&self, id: &TaskId) -> Result<Option<Task>>;
    async fn load_all(&self) -> Result<Vec<Task>>;
    async fn delete(&self, id: &TaskId) -> Result<()>;
    async fn clear(&self) -> Result<()>;
}

/// In-memory task persistence (for testing)
pub struct MemoryTaskPersistence {
    tasks: RwLock<HashMap<TaskId, Task>>,
}

impl MemoryTaskPersistence {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryTaskPersistence {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskPersistence for MemoryTaskPersistence {
    async fn save(&self, task: &Task) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task.clone());
        Ok(())
    }

    async fn load(&self, id: &TaskId) -> Result<Option<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(id).cloned())
    }

    async fn load_all(&self) -> Result<Vec<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.values().cloned().collect())
    }

    async fn delete(&self, id: &TaskId) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(id);
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.clear();
        Ok(())
    }
}

/// SQLite task persistence for durable storage
pub struct SqliteTaskPersistence {
    db_path: PathBuf,
    pool: Option<sqlx::SqlitePool>,
}

impl SqliteTaskPersistence {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
            pool: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        if let Some(parent) = self.db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let db_url = format!("sqlite://{}?mode=rwc", self.db_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        self.create_tables(&pool).await?;
        self.pool = Some(pool);
        Ok(())
    }

    async fn create_tables(&self, pool: &sqlx::SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                subject TEXT NOT NULL,
                description TEXT,
                owner TEXT,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                dependencies TEXT,
                blockers TEXT,
                tags TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                result TEXT,
                metadata TEXT
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    fn pool(&self) -> Result<&sqlx::SqlitePool> {
        self.pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not connected"))
    }
}

#[async_trait]
impl TaskPersistence for SqliteTaskPersistence {
    async fn save(&self, task: &Task) -> Result<()> {
        let pool = self.pool()?;

        let dependencies = serde_json::to_string(&task.dependencies)?;
        let blockers = serde_json::to_string(&task.blockers)?;
        let tags = serde_json::to_string(&task.tags)?;
        let owner = task
            .owner
            .as_ref()
            .and_then(|o| serde_json::to_string(o).ok());
        let result = task
            .result
            .as_ref()
            .and_then(|r| serde_json::to_string(r).ok());
        let metadata = serde_json::to_string(&task.metadata)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO tasks (
                id, subject, description, owner, status, priority,
                dependencies, blockers, tags, created_at, updated_at,
                started_at, completed_at, result, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.id)
        .bind(&task.subject)
        .bind(&task.description)
        .bind(&owner)
        .bind(format!("{}", task.status))
        .bind(serde_json::to_string(&task.priority)?)
        .bind(&dependencies)
        .bind(&blockers)
        .bind(&tags)
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .bind(task.started_at.map(|t| t.to_rfc3339()))
        .bind(task.completed_at.map(|t| t.to_rfc3339()))
        .bind(&result)
        .bind(&metadata)
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn load(&self, id: &TaskId) -> Result<Option<Task>> {
        let pool = self.pool()?;

        let row: Option<TaskRow> = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(r) => Ok(Some(r.into_task()?)),
            None => Ok(None),
        }
    }

    async fn load_all(&self) -> Result<Vec<Task>> {
        let pool = self.pool()?;

        let rows: Vec<TaskRow> = sqlx::query_as("SELECT * FROM tasks ORDER BY created_at")
            .fetch_all(pool)
            .await?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row.into_task()?);
        }
        Ok(tasks)
    }

    async fn delete(&self, id: &TaskId) -> Result<()> {
        let pool = self.pool()?;
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let pool = self.pool()?;
        sqlx::query("DELETE FROM tasks").execute(pool).await?;
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct TaskRow {
    id: String,
    subject: String,
    description: Option<String>,
    owner: Option<String>,
    status: String,
    priority: String,
    dependencies: String,
    blockers: String,
    tags: String,
    created_at: String,
    updated_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    result: Option<String>,
    metadata: String,
}

impl TaskRow {
    fn into_task(self) -> Result<Task> {
        use super::{TaskOwner, TaskPriority, TaskResult};
        use chrono::DateTime;

        let status = match self.status.as_str() {
            "queued" => TaskStatus::Queued,
            "blocked" => TaskStatus::Blocked,
            "running" => TaskStatus::Running,
            "done" => TaskStatus::Done,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Queued,
        };

        let priority: TaskPriority =
            serde_json::from_str(&self.priority).unwrap_or(TaskPriority::Normal);

        let owner: Option<TaskOwner> = self
            .owner
            .as_ref()
            .and_then(|o| serde_json::from_str(o).ok());

        let result: Option<TaskResult> = self
            .result
            .as_ref()
            .and_then(|r| serde_json::from_str(r).ok());

        Ok(Task {
            id: self.id,
            subject: self.subject,
            description: self.description.unwrap_or_default(),
            owner,
            status,
            priority,
            dependencies: serde_json::from_str(&self.dependencies)?,
            blockers: serde_json::from_str(&self.blockers)?,
            tags: serde_json::from_str(&self.tags)?,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&chrono::Utc),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at)?.with_timezone(&chrono::Utc),
            started_at: self
                .started_at
                .as_ref()
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|t| t.with_timezone(&chrono::Utc)),
            completed_at: self
                .completed_at
                .as_ref()
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|t| t.with_timezone(&chrono::Utc)),
            result,
            metadata: serde_json::from_str(&self.metadata)?,
        })
    }
}

/// JSON file-based persistence for shared task lists
pub struct JsonTaskPersistence {
    file_path: PathBuf,
}

impl JsonTaskPersistence {
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
        }
    }

    async fn read_tasks(&self) -> Result<HashMap<TaskId, Task>> {
        if !self.file_path.exists() {
            return Ok(HashMap::new());
        }
        let content = tokio::fs::read_to_string(&self.file_path).await?;
        let tasks: HashMap<TaskId, Task> = serde_json::from_str(&content)?;
        Ok(tasks)
    }

    async fn write_tasks(&self, tasks: &HashMap<TaskId, Task>) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(tasks)?;
        tokio::fs::write(&self.file_path, content).await?;
        Ok(())
    }
}

#[async_trait]
impl TaskPersistence for JsonTaskPersistence {
    async fn save(&self, task: &Task) -> Result<()> {
        let mut tasks = self.read_tasks().await?;
        tasks.insert(task.id.clone(), task.clone());
        self.write_tasks(&tasks).await
    }

    async fn load(&self, id: &TaskId) -> Result<Option<Task>> {
        let tasks = self.read_tasks().await?;
        Ok(tasks.get(id).cloned())
    }

    async fn load_all(&self) -> Result<Vec<Task>> {
        let tasks = self.read_tasks().await?;
        Ok(tasks.into_values().collect())
    }

    async fn delete(&self, id: &TaskId) -> Result<()> {
        let mut tasks = self.read_tasks().await?;
        tasks.remove(id);
        self.write_tasks(&tasks).await
    }

    async fn clear(&self) -> Result<()> {
        self.write_tasks(&HashMap::new()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_persistence() {
        let persistence = MemoryTaskPersistence::new();

        let task = Task::new("task-1", "Test task");
        persistence.save(&task).await.unwrap();

        let loaded = persistence.load(&"task-1".to_string()).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().subject, "Test task");

        let all = persistence.load_all().await.unwrap();
        assert_eq!(all.len(), 1);

        persistence.delete(&"task-1".to_string()).await.unwrap();
        let loaded = persistence.load(&"task-1".to_string()).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_json_persistence() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_tasks.json");

        // Clean up any existing file
        let _ = std::fs::remove_file(&file_path);

        let persistence = JsonTaskPersistence::new(&file_path);

        let task = Task::new("task-1", "Test task");
        persistence.save(&task).await.unwrap();

        let loaded = persistence.load(&"task-1".to_string()).await.unwrap();
        assert!(loaded.is_some());

        persistence.clear().await.unwrap();
        let all = persistence.load_all().await.unwrap();
        assert!(all.is_empty());

        // Clean up
        let _ = std::fs::remove_file(&file_path);
    }
}
