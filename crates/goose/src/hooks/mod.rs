//! Hook System for Goose
//!
//! Provides Claude Code-style lifecycle hooks with:
//! - 13 lifecycle events (SessionStart, PreToolUse, PostToolUse, Stop, etc.)
//! - Exit code flow control (0=success, 2=blocking error)
//! - JSON output for decisions (approve, block, ask)
//! - Per-hook logging with correlation IDs
//! - Async hooks for background operations

mod events;
mod handlers;
mod logging;
mod manager;

pub use events::{CompactTrigger, HookEvent, SessionEndReason, SessionSource};
pub use handlers::{HookDecision, HookHandler, HookMatcher, HookResult};
pub use logging::{HookLogEntry, HookLogger};
pub use manager::{HookConfig, HookManager, HookStats};

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default hook timeout
pub const DEFAULT_HOOK_TIMEOUT: Duration = Duration::from_secs(60);

/// Exit codes for hook flow control
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const BLOCKING_ERROR: i32 = 2;
}

/// Hook-specific output for different event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hookEventName", rename_all = "PascalCase")]
pub enum HookSpecificOutput {
    SessionStart {
        #[serde(rename = "additionalContext")]
        additional_context: Option<String>,
    },
    UserPromptSubmit {
        #[serde(rename = "additionalContext")]
        additional_context: Option<String>,
    },
    PreToolUse {
        #[serde(rename = "permissionDecision")]
        permission_decision: Option<PermissionDecision>,
        #[serde(rename = "permissionDecisionReason")]
        permission_decision_reason: Option<String>,
        #[serde(rename = "updatedInput")]
        updated_input: Option<serde_json::Value>,
        #[serde(rename = "additionalContext")]
        additional_context: Option<String>,
    },
    PostToolUse {
        #[serde(rename = "additionalContext")]
        additional_context: Option<String>,
    },
    Stop {
        #[serde(rename = "additionalContext")]
        additional_context: Option<String>,
    },
}

/// Permission decision for PreToolUse hooks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

/// JSON output structure from hooks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookOutput {
    #[serde(default)]
    pub continue_execution: bool,

    #[serde(rename = "stopReason")]
    pub stop_reason: Option<String>,

    #[serde(rename = "suppressOutput")]
    pub suppress_output: Option<bool>,

    pub decision: Option<String>,
    pub reason: Option<String>,

    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

impl HookOutput {
    pub fn should_block(&self) -> bool {
        self.decision.as_deref() == Some("block")
    }

    pub fn should_approve(&self) -> bool {
        self.decision.as_deref() == Some("approve")
    }

    pub fn get_additional_context(&self) -> Option<&str> {
        match &self.hook_specific_output {
            Some(HookSpecificOutput::SessionStart {
                additional_context, ..
            }) => additional_context.as_deref(),
            Some(HookSpecificOutput::UserPromptSubmit {
                additional_context, ..
            }) => additional_context.as_deref(),
            Some(HookSpecificOutput::PreToolUse {
                additional_context, ..
            }) => additional_context.as_deref(),
            Some(HookSpecificOutput::PostToolUse {
                additional_context, ..
            }) => additional_context.as_deref(),
            Some(HookSpecificOutput::Stop {
                additional_context, ..
            }) => additional_context.as_deref(),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_output_should_block() {
        let output = HookOutput {
            decision: Some("block".to_string()),
            ..Default::default()
        };
        assert!(output.should_block());
        assert!(!output.should_approve());
    }

    #[test]
    fn test_hook_output_should_approve() {
        let output = HookOutput {
            decision: Some("approve".to_string()),
            ..Default::default()
        };
        assert!(!output.should_block());
        assert!(output.should_approve());
    }

    #[test]
    fn test_permission_decision_serialization() {
        let json = serde_json::to_string(&PermissionDecision::Allow).unwrap();
        assert_eq!(json, "\"allow\"");

        let decision: PermissionDecision = serde_json::from_str("\"deny\"").unwrap();
        assert_eq!(decision, PermissionDecision::Deny);
    }
}
