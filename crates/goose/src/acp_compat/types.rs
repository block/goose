//! Core ACP types: Run, Session, status enums.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::message::AcpMessage;

/// Run status per ACP v0.2.0 spec.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AcpRunStatus {
    Created,
    InProgress,
    Awaiting,
    Completed,
    Cancelled,
    Failed,
}

/// Run mode per ACP v0.2.0 spec.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    #[default]
    Sync,
    Async,
    Stream,
}

/// Request payload for creating a new run.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RunCreateRequest {
    pub agent_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub input: Vec<AcpMessage>,
    #[serde(default)]
    pub mode: RunMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Request payload for resuming an awaiting run.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RunResumeRequest {
    pub run_id: String,
    pub await_resume: AwaitResume,
    /// Required per ACP v0.2.0 spec.
    pub mode: RunMode,
}

/// A run object per ACP v0.2.0 spec.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcpRun {
    pub run_id: String,
    pub agent_name: String,
    pub status: AcpRunStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default)]
    pub output: Vec<AcpMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_request: Option<AwaitRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AcpError>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// ACP error object.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcpError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// ACP session object.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcpSession {
    pub id: String,
    #[serde(default)]
    pub history: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// Generic await request — sent when run enters "awaiting" state.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AwaitRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Generic await resume — sent by client to resume an awaiting run.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AwaitResume {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}
