use anyhow::Result;
use goose_mcp::mcp_server_runner::serve;
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
struct ApprovalRequest {
    action_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ApprovalDecision {
    approved: bool,
    reason: Option<String>,
}

elicit_safe!(ApprovalDecision);

#[derive(Clone)]
struct ApprovalWorkflowServer {
    tool_router: ToolRouter<Self>,
}

impl ApprovalWorkflowServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl ApprovalWorkflowServer {
    #[tool(
        name = "request_approval",
        description = "Ask the user for approval before continuing a workflow"
    )]
    async fn request_approval(
        &self,
        params: Parameters<ApprovalRequest>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let prompt = format!(
            "Review the following action and decide whether it should proceed:\n\n{}",
            params.0.action_summary
        );

        match context
            .peer
            .elicit_with_timeout::<ApprovalDecision>(prompt, None)
            .await
        {
            Ok(Some(decision)) => {
                let status = if decision.approved {
                    "APPROVED"
                } else {
                    "REJECTED"
                };
                let reason = decision
                    .reason
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "No reason provided".to_string());

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "{status}: {reason}"
                ))]))
            }
            Ok(None) => Ok(CallToolResult::success(vec![Content::text(
                "No approval data was provided.".to_string(),
            )])),
            Err(ElicitationError::UserDeclined) => {
                Ok(CallToolResult::success(vec![Content::text(
                    "The user declined the approval request.".to_string(),
                )]))
            }
            Err(ElicitationError::UserCancelled) => {
                Ok(CallToolResult::success(vec![Content::text(
                    "The user cancelled the approval request.".to_string(),
                )]))
            }
            Err(error) => Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to collect approval: {error}"),
                None,
            )),
        }
    }
}

#[tool_handler]
impl ServerHandler for ApprovalWorkflowServer {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("approval-workflow-example", "1.0.0"))
            .with_instructions(
                "A tiny MCP server showing how to use elicitation for approval-gated workflows."
                    .to_string(),
            )
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    serve(ApprovalWorkflowServer::new()).await
}
