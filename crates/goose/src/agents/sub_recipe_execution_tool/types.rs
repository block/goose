use serde::{Deserialize, Serialize};
use serde_json::Value;

// Task definition that LLMs will send
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: String,
    pub payload: Value,
}

// Result for each task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// Configuration for the parallel executor
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_initial_workers")]
    pub initial_workers: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_workers: default_max_workers(),
            timeout_seconds: default_timeout(),
            initial_workers: default_initial_workers(),
        }
    }
}

fn default_max_workers() -> usize {
    10
}
fn default_timeout() -> u64 {
    300
}
fn default_initial_workers() -> usize {
    2
}

// Stats for the execution
#[derive(Debug, Serialize)]
pub struct ExecutionStats {
    pub total_tasks: usize,
    pub completed: usize,
    pub failed: usize,
    pub execution_time_ms: u128,
}

// Main response structure
#[derive(Debug, Serialize)]
pub struct ExecutionResponse {
    pub status: String,
    pub results: Vec<TaskResult>,
    pub stats: ExecutionStats,
}
