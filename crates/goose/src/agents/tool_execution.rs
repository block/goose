use std::future::Future;
use std::sync::Arc;

use async_stream::try_stream;
use futures::stream::{self, BoxStream};
use futures::{Stream, StreamExt};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::config::permission::PermissionLevel;
use crate::config::PermissionManager;
use crate::permission::Permission;
use mcp_core::ToolResult;
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
use crate::agents::Agent;
use crate::conversation::message::{Message, ToolRequest};

pub const DECLINED_RESPONSE: &str = "The user has declined to run this tool. \
    DO NOT attempt to call this tool again. \
    If there are no alternative methods to proceed, clearly explain the situation and STOP.";

pub const CHAT_MODE_TOOL_SKIPPED_RESPONSE: &str = "Let the user know the tool call was skipped in Goose chat mode. \
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
        permission_manager: &'a mut PermissionManager,
        message_tool_response: Arc<Mutex<Message>>,
        cancellation_token: Option<CancellationToken>,
    ) -> BoxStream<'a, anyhow::Result<Message>> {
        try_stream! {
            for request in tool_requests {
                if let Ok(tool_call) = request.tool_call.clone() {
                    let confirmation = Message::user().with_tool_confirmation_request(
                        request.id.clone(),
                        tool_call.name.clone(),
                        tool_call.arguments.clone(),
                        Some("Goose would like to call the above tool. Allow? (y/n):".to_string()),
                    );
                    yield confirmation;

                    let mut rx = self.confirmation_rx.lock().await;
                    while let Some((req_id, confirmation)) = rx.recv().await {
                        if req_id == request.id {
                            if confirmation.permission == Permission::AllowOnce || confirmation.permission == Permission::AlwaysAllow {
                                let (req_id, tool_result) = self.dispatch_tool_call(tool_call.clone(), request.id.clone(), cancellation_token.clone()).await;
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

                                if confirmation.permission == Permission::AlwaysAllow {
                                    permission_manager.update_user_permission(&tool_call.name, PermissionLevel::AlwaysAllow);
                                }
                            } else {
                                // User declined - add declined response
                                let mut response = message_tool_response.lock().await;
                                *response = response.clone().with_tool_response(
                                    request.id.clone(),
                                    Ok(vec![Content::text(DECLINED_RESPONSE)]),
                                );
                            }
                            break; // Exit the loop once the matching `req_id` is found
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
