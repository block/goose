use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::agents::subagent_execution_tool::task_execution_tracker::TaskExecutionTracker;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    #[default]
    Sequential,
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    TextInstruction,
    InlineRecipe,
    SubRecipe,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskType::TextInstruction => write!(f, "text_instruction"),
            TaskType::InlineRecipe => write!(f, "inline_recipe"),
            TaskType::SubRecipe => write!(f, "sub_recipe"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub payload: Value,
}

impl Task {
    pub fn get_sub_recipe(&self) -> Option<&Map<String, Value>> {
        matches!(self.task_type, TaskType::SubRecipe)
            .then(|| self.payload.get("sub_recipe")?.as_object())
            .flatten()
    }

    pub fn get_command_parameters(&self) -> Option<&Map<String, Value>> {
        self.get_sub_recipe()
            .and_then(|sr| sr.get("command_parameters"))
            .and_then(|cp| cp.as_object())
    }

    pub fn get_sequential_when_repeated(&self) -> bool {
        self.get_sub_recipe()
            .and_then(|sr| sr.get("sequential_when_repeated").and_then(|v| v.as_bool()))
            .unwrap_or_default()
    }

    pub fn get_sub_recipe_name(&self) -> Option<&str> {
        self.get_sub_recipe()
            .and_then(|sr| sr.get("name"))
            .and_then(|name| name.as_str())
    }

    pub fn get_sub_recipe_path(&self) -> Option<&str> {
        self.get_sub_recipe()
            .and_then(|sr| sr.get("recipe_path"))
            .and_then(|path| path.as_str())
    }

    pub fn get_text_instruction(&self) -> Option<&str> {
        if !matches!(self.task_type, TaskType::SubRecipe) {
            self.payload
                .get("text_instruction")
                .and_then(|text| text.as_str())
        } else {
            None
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_task_type_serialization() {
        // Test that TaskType serializes to the expected string format
        assert_eq!(
            serde_json::to_string(&TaskType::TextInstruction).unwrap(),
            "\"text_instruction\""
        );
        assert_eq!(
            serde_json::to_string(&TaskType::InlineRecipe).unwrap(),
            "\"inline_recipe\""
        );
        assert_eq!(
            serde_json::to_string(&TaskType::SubRecipe).unwrap(),
            "\"sub_recipe\""
        );
    }

    #[test]
    fn test_task_type_deserialization() {
        // Test that strings deserialize to the correct TaskType variants
        assert_eq!(
            serde_json::from_str::<TaskType>("\"text_instruction\"").unwrap(),
            TaskType::TextInstruction
        );
        assert_eq!(
            serde_json::from_str::<TaskType>("\"inline_recipe\"").unwrap(),
            TaskType::InlineRecipe
        );
        assert_eq!(
            serde_json::from_str::<TaskType>("\"sub_recipe\"").unwrap(),
            TaskType::SubRecipe
        );
    }

    #[test]
    fn test_task_serialization_with_enum() {
        let task = Task {
            id: "test-id".to_string(),
            task_type: TaskType::InlineRecipe,
            payload: json!({"recipe": "test"}),
        };

        let serialized = serde_json::to_value(&task).unwrap();
        assert_eq!(serialized["id"], "test-id");
        assert_eq!(serialized["task_type"], "inline_recipe");
        assert_eq!(serialized["payload"]["recipe"], "test");
    }

    #[test]
    fn test_task_deserialization_with_string() {
        // Test backward compatibility - JSON with string task_type should deserialize
        let json_str = r#"{
            "id": "test-id",
            "task_type": "sub_recipe",
            "payload": {"sub_recipe": {"name": "test"}}
        }"#;

        let task: Task = serde_json::from_str(json_str).unwrap();
        assert_eq!(task.id, "test-id");
        assert_eq!(task.task_type, TaskType::SubRecipe);
    }

    #[test]
    fn test_task_type_display() {
        assert_eq!(TaskType::TextInstruction.to_string(), "text_instruction");
        assert_eq!(TaskType::InlineRecipe.to_string(), "inline_recipe");
        assert_eq!(TaskType::SubRecipe.to_string(), "sub_recipe");
    }
}
