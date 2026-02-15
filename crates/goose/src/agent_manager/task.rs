use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Failed,
    Canceled,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Submitted => write!(f, "submitted"),
            Self::Working => write!(f, "working"),
            Self::InputRequired => write!(f, "input-required"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Canceled => write!(f, "canceled"),
        }
    }
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    pub task_id: String,
    pub agent_id: String,
    pub state: TaskState,
    pub message: Option<String>,
    pub result: Option<String>,
    pub created_at_secs_ago: u64,
    pub updated_at_secs_ago: u64,
}

struct TaskEntry {
    task_id: String,
    agent_id: String,
    state: TaskState,
    message: Option<String>,
    result: Option<String>,
    created_at: Instant,
    updated_at: Instant,
}

impl TaskEntry {
    fn to_status(&self) -> TaskStatus {
        let now = Instant::now();
        TaskStatus {
            task_id: self.task_id.clone(),
            agent_id: self.agent_id.clone(),
            state: self.state,
            message: self.message.clone(),
            result: self.result.clone(),
            created_at_secs_ago: now.duration_since(self.created_at).as_secs(),
            updated_at_secs_ago: now.duration_since(self.updated_at).as_secs(),
        }
    }
}

#[derive(Clone)]
pub struct TaskManager {
    tasks: Arc<Mutex<HashMap<String, TaskEntry>>>,
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn submit_task(&self, agent_id: &str) -> String {
        let task_id = Uuid::new_v4().to_string();
        let now = Instant::now();
        let entry = TaskEntry {
            task_id: task_id.clone(),
            agent_id: agent_id.to_string(),
            state: TaskState::Submitted,
            message: None,
            result: None,
            created_at: now,
            updated_at: now,
        };
        self.tasks.lock().await.insert(task_id.clone(), entry);
        task_id
    }

    pub async fn update_state(&self, task_id: &str, state: TaskState) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(entry) = tasks.get_mut(task_id) {
            entry.state = state;
            entry.updated_at = Instant::now();
            true
        } else {
            false
        }
    }

    pub async fn set_working(&self, task_id: &str) -> bool {
        self.update_state(task_id, TaskState::Working).await
    }

    pub async fn complete(&self, task_id: &str, result: String) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(entry) = tasks.get_mut(task_id) {
            entry.state = TaskState::Completed;
            entry.result = Some(result);
            entry.updated_at = Instant::now();
            true
        } else {
            false
        }
    }

    pub async fn fail(&self, task_id: &str, message: String) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(entry) = tasks.get_mut(task_id) {
            entry.state = TaskState::Failed;
            entry.message = Some(message);
            entry.updated_at = Instant::now();
            true
        } else {
            false
        }
    }

    pub async fn cancel(&self, task_id: &str) -> bool {
        self.update_state(task_id, TaskState::Canceled).await
    }

    pub async fn get_status(&self, task_id: &str) -> Option<TaskStatus> {
        self.tasks.lock().await.get(task_id).map(|e| e.to_status())
    }

    pub async fn list_tasks(&self) -> Vec<TaskStatus> {
        self.tasks
            .lock()
            .await
            .values()
            .map(|e| e.to_status())
            .collect()
    }

    pub async fn list_agent_tasks(&self, agent_id: &str) -> Vec<TaskStatus> {
        self.tasks
            .lock()
            .await
            .values()
            .filter(|e| e.agent_id == agent_id)
            .map(|e| e.to_status())
            .collect()
    }

    pub async fn prune_completed(&self) -> usize {
        let mut tasks = self.tasks.lock().await;
        let before = tasks.len();
        tasks.retain(|_, e| !e.state.is_terminal());
        before - tasks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_submit_and_get_status() {
        let mgr = TaskManager::new();
        let id = mgr.submit_task("agent-1").await;
        let status = mgr.get_status(&id).await.unwrap();
        assert_eq!(status.state, TaskState::Submitted);
        assert_eq!(status.agent_id, "agent-1");
    }

    #[tokio::test]
    async fn test_task_lifecycle() {
        let mgr = TaskManager::new();
        let id = mgr.submit_task("agent-1").await;

        assert!(mgr.set_working(&id).await);
        assert_eq!(mgr.get_status(&id).await.unwrap().state, TaskState::Working);

        assert!(mgr.complete(&id, "done!".to_string()).await);
        let status = mgr.get_status(&id).await.unwrap();
        assert_eq!(status.state, TaskState::Completed);
        assert_eq!(status.result.as_deref(), Some("done!"));
    }

    #[tokio::test]
    async fn test_task_failure() {
        let mgr = TaskManager::new();
        let id = mgr.submit_task("agent-1").await;

        assert!(mgr.set_working(&id).await);
        assert!(mgr.fail(&id, "something broke".to_string()).await);

        let status = mgr.get_status(&id).await.unwrap();
        assert_eq!(status.state, TaskState::Failed);
        assert_eq!(status.message.as_deref(), Some("something broke"));
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let mgr = TaskManager::new();
        let id = mgr.submit_task("agent-1").await;
        assert!(mgr.cancel(&id).await);
        assert_eq!(
            mgr.get_status(&id).await.unwrap().state,
            TaskState::Canceled
        );
    }

    #[tokio::test]
    async fn test_list_agent_tasks() {
        let mgr = TaskManager::new();
        let _id1 = mgr.submit_task("agent-1").await;
        let _id2 = mgr.submit_task("agent-2").await;
        let _id3 = mgr.submit_task("agent-1").await;

        let tasks = mgr.list_agent_tasks("agent-1").await;
        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().all(|t| t.agent_id == "agent-1"));
    }

    #[tokio::test]
    async fn test_prune_completed_tasks() {
        let mgr = TaskManager::new();
        let id1 = mgr.submit_task("agent-1").await;
        let id2 = mgr.submit_task("agent-1").await;
        let id3 = mgr.submit_task("agent-1").await;

        mgr.complete(&id1, "done".to_string()).await;
        mgr.fail(&id2, "oops".to_string()).await;
        // id3 stays submitted

        let pruned = mgr.prune_completed().await;
        assert_eq!(pruned, 2);
        assert_eq!(mgr.list_tasks().await.len(), 1);
        assert!(mgr.get_status(&id3).await.is_some());
    }

    #[tokio::test]
    async fn test_terminal_state() {
        assert!(TaskState::Completed.is_terminal());
        assert!(TaskState::Failed.is_terminal());
        assert!(TaskState::Canceled.is_terminal());
        assert!(!TaskState::Submitted.is_terminal());
        assert!(!TaskState::Working.is_terminal());
        assert!(!TaskState::InputRequired.is_terminal());
    }

    #[tokio::test]
    async fn test_nonexistent_task_returns_none() {
        let mgr = TaskManager::new();
        assert!(mgr.get_status("nope").await.is_none());
        assert!(!mgr.set_working("nope").await);
        assert!(!mgr.complete("nope", "x".to_string()).await);
    }
}
