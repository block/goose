//! A unified interface for requesting user approval
//! for various operations like tool calls, MCP sampling, etc

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ApprovalType {
    #[serde(rename_all = "camelCase")]
    ToolCall {
        tool_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        principal_type: String,
    },
    #[serde(rename_all = "camelCase")]
    Sampling {
        extension_name: String,
        #[schema(value_type = Vec<Object>)]
        messages: Vec<rmcp::model::SamplingMessage>,
        #[serde(skip_serializing_if = "Option::is_none")]
        system_prompt: Option<String>,
        max_tokens: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAction {
    AllowOnce,
    AlwaysAllow,
    Deny,
}

impl ApprovalAction {
    pub fn is_approved(&self) -> bool {
        matches!(
            self,
            ApprovalAction::AllowOnce | ApprovalAction::AlwaysAllow
        )
    }
}

#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    async fn request_approval(
        &self,
        session_id: String,
        approval_type: ApprovalType,
    ) -> Result<ApprovalAction, String>;
}