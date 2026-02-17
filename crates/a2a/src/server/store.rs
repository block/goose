//! Task store trait and in-memory implementation.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::A2AError;
use crate::types::core::Task;
use crate::types::requests::ListTasksRequest;
use crate::types::responses::ListTasksResponse;

/// Persistent storage for A2A tasks.
#[async_trait]
pub trait TaskStore: Send + Sync {
    async fn save(&self, task: &Task) -> Result<(), A2AError>;
    async fn load(&self, task_id: &str) -> Result<Option<Task>, A2AError>;
    async fn list(&self, request: &ListTasksRequest) -> Result<ListTasksResponse, A2AError>;
}

/// In-memory task store using a concurrent HashMap.
#[derive(Clone)]
pub struct InMemoryTaskStore {
    tasks: Arc<RwLock<HashMap<String, Task>>>,
}

impl InMemoryTaskStore {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryTaskStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskStore for InMemoryTaskStore {
    async fn save(&self, task: &Task) -> Result<(), A2AError> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task.clone());
        Ok(())
    }

    async fn load(&self, task_id: &str) -> Result<Option<Task>, A2AError> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(task_id).cloned())
    }

    async fn list(&self, request: &ListTasksRequest) -> Result<ListTasksResponse, A2AError> {
        let tasks = self.tasks.read().await;
        let mut filtered: Vec<&Task> = tasks.values().collect();

        if let Some(ref context_id) = request.context_id {
            filtered.retain(|t| t.context_id == *context_id);
        }

        if let Some(ref status) = request.status {
            filtered.retain(|t| t.status.state == *status);
        }

        // Sort by status timestamp descending (most recent first)
        filtered.sort_by(|a, b| {
            let ts_a = a.status.timestamp.as_deref().unwrap_or("");
            let ts_b = b.status.timestamp.as_deref().unwrap_or("");
            ts_b.cmp(ts_a)
        });

        let page_size = request.page_size.unwrap_or(50).min(100) as usize;
        let start = request
            .page_token
            .as_ref()
            .and_then(|t| t.parse::<usize>().ok())
            .unwrap_or(0);

        let page: Vec<Task> = filtered
            .into_iter()
            .skip(start)
            .take(page_size)
            .cloned()
            .collect();

        let next_token = if start + page_size < tasks.len() {
            (start + page_size).to_string()
        } else {
            String::new()
        };

        Ok(ListTasksResponse {
            total_size: tasks.len() as i32,
            page_size: page_size as i32,
            next_page_token: next_token,
            tasks: page,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::core::{Task, TaskState};

    #[tokio::test]
    async fn test_save_and_load() {
        let store = InMemoryTaskStore::new();
        let task = Task::new("t1", "ctx-1", TaskState::Submitted);

        store.save(&task).await.unwrap();
        let loaded = store.load("t1").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, "t1");
    }

    #[tokio::test]
    async fn test_load_missing() {
        let store = InMemoryTaskStore::new();
        let loaded = store.load("nonexistent").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_list_all() {
        let store = InMemoryTaskStore::new();
        store
            .save(&Task::new("t1", "ctx-1", TaskState::Submitted))
            .await
            .unwrap();
        store
            .save(&Task::new("t2", "ctx-1", TaskState::Working))
            .await
            .unwrap();

        let resp = store.list(&ListTasksRequest::default()).await.unwrap();
        assert_eq!(resp.total_size, 2);
        assert_eq!(resp.tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_list_filter_by_context() {
        let store = InMemoryTaskStore::new();
        store
            .save(&Task::new("t1", "ctx-1", TaskState::Submitted))
            .await
            .unwrap();
        store
            .save(&Task::new("t2", "ctx-2", TaskState::Submitted))
            .await
            .unwrap();

        let resp = store
            .list(&ListTasksRequest {
                context_id: Some("ctx-1".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(resp.tasks.len(), 1);
        assert_eq!(resp.tasks[0].id, "t1");
    }

    #[tokio::test]
    async fn test_list_filter_by_status() {
        let store = InMemoryTaskStore::new();
        store
            .save(&Task::new("t1", "ctx-1", TaskState::Submitted))
            .await
            .unwrap();
        store
            .save(&Task::new("t2", "ctx-1", TaskState::Working))
            .await
            .unwrap();

        let resp = store
            .list(&ListTasksRequest {
                status: Some(TaskState::Working),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(resp.tasks.len(), 1);
        assert_eq!(resp.tasks[0].id, "t2");
    }

    #[tokio::test]
    async fn test_save_overwrite() {
        let store = InMemoryTaskStore::new();
        let mut task = Task::new("t1", "ctx-1", TaskState::Submitted);
        store.save(&task).await.unwrap();

        task.status.state = TaskState::Working;
        store.save(&task).await.unwrap();

        let loaded = store.load("t1").await.unwrap().unwrap();
        assert_eq!(loaded.status.state, TaskState::Working);
    }
}
