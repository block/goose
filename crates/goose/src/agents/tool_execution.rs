use async_stream::try_stream;
use futures::stream::{self, BoxStream};
use futures::{Stream, StreamExt};
use rmcp::model::CallToolResult;
use std::collections::HashMap;
use std::future::Future;
use tokio_util::sync::CancellationToken;

use std::path::PathBuf;

use crate::config::permission::PermissionLevel;
use crate::mcp_utils::ToolResult;
use crate::permission::Permission;
use rmcp::model::{Content, ServerNotification};

/// Context passed through the tool call dispatch chain.
pub struct ToolCallContext {
    pub session_id: String,
    pub working_dir: Option<PathBuf>,
    pub tool_call_request_id: Option<String>,
}

impl ToolCallContext {
    pub fn new(
        session_id: String,
        working_dir: Option<PathBuf>,
        tool_call_request_id: Option<String>,
    ) -> Self {
        Self {
            session_id,
            working_dir,
            tool_call_request_id,
        }
    }

    pub fn working_dir_str(&self) -> Option<&str> {
        self.working_dir.as_ref().and_then(|p| p.to_str())
    }
}

// ToolCallResult combines the result of a tool call with an optional notification stream that
// can be used to receive notifications from the tool.
pub struct ToolCallResult {
    pub result: Box<dyn Future<Output = ToolResult<rmcp::model::CallToolResult>> + Send + Unpin>,
    pub notification_stream: Option<Box<dyn Stream<Item = ServerNotification> + Send + Unpin>>,
}

impl From<ToolResult<rmcp::model::CallToolResult>> for ToolCallResult {
    fn from(result: ToolResult<rmcp::model::CallToolResult>) -> Self {
        Self {
            result: Box::new(futures::future::ready(result)),
            notification_stream: None,
        }
    }
}

use super::agent::{tool_stream, ToolStream};
use crate::agents::Agent;
use crate::config::GooseMode;
use crate::conversation::message::{Message, ToolRequest};
use crate::permission::permission_judge::PermissionCheckResult;
use crate::session::Session;
use crate::tool_inspection::get_security_finding_id_from_results;

pub const DECLINED_RESPONSE: &str = "The user has declined to run this tool. \
    DO NOT attempt to call this tool again. \
    If there are no alternative methods to proceed, clearly explain the situation and STOP.";

pub const CHAT_MODE_TOOL_SKIPPED_RESPONSE: &str = "Let the user know the tool call was skipped in goose chat mode. \
                                        DO NOT apologize for skipping the tool call. DO NOT say sorry. \
                                        Provide an explanation of what the tool call would do, structured as a \
                                        plan for the user. Again, DO NOT apologize. \
                                        **Example Plan:**\n \
                                        1. **Identify Task Scope** - Determine the purpose and expected outcome.\n \
                                        2. **Outline Steps** - Break down the steps.\n \
                                        If needed, adjust the explanation based on user preferences or questions.";

impl Agent {
    /// Consolidated tool gate flow: inspect → permission-check → dispatch/deny/approve.
    ///
    /// Approved tools are dispatched immediately and added to `tool_futures`.
    /// Denied tools get a DECLINED_RESPONSE in `request_to_response_map`.
    /// Tools needing approval produce action-required messages in the returned stream;
    /// once confirmed, they're dispatched and added to `tool_futures`.
    pub(crate) fn gate_and_dispatch_tools<'a>(
        &'a self,
        session_id: &'a str,
        requests: &'a [ToolRequest],
        messages: &'a [Message],
        goose_mode: GooseMode,
        tool_futures: &'a mut Vec<(String, ToolStream)>,
        request_to_response_map: &'a mut HashMap<String, Message>,
        cancel_token: Option<CancellationToken>,
        session: &'a Session,
    ) -> BoxStream<'a, anyhow::Result<Message>> {
        try_stream! {
            let inspection_results = self
                .tool_inspection_manager
                .inspect_tools(session_id, requests, messages, goose_mode)
                .await?;

            let permission_check_result = self
                .tool_inspection_manager
                .process_inspection_results_with_permission_inspector(requests, &inspection_results)
                .unwrap_or_else(|| {
                    let mut result = PermissionCheckResult {
                        approved: vec![],
                        needs_approval: vec![],
                        denied: vec![],
                    };
                    result.needs_approval.extend(requests.iter().cloned());
                    result
                });

            // Dispatch approved tools
            for request in &permission_check_result.approved {
                if let Ok(tool_call) = request.tool_call.clone() {
                    let (req_id, tool_result) = self
                        .dispatch_tool_call(
                            tool_call,
                            request.id.clone(),
                            cancel_token.clone(),
                            session,
                        )
                        .await;
                    tool_futures.push((
                        req_id,
                        match tool_result {
                            Ok(result) => tool_stream(
                                result
                                    .notification_stream
                                    .unwrap_or_else(|| Box::new(stream::empty())),
                                result.result,
                            ),
                            Err(e) => {
                                tool_stream(Box::new(stream::empty()), futures::future::ready(Err(e)))
                            }
                        },
                    ));
                }
            }

            // Mark denied tools
            for request in &permission_check_result.denied {
                if let Some(response) = request_to_response_map.get_mut(&request.id) {
                    response.add_tool_response_with_metadata(
                        request.id.clone(),
                        Ok(CallToolResult::error(vec![Content::text(DECLINED_RESPONSE)])),
                        request.metadata.as_ref(),
                    );
                }
            }

            // Handle tools needing approval (yields action-required messages)
            for request in permission_check_result.needs_approval.iter() {
                if let Ok(tool_call) = request.tool_call.clone() {
                    let security_message = inspection_results.iter()
                        .find(|result| result.tool_request_id == request.id)
                        .and_then(|result| {
                            if let crate::tool_inspection::InspectionAction::RequireApproval(Some(message)) = &result.action {
                                Some(message.clone())
                            } else {
                                None
                            }
                        });

                    let confirmation_rx = self.tool_confirmation_router.register(request.id.clone()).await;

                    let action_required_msg = Message::assistant()
                        .with_action_required(
                            request.id.clone(),
                            tool_call.name.to_string().clone(),
                            tool_call.arguments.clone().unwrap_or_default(),
                            security_message,
                        )
                        .user_only();
                    yield action_required_msg;

                    let confirmation = confirmation_rx.await
                        .map_err(|_| anyhow::anyhow!("Confirmation channel closed for request {}", request.id))?;

                    if let Some(finding_id) = get_security_finding_id_from_results(&request.id, &inspection_results) {
                        tracing::info!(
                            monotonic_counter.goose.prompt_injection_user_decisions = 1,
                            decision = ?confirmation.permission,
                            finding_id = %finding_id,
                            tool_request_id = %request.id,
                            "Prompt injection detection: user decision on command injection finding"
                        );
                    }

                    if confirmation.permission == Permission::AllowOnce || confirmation.permission == Permission::AlwaysAllow {
                        let (req_id, tool_result) = self.dispatch_tool_call(tool_call.clone(), request.id.clone(), cancel_token.clone(), session).await;

                        tool_futures.push((req_id, match tool_result {
                            Ok(result) => tool_stream(
                                result.notification_stream.unwrap_or_else(|| Box::new(stream::empty())),
                                result.result,
                            ),
                            Err(e) => tool_stream(
                                Box::new(stream::empty()),
                                futures::future::ready(Err(e)),
                            ),
                        }));

                        if confirmation.permission == Permission::AlwaysAllow {
                            self.tool_inspection_manager
                                .update_permission_manager(&tool_call.name, PermissionLevel::AlwaysAllow)
                                .await;
                        }
                    } else {
                        if let Some(response) = request_to_response_map.get_mut(&request.id) {
                            response.add_tool_response_with_metadata(
                                request.id.clone(),
                                Ok(CallToolResult::error(vec![Content::text(DECLINED_RESPONSE)])),
                                request.metadata.as_ref(),
                            );
                        }

                        if confirmation.permission == Permission::AlwaysDeny {
                            self.tool_inspection_manager
                                .update_permission_manager(&tool_call.name, PermissionLevel::NeverAllow)
                                .await;
                        }
                    }
                }
            }
        }.boxed()
    }
}
