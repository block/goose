//! Task Graph System for Goose
//!
//! Provides Claude Code-style task management with:
//! - DAG-based dependencies
//! - Parallel execution with concurrency limits
//! - Event streaming (queued → running → done/failed)
//! - Cross-session persistence

mod events;
mod graph;
mod persistence;

pub use events::{TaskEvent, TaskEventReceiver, TaskEventSender};
pub use graph::{TaskGraph, TaskGraphBuilder, TaskGraphConfig};
pub use persistence::{MemoryTaskPersistence, SqliteTaskPersistence, TaskPersistence};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a task
pub type TaskId = String;

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

/// Owner/role that should execute the task
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOwner {
    Builder,
    Validator,
    Orchestrator,
    Custom(String),
}

/// Task status in the execution lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Queued,
    Blocked,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Queued => write!(f, "queued"),
            TaskStatus::Blocked => write!(f, "blocked"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Done => write!(f, "done"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub artifacts: Vec<String>,
    pub duration_ms: u64,
}

/// A task in the task graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub subject: String,
    pub description: String,
    pub owner: Option<TaskOwner>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub dependencies: Vec<TaskId>,
    pub blockers: Vec<TaskId>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<TaskResult>,
    pub metadata: HashMap<String, String>,
}

impl Task {
    pub fn new(id: impl Into<String>, subject: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            subject: subject.into(),
            description: String::new(),
            owner: None,
            status: TaskStatus::Queued,
            priority: TaskPriority::Normal,
            dependencies: Vec::new(),
            blockers: Vec::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            result: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_owner(mut self, owner: TaskOwner) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<TaskId>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn is_ready(&self) -> bool {
        self.status == TaskStatus::Queued && self.blockers.is_empty()
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Done | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }
}

/// Update operations for a task
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskUpdate {
    pub status: Option<TaskStatus>,
    pub result: Option<TaskResult>,
    pub add_blockers: Vec<TaskId>,
    pub remove_blockers: Vec<TaskId>,
    pub metadata: HashMap<String, String>,
}

impl TaskUpdate {
    pub fn status(status: TaskStatus) -> Self {
        Self {
            status: Some(status),
            ..Default::default()
        }
    }

    pub fn complete(result: TaskResult) -> Self {
        Self {
            status: Some(if result.success {
                TaskStatus::Done
            } else {
                TaskStatus::Failed
            }),
            result: Some(result),
            ..Default::default()
        }
    }

    pub fn add_blocker(blocker: TaskId) -> Self {
        Self {
            add_blockers: vec![blocker],
            ..Default::default()
        }
    }

    pub fn remove_blocker(blocker: TaskId) -> Self {
        Self {
            remove_blockers: vec![blocker],
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("task-1", "Test task")
            .with_description("A test task")
            .with_owner(TaskOwner::Builder)
            .with_priority(TaskPriority::High);

        assert_eq!(task.id, "task-1");
        assert_eq!(task.subject, "Test task");
        assert_eq!(task.description, "A test task");
        assert_eq!(task.owner, Some(TaskOwner::Builder));
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.status, TaskStatus::Queued);
        assert!(task.is_ready());
    }

    #[test]
    fn test_task_with_dependencies() {
        let task =
            Task::new("task-2", "Dependent task").with_dependencies(vec!["task-1".to_string()]);

        assert_eq!(task.dependencies, vec!["task-1".to_string()]);
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(format!("{}", TaskStatus::Queued), "queued");
        assert_eq!(format!("{}", TaskStatus::Running), "running");
        assert_eq!(format!("{}", TaskStatus::Done), "done");
    }

    #[test]
    fn test_task_is_terminal() {
        let mut task = Task::new("task-1", "Test");
        assert!(!task.is_terminal());

        task.status = TaskStatus::Done;
        assert!(task.is_terminal());

        task.status = TaskStatus::Failed;
        assert!(task.is_terminal());

        task.status = TaskStatus::Cancelled;
        assert!(task.is_terminal());
    }
}
