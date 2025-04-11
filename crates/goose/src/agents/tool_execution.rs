use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_stream::try_stream;
use futures::stream::BoxStream;
use futures::stream::StreamExt;
use tokio::sync::Mutex;

use crate::config::permission::PermissionLevel;
use crate::config::PermissionManager;
use crate::message::{Message, ToolRequest};
use crate::permission::Permission;
use mcp_core::{Content, ToolError};

// Type alias to reduce complexity
pub(crate) type ToolFuture<'a> =
    Pin<Box<dyn Future<Output = (String, Result<Vec<Content>, ToolError>)> + Send + 'a>>;
pub(crate) type ToolFuturesVec<'a> = Arc<Mutex<Vec<ToolFuture<'a>>>>;

use crate::agents::Agent;

impl Agent {
    pub(crate) fn handle_approval_tool_requests<'a>(
        &'a self,
        tool_requests: &'a [ToolRequest],
        tool_futures: ToolFuturesVec<'a>,
        permission_manager: &'a mut PermissionManager,
        message_tool_response: Arc<Mutex<Message>>,
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
                    while let Some((req_id, tool_confirmation)) = rx.recv().await {
                        if req_id == request.id {
                            let confirmed = tool_confirmation.permission == Permission::AllowOnce|| tool_confirmation.permission == Permission::AlwaysAllow;
                            if confirmed {
                                // Add this tool call to the futures collection
                                let tool_future = self.dispatch_tool_call(tool_call.clone(), request.id.clone());
                                let mut futures = tool_futures.lock().await;
                                futures.push(Box::pin(tool_future));

                                if tool_confirmation.permission == Permission::AlwaysAllow {
                                    permission_manager.update_user_permission(&tool_call.name, PermissionLevel::AlwaysAllow);
                                }
                            } else {
                                // User declined - add declined response
                                let denied_content_text = Content::text(
                                    "The user has declined to run this tool. \
                                    DO NOT attempt to call this tool again. \
                                    If there are no alternative methods to proceed, clearly explain the situation and STOP.");
                                let mut response = message_tool_response.lock().await;
                                *response = response.clone().with_tool_response(
                                    request.id.clone(),
                                    Ok(vec![denied_content_text.clone()]),
                                );
                            }
                            break; // Exit the loop once the matching `req_id` is found
                        }
                    }
                }
            }
        }
        .boxed()
    }
}
