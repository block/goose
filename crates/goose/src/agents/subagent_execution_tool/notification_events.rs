use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype")]
pub enum TaskExecutionNotificationEvent {
    #[serde(rename = "line_output")]
    LineOutput { task_id: String, output: String },
    #[serde(rename = "tasks_update")]
    TasksUpdate {
        stats: TaskExecutionStats,
        tasks: Vec<TaskInfo>,
    },
    #[serde(rename = "tasks_complete")]
    TasksComplete {
        stats: TaskCompletionStats,
        failed_tasks: Vec<FailedTaskInfo>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionStats {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletionStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub status: TaskStatus,
    pub duration_secs: Option<f64>,
    pub current_output: String,
    pub task_type: String,
    pub task_name: String,
    pub task_metadata: String,
    pub error: Option<String>,
    pub result_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedTaskInfo {
    pub id: String,
    pub name: String,
    pub error: Option<String>,
}

impl TaskExecutionNotificationEvent {
    pub fn line_output(task_id: String, output: String) -> Self {
        Self::LineOutput { task_id, output }
    }

    pub fn tasks_update(stats: TaskExecutionStats, tasks: Vec<TaskInfo>) -> Self {
        Self::TasksUpdate { stats, tasks }
    }

    pub fn tasks_complete(stats: TaskCompletionStats, failed_tasks: Vec<FailedTaskInfo>) -> Self {
        Self::TasksComplete {
            stats,
            failed_tasks,
        }
    }

    /// Convert event to JSON format for MCP notification
    pub fn to_notification_data(&self) -> serde_json::Value {
        let mut event_data = serde_json::to_value(self).expect("Failed to serialize event");

        // Add the type field at the root level
        if let serde_json::Value::Object(ref mut map) = event_data {
            map.insert(
                "type".to_string(),
                serde_json::Value::String("task_execution".to_string()),
            );
        }

        event_data
    }
}

impl TaskExecutionStats {
    pub fn new(
        total: usize,
        pending: usize,
        running: usize,
        completed: usize,
        failed: usize,
    ) -> Self {
        Self {
            total,
            pending,
            running,
            completed,
            failed,
        }
    }
}

impl TaskCompletionStats {
    pub fn new(total: usize, completed: usize, failed: usize) -> Self {
        let success_rate = if total > 0 {
            (completed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total,
            completed,
            failed,
            success_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_output_event_serialization() {
        let event = TaskExecutionNotificationEvent::line_output(
            "task-1".to_string(),
            "Hello World".to_string(),
        );

        let notification_data = event.to_notification_data();
        assert_eq!(notification_data["type"], "task_execution");
        assert_eq!(notification_data["subtype"], "line_output");
        assert_eq!(notification_data["task_id"], "task-1");
        assert_eq!(notification_data["output"], "Hello World");
    }

    #[test]
    fn test_tasks_update_event_serialization() {
        let stats = TaskExecutionStats::new(5, 2, 1, 1, 1);
        let tasks = vec![TaskInfo {
            id: "task-1".to_string(),
            status: TaskStatus::Running,
            duration_secs: Some(1.5),
            current_output: "Processing...".to_string(),
            task_type: "sub_recipe".to_string(),
            task_name: "test-task".to_string(),
            task_metadata: "param=value".to_string(),
            error: None,
            result_data: None,
        }];

        let event = TaskExecutionNotificationEvent::tasks_update(stats, tasks);
        let notification_data = event.to_notification_data();

        assert_eq!(notification_data["type"], "task_execution");
        assert_eq!(notification_data["subtype"], "tasks_update");
        assert_eq!(notification_data["stats"]["total"], 5);
        assert_eq!(notification_data["tasks"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_event_roundtrip_serialization() {
        let original_event = TaskExecutionNotificationEvent::line_output(
            "task-1".to_string(),
            "Test output".to_string(),
        );

        // Serialize to JSON
        let json_data = original_event.to_notification_data();

        // Deserialize back to event (excluding the type field)
        let mut event_data = json_data.clone();
        if let serde_json::Value::Object(ref mut map) = event_data {
            map.remove("type");
        }

        let deserialized_event: TaskExecutionNotificationEvent =
            serde_json::from_value(event_data).expect("Failed to deserialize");

        match (original_event, deserialized_event) {
            (
                TaskExecutionNotificationEvent::LineOutput {
                    task_id: id1,
                    output: out1,
                },
                TaskExecutionNotificationEvent::LineOutput {
                    task_id: id2,
                    output: out2,
                },
            ) => {
                assert_eq!(id1, id2);
                assert_eq!(out1, out2);
            }
            _ => panic!("Event types don't match after roundtrip"),
        }
    }
}
