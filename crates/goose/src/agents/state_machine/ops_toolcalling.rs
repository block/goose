use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use rmcp::model::{CallToolResult, Content, ErrorCode, ErrorData, Role};

use crate::agents::agent::{tool_stream, ToolStreamItem};
use crate::agents::extension_manager::ExtensionManager;
use crate::agents::state_machine::operation::{Emitter, Operation, OperationResult};
use crate::agents::state_machine::ops_tool_approval::request_executable;
use crate::agents::tool_execution::DECLINED_RESPONSE;
use crate::agents::tool_execution::{ToolCallContext, ToolCallResult};
use crate::agents::AgentEvent;
use crate::conversation::message::{ActionRequiredData, Message, MessageContent, ToolRequest};
use crate::conversation::Conversation;
use crate::session::{Session, SessionManager};

/// Executes pending tool requests: when the last message is an assistant
/// message carrying tool requests that have not yet been answered, dispatch
/// each one through the extension manager and append a single message with the
/// collected responses.
///
/// Scoped to ordinary extension tools. Approval, frontend tools (which yield to
/// the client), platform tools, and hooks are handled elsewhere.
pub struct ToolExecutionOperation {
    extension_manager: Arc<ExtensionManager>,
    session_manager: Arc<SessionManager>,
}

impl ToolExecutionOperation {
    pub fn new(
        extension_manager: Arc<ExtensionManager>,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        Self {
            extension_manager,
            session_manager,
        }
    }
}

/// Index of the last genuine user prompt — the message that started the
/// current request. Tool requests before it are stale leftovers (e.g. from a
/// crash mid-execution): they are not executed or re-approved, and the LLM op
/// hides them from the provider. Approval-flow re-entry messages
/// (`ToolConfirmationResponse`, agent-invisible) don't move the boundary, so
/// requests awaiting approval stay current.
pub(crate) fn current_request_start(messages: &[Message]) -> usize {
    messages
        .iter()
        .rposition(|m| m.role == Role::User && !m.is_tool_response() && m.is_agent_visible())
        .unwrap_or(0)
}

fn pending_tool_requests(conversation: &Conversation) -> Vec<(ToolRequest, ToolDisposition)> {
    let mut answered = HashSet::new();
    let mut approval_requests = HashSet::new();
    let mut approvals = std::collections::HashMap::new();
    for message in conversation.messages() {
        for content in &message.content {
            match content {
                MessageContent::ToolResponse(response) => {
                    answered.insert(response.id.clone());
                }
                MessageContent::ActionRequired(action) => match &action.data {
                    ActionRequiredData::ToolConfirmation { id, .. } => {
                        approval_requests.insert(id.clone());
                    }
                    ActionRequiredData::ToolConfirmationResponse { id, permission } => {
                        approvals.insert(id.clone(), permission.clone());
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    let start = current_request_start(conversation.messages());
    conversation.messages()[start..]
        .iter()
        .filter(|message| message.role == Role::Assistant)
        .flat_map(|message| {
            message.content.iter().filter_map(|c| match c {
                MessageContent::ToolRequest(req)
                    if req.tool_call.is_ok() && !answered.contains(&req.id) =>
                {
                    match request_executable(req).unwrap_or(true) {
                        true => Some((req.clone(), ToolDisposition::Execute)),
                        false => {
                            if approval_requests.contains(&req.id)
                                && !approval_denied(approvals.get(&req.id))
                            {
                                None
                            } else {
                                Some((req.clone(), ToolDisposition::Decline))
                            }
                        }
                    }
                }
                _ => None,
            })
        })
        .collect()
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ToolDisposition {
    Execute,
    Decline,
}

fn approval_denied(permission: Option<&crate::permission::Permission>) -> bool {
    matches!(
        permission,
        Some(
            crate::permission::Permission::DenyOnce
                | crate::permission::Permission::AlwaysDeny
                | crate::permission::Permission::Cancel
        )
    )
}

#[async_trait]
impl Operation for ToolExecutionOperation {
    fn name(&self) -> &'static str {
        "tool_execution"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        let pending = pending_tool_requests(conversation);
        let requests: Vec<_> = pending.iter().map(|(request, _)| request.clone()).collect();
        if requests.is_empty() {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let mut tool_streams = Vec::new();
        for (request, disposition) in &pending {
            if *disposition != ToolDisposition::Execute {
                continue;
            }
            let tool_call = request
                .tool_call
                .clone()
                .map_err(|e| anyhow!("tool call could not be parsed: {e}"))?;
            let ctx = ToolCallContext::new(
                session.id.clone(),
                Some(session.working_dir.clone()),
                Some(request.id.clone()),
            );
            let result = self
                .extension_manager
                .dispatch_tool_call(&ctx, tool_call, emit.cancel_token().clone())
                .await
                .unwrap_or_else(|e| {
                    let error_data = e.downcast::<ErrorData>().unwrap_or_else(|e| {
                        ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    });
                    ToolCallResult::from(Err(error_data))
                });

            let req_id = request.id.clone();
            let stream = tool_stream(
                result
                    .notification_stream
                    .unwrap_or_else(|| Box::new(futures::stream::empty())),
                result
                    .action_required_stream
                    .unwrap_or_else(|| Box::new(futures::stream::empty())),
                result.result,
            )
            .map(move |item| (req_id.clone(), item));
            tool_streams.push(stream);
        }

        let mut combined = futures::stream::select_all(tool_streams);
        let mut response = Message::user().with_generated_id();
        for (request, disposition) in &pending {
            if *disposition == ToolDisposition::Decline {
                response.add_tool_response_with_metadata(
                    request.id.clone(),
                    Ok(CallToolResult::error(vec![Content::text(
                        DECLINED_RESPONSE,
                    )])),
                    request.metadata.as_ref(),
                );
            }
        }

        loop {
            tokio::select! {
                biased;
                _ = emit.cancelled() => break,
                item = combined.next() => {
                    let Some((request_id, item)) = item else { break };
                    match item {
                        ToolStreamItem::Result(output) => {
                            let metadata = requests
                                .iter()
                                .find(|r| r.id == request_id)
                                .and_then(|r| r.metadata.as_ref());
                            response.add_tool_response_with_metadata(request_id, output, metadata);
                        }
                        ToolStreamItem::Message(msg) => {
                            emit.emit(AgentEvent::McpNotification((request_id, msg)))
                                .await;
                        }
                        ToolStreamItem::ActionRequired(mut msg) => {
                            if msg.id.is_none() {
                                msg = msg.with_generated_id();
                            }
                            // Persisted here rather than as an effect: this op
                            // stays blocked until the elicitation is answered
                            // (possibly much later, from another reply call
                            // that reads the conversation), so an effect
                            // returned at completion would record the question
                            // only after its answer.
                            if let Err(e) =
                                self.session_manager.add_message(&session.id, &msg).await
                            {
                                tracing::warn!("Failed to persist action-required message: {e}");
                            }
                            emit.emit(AgentEvent::Message(msg)).await;
                        }
                    }
                }
            }
        }

        let answered: HashSet<String> = response
            .get_tool_response_ids()
            .into_iter()
            .map(str::to_string)
            .collect();
        for request in &requests {
            if !answered.contains(request.id.as_str()) {
                response.add_tool_response_with_metadata(
                    request.id.clone(),
                    Ok(CallToolResult::error(vec![Content::text(
                        "Tool call was interrupted before completing",
                    )])),
                    request.metadata.as_ref(),
                );
            }
        }

        emit.emit(AgentEvent::Message(response.clone())).await;
        Ok(OperationResult::Applied(vec![response.into()]))
    }
}
