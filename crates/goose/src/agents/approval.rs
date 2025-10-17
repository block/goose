//! Approval handler trait for human-in-the-loop approval requests
//!
//! This module provides a unified interface for requesting user approval
//! for various operations like tool calls, MCP sampling, etc.

use async_trait::async_trait;

/// The action taken in response to an approval request
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalAction {
    /// Allow this specific request once
    AllowOnce,
    /// Always allow requests of this type
    AlwaysAllow,
    /// Deny this request
    Deny,
}

impl ApprovalAction {
    /// Convert to a boolean approval (AllowOnce and AlwaysAllow = true, Deny = false)
    pub fn is_approved(&self) -> bool {
        matches!(
            self,
            ApprovalAction::AllowOnce | ApprovalAction::AlwaysAllow
        )
    }
}

/// The type of approval being requested
#[derive(Debug, Clone)]
pub enum ApprovalType {
    /// Approval for a tool call
    ToolCall {
        tool_name: String,
        prompt: Option<String>,
        principal_type: String,
    },
    /// Approval for MCP sampling
    Sampling {
        extension_name: String,
        messages: Vec<rmcp::model::SamplingMessage>,
        system_prompt: Option<String>,
        max_tokens: u32,
    },
}

/// Handler for approval requests
///
/// Implementations of this trait manage the communication with the user
/// interface to request and receive approval decisions.
#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    /// Request approval for an operation
    ///
    /// # Arguments
    /// * `session_id` - The session ID for this request
    /// * `approval_type` - The type of approval being requested
    ///
    /// # Returns
    /// The approval action taken by the user
    async fn request_approval(
        &self,
        session_id: String,
        approval_type: ApprovalType,
    ) -> Result<ApprovalAction, String>;
}

/// A no-op approval handler that automatically approves all requests
///
/// This is useful for CLI mode or automated testing where no UI is available
pub struct AutoApprovalHandler;

#[async_trait]
impl ApprovalHandler for AutoApprovalHandler {
    async fn request_approval(
        &self,
        _session_id: String,
        _approval_type: ApprovalType,
    ) -> Result<ApprovalAction, String> {
        Ok(ApprovalAction::AllowOnce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_action_is_approved() {
        assert!(ApprovalAction::AllowOnce.is_approved());
        assert!(ApprovalAction::AlwaysAllow.is_approved());
        assert!(!ApprovalAction::Deny.is_approved());
    }

    #[tokio::test]
    async fn test_auto_approval_handler() {
        let handler = AutoApprovalHandler;

        let result = handler
            .request_approval(
                "test-session".to_string(),
                ApprovalType::ToolCall {
                    tool_name: "test_tool".to_string(),
                    prompt: None,
                    principal_type: "Tool".to_string(),
                },
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ApprovalAction::AllowOnce);

        let result = handler
            .request_approval(
                "test-session".to_string(),
                ApprovalType::Sampling {
                    extension_name: "test-extension".to_string(),
                    messages: vec![],
                    system_prompt: None,
                    max_tokens: 100,
                },
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ApprovalAction::AllowOnce);
    }
}
