//! Streaming event types mapped from a2a.proto.

use serde::{Deserialize, Serialize};

use super::core::{Artifact, Message, Task, TaskStatus};

/// Task status change event (proto `TaskStatusUpdateEvent`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusUpdateEvent {
    pub task_id: String,
    pub context_id: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Artifact generation/update event (proto `TaskArtifactUpdateEvent`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskArtifactUpdateEvent {
    pub task_id: String,
    pub context_id: String,
    pub artifact: Artifact,
    #[serde(default)]
    pub append: bool,
    #[serde(default)]
    pub last_chunk: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Streaming response wrapper (proto `StreamResponse` oneof).
///
/// Used for SSE streaming in both JSON-RPC and REST bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StreamResponse {
    #[serde(rename = "task")]
    Task(Task),
    #[serde(rename = "message")]
    Message(Message),
    #[serde(rename = "status-update")]
    StatusUpdate(TaskStatusUpdateEvent),
    #[serde(rename = "artifact-update")]
    ArtifactUpdate(TaskArtifactUpdateEvent),
}

/// Agent execution event emitted by an `AgentExecutor`.
#[derive(Debug, Clone)]
pub enum AgentExecutionEvent {
    StatusUpdate(TaskStatusUpdateEvent),
    ArtifactUpdate(TaskArtifactUpdateEvent),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::core::{TaskState, TaskStatus};

    #[test]
    fn test_status_update_event_serde() {
        let event = TaskStatusUpdateEvent {
            task_id: "task-1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Working,
                message: None,
                timestamp: Some("2025-01-01T00:00:00Z".to_string()),
            },
            metadata: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["taskId"], "task-1");
        assert_eq!(json["status"]["state"], "TASK_STATE_WORKING");
    }

    #[test]
    fn test_stream_response_status_update() {
        let response = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
            task_id: "task-1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Completed,
                message: None,
                timestamp: None,
            },
            metadata: None,
        });
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["kind"], "status-update");
        assert_eq!(json["status"]["state"], "TASK_STATE_COMPLETED");
    }
}
