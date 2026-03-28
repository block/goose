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
    /// Raw accumulated markdown from the agent (used for incremental re-render).
    pub agent_raw: String,
    /// Rendered (markdown → plain text) agent reply; updated on every chunk.
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

/// Return an icon glyph for a tool based on its name.
///
/// Tool names arrive as `{extension}__{tool}` (e.g. `developer__shell`).
/// We match on keywords in the full name to infer a kind icon, mirroring
/// the KIND_ICONS table used by the Node.js text TUI.
pub fn tool_kind_icon(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if n.contains("think")                                           { return "💭"; }
    if n.contains("fetch") || n.contains("http") || n.contains("web") || n.contains("url") || n.contains("browse") { return "🌐"; }
    if n.contains("shell") || n.contains("bash") || n.contains("exec") || n.contains("run") || n.contains("command") { return "▶"; }
    if n.contains("search") || n.contains("grep") || n.contains("find") || n.contains("glob") { return "🔍"; }
    if n.contains("delete") || n.contains("remove") || n.contains("unlink") { return "🗑"; }
    if n.contains("move")   || n.contains("rename") || n.contains("copy")  { return "📦"; }
    if n.contains("edit")   || n.contains("write")  || n.contains("create") || n.contains("patch") || n.contains("str_replace") || n.contains("append") { return "✏️"; }
    if n.contains("read")   || n.contains("view")   || n.contains("cat")   || n.contains("list")  || n.contains("get") { return "📖"; }
    if n.contains("switch") || n.contains("mode")                           { return "🔀"; }
    "⚙"
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
    /// Agent needs free-text input from the user (elicitation).
    ElicitationRequest(ElicitationReq, oneshot::Sender<String>),
    Finished { stop_reason: String },
    /// Token usage after a completed turn.
    TokenUsage { input: i64, output: i64, total: i64 },
    Error(String),
}

#[derive(Clone, Debug)]
pub struct ElicitationReq {
    pub id: String,
    pub message: String,
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

/// Same pattern for elicitation (free-text) reply.
#[derive(Clone, Default)]
pub struct PendingElicitReply(pub Arc<Mutex<Option<oneshot::Sender<String>>>>);

impl PendingElicitReply {
    pub fn put(&self, tx: oneshot::Sender<String>) {
        *self.0.lock().unwrap() = Some(tx);
    }

    pub fn take(&self) -> Option<oneshot::Sender<String>> {
        self.0.lock().unwrap().take()
    }
}
