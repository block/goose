//! Task Event System for real-time task state notifications

#![allow(dead_code)]

use super::{TaskId, TaskResult, TaskStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Events emitted by the task graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum TaskEvent {
    /// Task was created
    Created(TaskId),

    /// Task status changed
    StatusChanged {
        id: TaskId,
        old: TaskStatus,
        new: TaskStatus,
    },

    /// Task dependency was unblocked
    DependencyUnblocked { id: TaskId, unblocked_by: TaskId },

    /// Task completed successfully
    Completed { id: TaskId, result: TaskResult },

    /// Task failed
    Failed { id: TaskId, error: String },

    /// Task was cancelled
    Cancelled(TaskId),

    /// All tasks completed
    AllComplete {
        total: usize,
        succeeded: usize,
        failed: usize,
    },
}

impl TaskEvent {
    pub fn task_id(&self) -> Option<&TaskId> {
        match self {
            TaskEvent::Created(id) => Some(id),
            TaskEvent::StatusChanged { id, .. } => Some(id),
            TaskEvent::DependencyUnblocked { id, .. } => Some(id),
            TaskEvent::Completed { id, .. } => Some(id),
            TaskEvent::Failed { id, .. } => Some(id),
            TaskEvent::Cancelled(id) => Some(id),
            TaskEvent::AllComplete { .. } => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskEvent::Completed { .. } | TaskEvent::Failed { .. } | TaskEvent::Cancelled(_)
        )
    }
}

/// Timestamped task event for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedEvent {
    pub timestamp: DateTime<Utc>,
    pub run_id: String,
    pub event: TaskEvent,
}

impl TimestampedEvent {
    pub fn new(run_id: impl Into<String>, event: TaskEvent) -> Self {
        Self {
            timestamp: Utc::now(),
            run_id: run_id.into(),
            event,
        }
    }
}

/// Sender for task events
pub type TaskEventSender = broadcast::Sender<TaskEvent>;

/// Receiver for task events
pub type TaskEventReceiver = broadcast::Receiver<TaskEvent>;

/// Create a new event channel with the specified capacity
pub fn create_event_channel(capacity: usize) -> (TaskEventSender, TaskEventReceiver) {
    broadcast::channel(capacity)
}

/// Event logger that writes events to JSONL files
pub struct TaskEventLogger {
    run_id: String,
    log_path: std::path::PathBuf,
}

impl TaskEventLogger {
    pub fn new(run_id: impl Into<String>, log_dir: impl Into<std::path::PathBuf>) -> Self {
        let run_id = run_id.into();
        let log_dir = log_dir.into();
        let log_path = log_dir.join(format!("tasks_{}.jsonl", run_id));
        Self { run_id, log_path }
    }

    pub async fn log(&self, event: TaskEvent) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;

        let timestamped = TimestampedEvent::new(&self.run_id, event);
        let json = serde_json::to_string(&timestamped)?;

        if let Some(parent) = self.log_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;

        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    pub fn log_path(&self) -> &std::path::Path {
        &self.log_path
    }
}

/// Event aggregator for collecting statistics
#[derive(Debug, Default)]
pub struct TaskEventAggregator {
    pub created_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub cancelled_count: usize,
    pub events: Vec<TimestampedEvent>,
}

impl TaskEventAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, run_id: &str, event: TaskEvent) {
        match &event {
            TaskEvent::Created(_) => self.created_count += 1,
            TaskEvent::Completed { .. } => self.completed_count += 1,
            TaskEvent::Failed { .. } => self.failed_count += 1,
            TaskEvent::Cancelled(_) => self.cancelled_count += 1,
            _ => {}
        }
        self.events.push(TimestampedEvent::new(run_id, event));
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.completed_count + self.failed_count;
        if total == 0 {
            0.0
        } else {
            self.completed_count as f64 / total as f64
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Tasks: {} created, {} completed, {} failed, {} cancelled ({}% success rate)",
            self.created_count,
            self.completed_count,
            self.failed_count,
            self.cancelled_count,
            (self.success_rate() * 100.0) as u32
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_event_task_id() {
        let event = TaskEvent::Created("task-1".to_string());
        assert_eq!(event.task_id(), Some(&"task-1".to_string()));

        let event = TaskEvent::AllComplete {
            total: 5,
            succeeded: 4,
            failed: 1,
        };
        assert_eq!(event.task_id(), None);
    }

    #[test]
    fn test_task_event_is_terminal() {
        assert!(!TaskEvent::Created("task-1".to_string()).is_terminal());
        assert!(TaskEvent::Completed {
            id: "task-1".to_string(),
            result: TaskResult {
                success: true,
                output: None,
                error: None,
                artifacts: vec![],
                duration_ms: 100,
            }
        }
        .is_terminal());
        assert!(TaskEvent::Failed {
            id: "task-1".to_string(),
            error: "Error".to_string(),
        }
        .is_terminal());
    }

    #[test]
    fn test_event_aggregator() {
        let mut aggregator = TaskEventAggregator::new();

        aggregator.record("run-1", TaskEvent::Created("task-1".to_string()));
        aggregator.record("run-1", TaskEvent::Created("task-2".to_string()));
        aggregator.record(
            "run-1",
            TaskEvent::Completed {
                id: "task-1".to_string(),
                result: TaskResult {
                    success: true,
                    output: None,
                    error: None,
                    artifacts: vec![],
                    duration_ms: 100,
                },
            },
        );
        aggregator.record(
            "run-1",
            TaskEvent::Failed {
                id: "task-2".to_string(),
                error: "Error".to_string(),
            },
        );

        assert_eq!(aggregator.created_count, 2);
        assert_eq!(aggregator.completed_count, 1);
        assert_eq!(aggregator.failed_count, 1);
        assert_eq!(aggregator.success_rate(), 0.5);
    }

    #[test]
    fn test_timestamped_event_serialization() {
        let event = TimestampedEvent::new("run-1", TaskEvent::Created("task-1".to_string()));
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("task-1"));
        assert!(json.contains("run-1"));
    }
}
