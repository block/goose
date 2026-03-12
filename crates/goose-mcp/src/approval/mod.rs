use rmcp::{
    elicit_safe,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorCode, ErrorData, Implementation, InitializeResult,
        ServerCapabilities, ServerInfo,
    },
    schemars::JsonSchema,
    service::{ElicitationError, RequestContext},
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalRequest {
    /// Human-readable summary of the action being reviewed
    pub action_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalDecision {
    /// Whether the action should proceed
    pub approved: bool,
    /// Optional explanation or rationale
    pub reason: Option<String>,
}

elicit_safe!(ApprovalDecision);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ApprovalStatus {
    Approved,
    Rejected,
    Declined,
    Cancelled,
    NoResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ApprovalOutcome {
    action_summary: String,
    status: ApprovalStatus,
    approved: bool,
    reason: Option<String>,
}

impl ApprovalOutcome {
    fn from_decision(action_summary: String, decision: ApprovalDecision) -> Self {
        let reason = decision
            .reason
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Self {
            action_summary,
            status: if decision.approved {
                ApprovalStatus::Approved
            } else {
                ApprovalStatus::Rejected
            },
            approved: decision.approved,
            reason,
        }
    }

    fn declined(action_summary: String) -> Self {
        Self {
            action_summary,
            status: ApprovalStatus::Declined,
            approved: false,
            reason: None,
        }
    }

    fn cancelled(action_summary: String) -> Self {
        Self {
            action_summary,
            status: ApprovalStatus::Cancelled,
            approved: false,
            reason: None,
        }
    }

    fn no_response(action_summary: String) -> Self {
        Self {
            action_summary,
            status: ApprovalStatus::NoResponse,
            approved: false,
            reason: None,
        }
    }

    fn as_text(&self) -> String {
        match self.status {
            ApprovalStatus::Approved | ApprovalStatus::Rejected => {
                let status = if self.approved {
                    "APPROVED"
                } else {
                    "REJECTED"
                };
                match &self.reason {
                    Some(reason) => format!("{status}: {reason}"),
                    None => status.to_string(),
                }
            }
            ApprovalStatus::Declined => "The user declined the approval request.".to_string(),
            ApprovalStatus::Cancelled => "The user cancelled the approval request.".to_string(),
            ApprovalStatus::NoResponse => "No approval data was provided.".to_string(),
        }
    }
}

/// Approval MCP Server using official RMCP SDK.
#[derive(Clone)]
pub struct ApprovalServer {
    tool_router: ToolRouter<Self>,
}

impl Default for ApprovalServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl ApprovalServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    fn approval_result(outcome: ApprovalOutcome) -> CallToolResult {
        let mut result = CallToolResult::success(vec![Content::text(outcome.as_text())]);
        result.structured_content = serde_json::to_value(outcome).ok();
        result
    }

    /// Ask the user for approval before continuing a workflow.
    #[tool(
        name = "request_approval",
        description = "Ask the user for structured approval before continuing a workflow"
    )]
    pub async fn request_approval(
        &self,
        params: Parameters<ApprovalRequest>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let action_summary = params.0.action_summary;
        let prompt = format!(
            "Review the following action and decide whether it should proceed:\n\n{}",
            action_summary
        );

        match context
            .peer
            .elicit_with_timeout::<ApprovalDecision>(prompt, None)
            .await
        {
            Ok(Some(decision)) => Ok(Self::approval_result(ApprovalOutcome::from_decision(
                action_summary,
                decision,
            ))),
            Ok(None) => Ok(Self::approval_result(ApprovalOutcome::no_response(
                action_summary,
            ))),
            Err(ElicitationError::UserDeclined) => Ok(Self::approval_result(
                ApprovalOutcome::declined(action_summary),
            )),
            Err(ElicitationError::UserCancelled) => Ok(Self::approval_result(
                ApprovalOutcome::cancelled(action_summary),
            )),
            Err(error) => Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to collect approval: {error}"),
                None,
            )),
        }
    }
}

#[tool_handler]
impl ServerHandler for ApprovalServer {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "goose-approval",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "The Approval extension provides a reusable approval-gated workflow primitive using MCP elicitation. Use it when you need explicit user confirmation before continuing."
                    .to_string(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_approval_server_creation() {
        let server = ApprovalServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "goose-approval");
        assert!(info
            .instructions
            .unwrap_or_default()
            .contains("approval-gated workflow primitive"));
    }

    #[test]
    fn test_approval_tool_schema() {
        let attr = ApprovalServer::request_approval_tool_attr();
        assert_eq!(attr.name, "request_approval");
        assert_eq!(
            attr.input_schema
                .get("properties")
                .and_then(|v| v.get("action_summary"))
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("string")
        );
    }

    #[test]
    fn test_approval_result_includes_structured_content() {
        let result = ApprovalServer::approval_result(ApprovalOutcome::from_decision(
            "Run database migration".to_string(),
            ApprovalDecision {
                approved: true,
                reason: Some("Reviewed and safe to proceed".to_string()),
            },
        ));

        assert_eq!(result.is_error, Some(false));
        assert_eq!(
            result.structured_content,
            Some(serde_json::json!({
                "action_summary": "Run database migration",
                "status": "approved",
                "approved": true,
                "reason": "Reviewed and safe to proceed"
            }))
        );
    }
}
