//! Shared data types for TUI state and agent↔UI messaging.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

// ── Conversation model ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct Turn {
    pub user_text: String,
    /// Ordered list of tool call IDs (arrival order).
    pub tool_call_order: Vec<String>,
    pub tool_calls: HashMap<String, ToolCallInfo>,
    /// Final rendered (markdown → plain text) agent reply.
    pub agent_text: String,
}

#[derive(Clone, Debug)]
pub struct ToolCallInfo {
    pub id: String,
    pub title: String,
    pub status: ToolStatus,
    pub input_preview: Option<String>,
    pub output_preview: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolStatus {
    Pending,
    Running,
    Success,
    Error,
}

impl ToolStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::Running => "◐",
            Self::Success => "✓",
            Self::Error   => "✗",
        }
    }
}

// ── Agent ↔ UI messages ───────────────────────────────────────────────────────

/// Messages the agent background task sends to the UI event loop.
#[derive(Debug)]
pub enum AgentMsg {
    TextChunk(String),
    ToolCallUpdate(ToolCallInfo),
    /// Agent needs permission; the `Sender` must be resolved before the agent
    /// proceeds.  The UI stores it and sends a reply once the user decides.
    PermissionRequest(PermissionReq, oneshot::Sender<PermissionChoice>),
    Finished { stop_reason: String },
    Error(String),
}

#[derive(Clone, Debug)]
pub struct PermissionReq {
    pub tool_title: String,
    pub options: Vec<PermissionOption>,
}

#[derive(Clone, Debug)]
pub struct PermissionOption {
    pub id: String,
    pub label: String,
    pub key: char,
}

#[derive(Debug)]
pub enum PermissionChoice {
    Selected(String), // option id
    Cancelled,
}

// ── Shared reply-sender handle ────────────────────────────────────────────────

/// An `Arc<Mutex<Option<…>>>` wrapper so we can store a non-Clone, non-Default
/// `oneshot::Sender` in iocraft `State`.
#[derive(Clone, Default)]
pub struct PendingReply(pub Arc<Mutex<Option<oneshot::Sender<PermissionChoice>>>>);

impl PendingReply {
    pub fn put(&self, tx: oneshot::Sender<PermissionChoice>) {
        *self.0.lock().unwrap() = Some(tx);
    }

    pub fn take(&self) -> Option<oneshot::Sender<PermissionChoice>> {
        self.0.lock().unwrap().take()
    }
}
