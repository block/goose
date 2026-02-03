//! Hook Events - All 13 lifecycle events from Claude Code

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Source of session start
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionSource {
    Startup,
    Resume,
    Clear,
    Compact,
}

/// Trigger for compaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactTrigger {
    Manual,
    Auto,
}

/// Reason for session end
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionEndReason {
    Exit,
    Sigint,
    Error,
    Timeout,
}

/// All 13 hook events supported by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name", rename_all = "PascalCase")]
pub enum HookEvent {
    /// Fires when entering a repository (init) or periodically (maintenance)
    Setup {
        trigger: String, // "init" or "maintenance"
        session_id: String,
        cwd: String,
    },

    /// Fires when a new session starts or resumes
    SessionStart {
        source: SessionSource,
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        model: Option<String>,
        agent_type: Option<String>,
    },

    /// Fires immediately when user submits a prompt
    UserPromptSubmit {
        prompt: String,
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
    },

    /// Fires before any tool execution
    PreToolUse {
        tool_name: String,
        tool_input: Value,
        tool_use_id: String,
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
    },

    /// Fires when user is shown a permission dialog
    PermissionRequest {
        tool_name: String,
        tool_input: Value,
        tool_use_id: String,
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
    },

    /// Fires after successful tool completion
    PostToolUse {
        tool_name: String,
        tool_input: Value,
        tool_response: Value,
        tool_use_id: String,
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
    },

    /// Fires when a tool execution fails
    PostToolUseFailure {
        tool_name: String,
        tool_input: Value,
        tool_use_id: String,
        error: String,
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
    },

    /// Fires when notifications are sent
    Notification {
        message: String,
        session_id: String,
        cwd: String,
    },

    /// Fires when a subagent spawns
    SubagentStart {
        agent_id: String,
        agent_type: String,
        session_id: String,
        cwd: String,
    },

    /// Fires when a subagent finishes
    SubagentStop {
        agent_id: String,
        stop_hook_active: bool,
        session_id: String,
        cwd: String,
    },

    /// Fires when the agent finishes responding
    Stop {
        stop_hook_active: bool,
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
    },

    /// Fires before compaction operations
    PreCompact {
        trigger: CompactTrigger,
        custom_instructions: Option<String>,
        session_id: String,
        transcript_path: String,
        cwd: String,
    },

    /// Fires when session ends
    SessionEnd {
        reason: SessionEndReason,
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
    },
}

impl HookEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            HookEvent::Setup { .. } => "Setup",
            HookEvent::SessionStart { .. } => "SessionStart",
            HookEvent::UserPromptSubmit { .. } => "UserPromptSubmit",
            HookEvent::PreToolUse { .. } => "PreToolUse",
            HookEvent::PermissionRequest { .. } => "PermissionRequest",
            HookEvent::PostToolUse { .. } => "PostToolUse",
            HookEvent::PostToolUseFailure { .. } => "PostToolUseFailure",
            HookEvent::Notification { .. } => "Notification",
            HookEvent::SubagentStart { .. } => "SubagentStart",
            HookEvent::SubagentStop { .. } => "SubagentStop",
            HookEvent::Stop { .. } => "Stop",
            HookEvent::PreCompact { .. } => "PreCompact",
            HookEvent::SessionEnd { .. } => "SessionEnd",
        }
    }

    pub fn session_id(&self) -> &str {
        match self {
            HookEvent::Setup { session_id, .. } => session_id,
            HookEvent::SessionStart { session_id, .. } => session_id,
            HookEvent::UserPromptSubmit { session_id, .. } => session_id,
            HookEvent::PreToolUse { session_id, .. } => session_id,
            HookEvent::PermissionRequest { session_id, .. } => session_id,
            HookEvent::PostToolUse { session_id, .. } => session_id,
            HookEvent::PostToolUseFailure { session_id, .. } => session_id,
            HookEvent::Notification { session_id, .. } => session_id,
            HookEvent::SubagentStart { session_id, .. } => session_id,
            HookEvent::SubagentStop { session_id, .. } => session_id,
            HookEvent::Stop { session_id, .. } => session_id,
            HookEvent::PreCompact { session_id, .. } => session_id,
            HookEvent::SessionEnd { session_id, .. } => session_id,
        }
    }

    pub fn tool_name(&self) -> Option<&str> {
        match self {
            HookEvent::PreToolUse { tool_name, .. } => Some(tool_name),
            HookEvent::PermissionRequest { tool_name, .. } => Some(tool_name),
            HookEvent::PostToolUse { tool_name, .. } => Some(tool_name),
            HookEvent::PostToolUseFailure { tool_name, .. } => Some(tool_name),
            _ => None,
        }
    }

    pub fn can_block(&self) -> bool {
        matches!(
            self,
            HookEvent::UserPromptSubmit { .. }
                | HookEvent::PreToolUse { .. }
                | HookEvent::Stop { .. }
                | HookEvent::SubagentStop { .. }
        )
    }

    pub fn to_json_input(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Common input fields present in all hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonHookInput {
    pub session_id: String,
    pub transcript_path: Option<String>,
    pub cwd: String,
    pub permission_mode: Option<String>,
    pub hook_event_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_names() {
        let event = HookEvent::SessionStart {
            source: SessionSource::Startup,
            session_id: "test".to_string(),
            transcript_path: "/path".to_string(),
            cwd: "/cwd".to_string(),
            permission_mode: "default".to_string(),
            model: None,
            agent_type: None,
        };
        assert_eq!(event.event_name(), "SessionStart");
    }

    #[test]
    fn test_hook_event_can_block() {
        let blocking_event = HookEvent::PreToolUse {
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({}),
            tool_use_id: "id".to_string(),
            session_id: "test".to_string(),
            transcript_path: "/path".to_string(),
            cwd: "/cwd".to_string(),
            permission_mode: "default".to_string(),
        };
        assert!(blocking_event.can_block());

        let non_blocking_event = HookEvent::Notification {
            message: "test".to_string(),
            session_id: "test".to_string(),
            cwd: "/cwd".to_string(),
        };
        assert!(!non_blocking_event.can_block());
    }

    #[test]
    fn test_hook_event_serialization() {
        let event = HookEvent::UserPromptSubmit {
            prompt: "test prompt".to_string(),
            session_id: "session-1".to_string(),
            transcript_path: "/path".to_string(),
            cwd: "/cwd".to_string(),
            permission_mode: "default".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("UserPromptSubmit"));
        assert!(json.contains("test prompt"));
    }

    #[test]
    fn test_session_source_serialization() {
        let source = SessionSource::Startup;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"startup\"");
    }
}
