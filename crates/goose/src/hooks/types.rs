use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum HookEventKind {
    #[default]
    SessionStart,
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    UserPromptSubmit,
    Stop,
    SubagentStop,
    SubagentStart,
    SessionEnd,
    PreCompact,
    PostCompact,
    Notification,
    PermissionRequest,
    TeammateIdle,
    TaskCompleted,
    ConfigChange,
}

impl HookEventKind {
    pub fn can_block(&self) -> bool {
        matches!(
            self,
            Self::PreToolUse
                | Self::PermissionRequest
                | Self::UserPromptSubmit
                | Self::Stop
                | Self::SubagentStop
                | Self::TeammateIdle
                | Self::TaskCompleted
                | Self::ConfigChange
        )
    }
}

impl std::str::FromStr for HookEventKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(s.to_string()))
            .map_err(|e| format!("unknown hook event '{}': {}", s, e))
    }
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct HookInvocation {
    #[serde(rename = "hook_event_name")]
    pub event: HookEventKind,
    pub session_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_error: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_prompt: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count_before: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count_after: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_compact: Option<bool>,
}

impl HookInvocation {
    fn base(event: HookEventKind, session_id: String) -> Self {
        Self {
            event,
            session_id,
            ..Default::default()
        }
    }

    pub fn pre_tool_use(
        session_id: String,
        tool_name: String,
        tool_input: Value,
        cwd: String,
    ) -> Self {
        Self {
            tool_name: Some(tool_name),
            tool_input: Some(tool_input),
            cwd: Some(cwd),
            ..Self::base(HookEventKind::PreToolUse, session_id)
        }
    }

    pub fn post_tool_use(
        session_id: String,
        tool_name: String,
        tool_input: Value,
        tool_output: Value,
        cwd: String,
    ) -> Self {
        Self {
            tool_name: Some(tool_name),
            tool_input: Some(tool_input),
            tool_output: Some(tool_output),
            cwd: Some(cwd),
            ..Self::base(HookEventKind::PostToolUse, session_id)
        }
    }

    pub fn post_tool_use_failure(
        session_id: String,
        tool_name: String,
        tool_input: Value,
        tool_error: String,
        cwd: String,
    ) -> Self {
        Self {
            tool_name: Some(tool_name),
            tool_input: Some(tool_input),
            tool_error: Some(tool_error),
            cwd: Some(cwd),
            ..Self::base(HookEventKind::PostToolUseFailure, session_id)
        }
    }

    pub fn user_prompt_submit(session_id: String, user_prompt: String, cwd: String) -> Self {
        Self {
            user_prompt: Some(user_prompt),
            cwd: Some(cwd),
            ..Self::base(HookEventKind::UserPromptSubmit, session_id)
        }
    }

    pub fn session_start(session_id: String, cwd: String) -> Self {
        Self {
            cwd: Some(cwd),
            ..Self::base(HookEventKind::SessionStart, session_id)
        }
    }

    pub fn session_end(session_id: String, reason: Option<String>) -> Self {
        Self {
            reason,
            ..Self::base(HookEventKind::SessionEnd, session_id)
        }
    }

    pub fn stop(session_id: String, reason: Option<String>, cwd: String) -> Self {
        Self {
            reason,
            cwd: Some(cwd),
            ..Self::base(HookEventKind::Stop, session_id)
        }
    }

    pub fn subagent_start(session_id: String, cwd: String) -> Self {
        Self {
            cwd: Some(cwd),
            ..Self::base(HookEventKind::SubagentStart, session_id)
        }
    }

    pub fn subagent_stop(session_id: String, reason: Option<String>) -> Self {
        Self {
            reason,
            ..Self::base(HookEventKind::SubagentStop, session_id)
        }
    }

    pub fn pre_compact(
        session_id: String,
        message_count_before: usize,
        manual: bool,
        cwd: String,
    ) -> Self {
        Self {
            message_count_before: Some(message_count_before),
            manual_compact: Some(manual),
            cwd: Some(cwd),
            ..Self::base(HookEventKind::PreCompact, session_id)
        }
    }

    pub fn post_compact(
        session_id: String,
        message_count_before: usize,
        message_count_after: usize,
        manual: bool,
        cwd: String,
    ) -> Self {
        Self {
            message_count_before: Some(message_count_before),
            message_count_after: Some(message_count_after),
            manual_compact: Some(manual),
            cwd: Some(cwd),
            ..Self::base(HookEventKind::PostCompact, session_id)
        }
    }

    pub fn permission_request(
        session_id: String,
        tool_name: String,
        tool_input: Value,
        cwd: String,
    ) -> Self {
        Self {
            tool_name: Some(tool_name),
            tool_input: Some(tool_input),
            cwd: Some(cwd),
            ..Self::base(HookEventKind::PermissionRequest, session_id)
        }
    }

    pub fn notification(session_id: String, notification_type: String, cwd: String) -> Self {
        Self {
            notification_type: Some(notification_type),
            cwd: Some(cwd),
            ..Self::base(HookEventKind::Notification, session_id)
        }
    }

    pub fn teammate_idle(session_id: String, cwd: String) -> Self {
        Self {
            cwd: Some(cwd),
            ..Self::base(HookEventKind::TeammateIdle, session_id)
        }
    }

    pub fn task_completed(session_id: String, cwd: String) -> Self {
        Self {
            cwd: Some(cwd),
            ..Self::base(HookEventKind::TaskCompleted, session_id)
        }
    }

    pub fn config_change(session_id: String, cwd: String) -> Self {
        Self {
            cwd: Some(cwd),
            ..Self::base(HookEventKind::ConfigChange, session_id)
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookResult {
    #[serde(default)]
    pub decision: Option<HookDecision>,

    #[serde(default)]
    pub reason: Option<String>,

    #[serde(default)]
    pub hook_specific_output: Option<Value>,

    #[serde(default, rename = "continue")]
    pub continue_: Option<bool>,

    #[serde(default)]
    pub stop_reason: Option<String>,

    #[serde(default)]
    pub additional_context: Option<String>,

    #[serde(default)]
    pub system_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookDecision {
    Allow,
    Block,
}

#[derive(Debug, Default)]
pub struct HooksOutcome {
    pub blocked: bool,
    pub context: Option<String>,
}
