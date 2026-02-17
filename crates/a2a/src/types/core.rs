//! Core A2A data model types mapped 1:1 from the a2a.proto specification.
//!
//! Proto source of truth: a2a.proto messages Task, Message, Part, Artifact, TaskStatus, TaskState, Role.

use serde::{Deserialize, Serialize};

/// Task lifecycle states per A2A proto `TaskState` enum.
///
/// Serialized as ProtoJSON SCREAMING_SNAKE_CASE per ADR-001.
/// Terminal states: Completed, Failed, Canceled, Rejected.
/// Interrupted states: InputRequired, AuthRequired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskState {
    #[serde(rename = "TASK_STATE_UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "TASK_STATE_SUBMITTED")]
    Submitted,
    #[serde(rename = "TASK_STATE_WORKING")]
    Working,
    #[serde(rename = "TASK_STATE_COMPLETED")]
    Completed,
    #[serde(rename = "TASK_STATE_FAILED")]
    Failed,
    #[serde(rename = "TASK_STATE_CANCELED")]
    Canceled,
    #[serde(rename = "TASK_STATE_INPUT_REQUIRED")]
    InputRequired,
    #[serde(rename = "TASK_STATE_REJECTED")]
    Rejected,
    #[serde(rename = "TASK_STATE_AUTH_REQUIRED")]
    AuthRequired,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Canceled | Self::Rejected
        )
    }

    pub fn is_interrupted(&self) -> bool {
        matches!(self, Self::InputRequired | Self::AuthRequired)
    }
}

/// Container for task status (proto `TaskStatus` message).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatus {
    pub state: TaskState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Box<Message>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Core task type (proto `Task` message).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub context_id: String,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Message sender role (proto `Role` enum).
///
/// Serialized as ProtoJSON SCREAMING_SNAKE_CASE per ADR-001.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "ROLE_UNSPECIFIED")]
    Unspecified,
    #[serde(rename = "ROLE_USER")]
    User,
    #[serde(rename = "ROLE_AGENT")]
    Agent,
}

/// A single message unit (proto `Message` message).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub role: Role,
    pub parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reference_task_ids: Vec<String>,
}

/// Part content variants (proto `Part.content` oneof).
///
/// The proto defines: text, raw (bytes), url, data (Struct/Value).
/// JSON serialization uses a `type` discriminator field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PartContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "file")]
    File {
        #[serde(skip_serializing_if = "Option::is_none")]
        raw: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    #[serde(rename = "data")]
    Data { data: serde_json::Value },
}

/// A content part within a message or artifact (proto `Part` message).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    #[serde(flatten)]
    pub content: PartContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

/// An output artifact (proto `Artifact` message).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub artifact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
}

impl Part {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: PartContent::Text { text: text.into() },
            metadata: None,
            filename: None,
            media_type: None,
        }
    }

    pub fn data(data: serde_json::Value) -> Self {
        Self {
            content: PartContent::Data { data },
            metadata: None,
            filename: None,
            media_type: None,
        }
    }

    pub fn file_url(url: impl Into<String>) -> Self {
        Self {
            content: PartContent::File {
                raw: None,
                url: Some(url.into()),
            },
            metadata: None,
            filename: None,
            media_type: None,
        }
    }
}

impl Message {
    pub fn user(parts: Vec<Part>) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            context_id: None,
            task_id: None,
            role: Role::User,
            parts,
            metadata: None,
            extensions: Vec::new(),
            reference_task_ids: Vec::new(),
        }
    }

    pub fn agent(parts: Vec<Part>) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            context_id: None,
            task_id: None,
            role: Role::Agent,
            parts,
            metadata: None,
            extensions: Vec::new(),
            reference_task_ids: Vec::new(),
        }
    }
}

impl Task {
    pub fn new(id: impl Into<String>, context_id: impl Into<String>, state: TaskState) -> Self {
        Self {
            id: id.into(),
            context_id: context_id.into(),
            status: TaskStatus {
                state,
                message: None,
                timestamp: Some(chrono::Utc::now().to_rfc3339()),
            },
            artifacts: Vec::new(),
            history: Vec::new(),
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_state_terminal() {
        assert!(TaskState::Completed.is_terminal());
        assert!(TaskState::Failed.is_terminal());
        assert!(TaskState::Canceled.is_terminal());
        assert!(TaskState::Rejected.is_terminal());
        assert!(!TaskState::Working.is_terminal());
        assert!(!TaskState::Submitted.is_terminal());
        assert!(!TaskState::InputRequired.is_terminal());
    }

    #[test]
    fn test_task_state_interrupted() {
        assert!(TaskState::InputRequired.is_interrupted());
        assert!(TaskState::AuthRequired.is_interrupted());
        assert!(!TaskState::Working.is_interrupted());
        assert!(!TaskState::Completed.is_interrupted());
    }

    #[test]
    fn test_task_state_serde_roundtrip() {
        let state = TaskState::InputRequired;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"TASK_STATE_INPUT_REQUIRED\"");
        let deserialized: TaskState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, state);
    }

    #[test]
    fn test_task_state_unspecified() {
        let state = TaskState::Unspecified;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"TASK_STATE_UNSPECIFIED\"");
        assert!(!state.is_terminal());
        assert!(!state.is_interrupted());
    }

    #[test]
    fn test_message_serde_roundtrip() {
        let msg = Message::user(vec![Part::text("Hello")]);
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "ROLE_USER");
        assert_eq!(json["parts"][0]["type"], "text");
        assert_eq!(json["parts"][0]["text"], "Hello");
        let deserialized: Message = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.role, Role::User);
    }

    #[test]
    fn test_task_serde_roundtrip() {
        let task = Task::new("task-1", "ctx-1", TaskState::Submitted);
        let json = serde_json::to_value(&task).unwrap();
        assert_eq!(json["id"], "task-1");
        assert_eq!(json["contextId"], "ctx-1");
        assert_eq!(json["status"]["state"], "TASK_STATE_SUBMITTED");
        let deserialized: Task = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.id, "task-1");
    }

    #[test]
    fn test_part_text() {
        let part = Part::text("hello world");
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "hello world");
    }

    #[test]
    fn test_part_file_url() {
        let part = Part::file_url("https://example.com/doc.pdf");
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["type"], "file");
        assert_eq!(json["url"], "https://example.com/doc.pdf");
    }

    #[test]
    fn test_part_data() {
        let part = Part::data(serde_json::json!({"key": "value"}));
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["type"], "data");
        assert_eq!(json["data"]["key"], "value");
    }

    #[test]
    fn test_artifact_serde() {
        let artifact = Artifact {
            artifact_id: "art-1".to_string(),
            name: Some("Report".to_string()),
            description: None,
            parts: vec![Part::text("content")],
            metadata: None,
            extensions: vec![],
        };
        let json = serde_json::to_value(&artifact).unwrap();
        assert_eq!(json["artifactId"], "art-1");
        assert_eq!(json["name"], "Report");
        assert!(json.get("description").is_none());
    }
}
