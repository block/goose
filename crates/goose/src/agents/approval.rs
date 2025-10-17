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

/// Handler for approval requests
///
/// Implementations of this trait manage the communication with the user
/// interface to request and receive approval decisions.
#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    /// Request approval for a tool call
    ///
    /// # Arguments
    /// * `session_id` - The session ID for this request
    /// * `tool_name` - The name of the tool being called
    /// * `prompt` - Optional security prompt to display
    /// * `principal_type` - The type of principal (e.g., "Tool")
    ///
    /// # Returns
    /// The approval action taken by the user
    async fn request_tool_approval(
        &self,
        session_id: String,
        tool_name: String,
        prompt: Option<String>,
        principal_type: String,
    ) -> Result<ApprovalAction, String>;

    /// Request approval for MCP sampling
    ///
    /// # Arguments
    /// * `session_id` - The session ID for this request
    /// * `extension_name` - The name of the extension requesting sampling
    /// * `messages` - The messages to be sent for sampling
    /// * `system_prompt` - Optional system prompt
    /// * `max_tokens` - Maximum tokens for the sampling request
    ///
    /// # Returns
    /// The approval action taken by the user
    async fn request_sampling_approval(
        &self,
        session_id: String,
        extension_name: String,
        messages: Vec<rmcp::model::SamplingMessage>,
        system_prompt: Option<String>,
        max_tokens: u32,
    ) -> Result<ApprovalAction, String>;
}

/// A no-op approval handler that automatically approves all requests
///
/// This is useful for CLI mode or automated testing where no UI is available
pub struct AutoApprovalHandler;

#[async_trait]
impl ApprovalHandler for AutoApprovalHandler {
    async fn request_tool_approval(
        &self,
        _session_id: String,
        _tool_name: String,
        _prompt: Option<String>,
        _principal_type: String,
    ) -> Result<ApprovalAction, String> {
        Ok(ApprovalAction::AllowOnce)
    }

    async fn request_sampling_approval(
        &self,
        _session_id: String,
        _extension_name: String,
        _messages: Vec<rmcp::model::SamplingMessage>,
        _system_prompt: Option<String>,
        _max_tokens: u32,
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
            .request_tool_approval(
                "test-session".to_string(),
                "test_tool".to_string(),
                None,
                "Tool".to_string(),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ApprovalAction::AllowOnce);

        let result = handler
            .request_sampling_approval(
                "test-session".to_string(),
                "test-extension".to_string(),
                vec![],
                None,
                100,
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ApprovalAction::AllowOnce);
    }
}
