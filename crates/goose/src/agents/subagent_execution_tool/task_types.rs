use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::agents::subagent_execution_tool::task_execution_tracker::TaskExecutionTracker;
use crate::recipe::Recipe;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    #[default]
    Sequential,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayload {
    pub recipe: Recipe,
    pub return_last_only: bool,
    pub sequential_when_repeated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_values: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub payload: TaskPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::Running => write!(f, "Running"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub task: Task,
    pub status: TaskStatus,
    pub start_time: Option<tokio::time::Instant>,
    pub end_time: Option<tokio::time::Instant>,
    pub result: Option<TaskResult>,
    pub current_output: String,
}

impl TaskInfo {
    pub fn error(&self) -> Option<&String> {
        self.result.as_ref().and_then(|r| r.error.as_ref())
    }

    pub fn data(&self) -> Option<&Value> {
        self.result.as_ref().and_then(|r| r.data.as_ref())
    }
}

pub struct SharedState {
    pub task_receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<Task>>>,
    pub result_sender: mpsc::Sender<TaskResult>,
    pub active_workers: Arc<AtomicUsize>,
    pub task_execution_tracker: Arc<TaskExecutionTracker>,
    pub cancellation_token: CancellationToken,
}

impl SharedState {
    pub fn increment_active_workers(&self) {
        self.active_workers.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_active_workers(&self) {
        self.active_workers.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Debug, Serialize)]
pub struct ExecutionStats {
    pub total_tasks: usize,
    pub completed: usize,
    pub failed: usize,
    pub execution_time_ms: u128,
}

#[derive(Debug, Serialize)]
pub struct ExecutionResponse {
    pub status: String,
    pub results: Vec<TaskResult>,
    pub stats: ExecutionStats,
}
