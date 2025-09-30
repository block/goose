use std::future::Future;
use std::sync::Arc;

use async_stream::try_stream;
use futures::stream::{self, BoxStream};
use futures::{Stream, StreamExt};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::config::permission::PermissionLevel;
use crate::mcp_utils::ToolResult;
use rmcp::model::{Content, ServerNotification};

// ToolCallResult combines the result of a tool call with an optional notification stream that
// can be used to receive notifications from the tool.
pub struct ToolCallResult {
    pub result: Box<dyn Future<Output = ToolResult<Vec<Content>>> + Send + Unpin>,
    pub notification_stream: Option<Box<dyn Stream<Item = ServerNotification> + Send + Unpin>>,
}

impl From<ToolResult<Vec<Content>>> for ToolCallResult {
    fn from(result: ToolResult<Vec<Content>>) -> Self {
        Self {
            result: Box::new(futures::future::ready(result)),
            notification_stream: None,
        }
    }
}

use super::agent::{tool_stream, ToolStream};
use crate::agents::{Agent, SessionConfig};
use crate::conversation::message::{Message, ToolRequest};
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
    pub(crate) fn handle_approval_tool_requests<'a>(
        &'a self,
        tool_requests: &'a [ToolRequest],
        tool_futures: Arc<Mutex<Vec<(String, ToolStream)>>>,
        message_tool_response: Arc<Mutex<Message>>,
        cancellation_token: Option<CancellationToken>,
        session: Option<SessionConfig>,
        inspection_results: &'a [crate::tool_inspection::InspectionResult],
    ) -> BoxStream<'a, anyhow::Result<Message>> {
        try_stream! {
            for request in tool_requests.iter() {
                if let Ok(tool_call) = request.tool_call.clone() {
                    // Find the corresponding inspection result for this tool request
                    let security_message = inspection_results.iter()
                        .find(|result| result.tool_request_id == request.id)
                        .and_then(|result| {
                            if let crate::tool_inspection::InspectionAction::RequireApproval(Some(message)) = &result.action {
                                Some(message.clone())
                            } else {
                                None
                            }
                        });

                    let confirmation = Message::user().with_tool_confirmation_request(
                        request.id.clone(),
                        tool_call.name.to_string().clone(),
                        tool_call.arguments.clone().unwrap_or_default(),
                        security_message.clone(),
                    );
                    yield confirmation;

                    // Use the unified approval handler system
                    let approval_handler = self.extension_manager.get_approval_handler().await;
                    let approval_action = if let Some(handler) = approval_handler {
                        let session_id = session.as_ref()
                            .map(|s| s.id.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        match handler.request_approval(
                            session_id,
                            crate::agents::approval::ApprovalType::ToolCall {
                                tool_name: tool_call.name.to_string(),
                                prompt: security_message,
                                principal_type: "Tool".to_string(),
                            }
                        ).await {
                            Ok(action) => Some(action),
                            Err(e) => {
                                tracing::error!("Approval handler error: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    };

                    if let Some(action) = approval_action {
                        // Log user decision if this was a security alert
                        if let Some(finding_id) = get_security_finding_id_from_results(&request.id, inspection_results) {
                            tracing::info!(
                                "🔒 User security decision: {:?} for finding ID: {}",
                                action,
                                finding_id
                            );
                        }

                        if action.is_approved() {
                            let (req_id, tool_result) = self.dispatch_tool_call(
                                tool_call.clone(),
                                request.id.clone(),
                                cancellation_token.clone(),
                                session.clone()
                            ).await;
                            let mut futures = tool_futures.lock().await;

                            futures.push((req_id, match tool_result {
                                Ok(result) => tool_stream(
                                    result.notification_stream.unwrap_or_else(|| Box::new(stream::empty())),
                                    result.result,
                                ),
                                Err(e) => tool_stream(
                                    Box::new(stream::empty()),
                                    futures::future::ready(Err(e)),
                                ),
                            }));

                            // Update the shared permission manager when user selects "Always Allow"
                            if matches!(action, crate::agents::approval::ApprovalAction::AlwaysAllow) {
                                self.tool_inspection_manager
                                    .update_permission_manager(&tool_call.name, PermissionLevel::AlwaysAllow)
                                    .await;
                            }
                        } else {
                            // User declined - add declined response
                            let mut response = message_tool_response.lock().await;
                            *response = response.clone().with_tool_response(
                                request.id.clone(),
                                Ok(vec![Content::text(DECLINED_RESPONSE)]),
                            );
                        }
                    }
                }
            }
        }.boxed()
    }

    pub(crate) fn handle_frontend_tool_requests<'a>(
        &'a self,
        tool_requests: &'a [ToolRequest],
        message_tool_response: Arc<Mutex<Message>>,
    ) -> BoxStream<'a, anyhow::Result<Message>> {
        try_stream! {
            for request in tool_requests {
                if let Ok(tool_call) = request.tool_call.clone() {
                    if self.is_frontend_tool(&tool_call.name).await {
                        // Send frontend tool request and wait for response
                        yield Message::assistant().with_frontend_tool_request(
                            request.id.clone(),
                            Ok(tool_call.clone())
                        );

                        if let Some((id, result)) = self.tool_result_rx.lock().await.recv().await {
                            let mut response = message_tool_response.lock().await;
                            *response = response.clone().with_tool_response(id, result);
                        }
                    }
                }
            }
        }
        .boxed()
    }
}
