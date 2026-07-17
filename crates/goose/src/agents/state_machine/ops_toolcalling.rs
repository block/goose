use std::collections::HashSet;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use rmcp::model::{CallToolResult, Content, Role};

use crate::agents::agent::{tool_stream, ToolStreamItem};
use crate::agents::platform_extensions::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;
use crate::agents::state_machine::operation::{Emitter, Operation, OperationResult};
use crate::agents::state_machine::ops_tool_approval::request_executable;
use crate::agents::tool_execution::ToolCallResult;
use crate::agents::tool_execution::{CHAT_MODE_TOOL_SKIPPED_RESPONSE, DECLINED_RESPONSE};
use crate::agents::{Agent, AgentEvent};
use crate::config::GooseMode;
use crate::conversation::message::{ActionRequiredData, Message, MessageContent, ToolRequest};
use crate::conversation::Conversation;
use crate::session::Session;

pub struct ToolExecutionOperation<'a> {
    agent: &'a Agent,
}

impl<'a> ToolExecutionOperation<'a> {
    pub fn new(agent: &'a Agent) -> Self {
        Self { agent }
    }
}

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
                MessageContent::ToolRequest(req) if !answered.contains(&req.id) => {
                    if let Err(parse_error) = &req.tool_call {
                        return Some((
                            req.clone(),
                            ToolDisposition::ParseError(parse_error.to_string()),
                        ));
                    }
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

#[derive(Clone, Eq, PartialEq)]
enum ToolDisposition {
    Execute,
    Decline,
    ParseError(String),
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
impl Operation for ToolExecutionOperation<'_> {
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

        if self.agent.goose_mode().await == GooseMode::Chat {
            let mut response = Message::user().with_generated_id();
            for (request, disposition) in &pending {
                let result = match disposition {
                    ToolDisposition::ParseError(parse_error) => {
                        CallToolResult::error(vec![Content::text(format!(
                            "The tool call could not be parsed: {parse_error}."
                        ))])
                    }
                    _ => CallToolResult::success(vec![Content::text(
                        CHAT_MODE_TOOL_SKIPPED_RESPONSE,
                    )]),
                };
                response.add_tool_response_with_metadata(
                    request.id.clone(),
                    Ok(result),
                    request.metadata.as_ref(),
                );
            }
            emit.emit(AgentEvent::Message(response.clone())).await;
            return Ok(OperationResult::Applied(vec![response.into()]));
        }

        let manage_extensions_ids: HashSet<&str> = pending
            .iter()
            .filter_map(|(request, _)| match &request.tool_call {
                Ok(tool_call) if tool_call.name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE => {
                    Some(request.id.as_str())
                }
                _ => None,
            })
            .collect();
        let mut extension_change_failed = false;

        let mut tool_streams = Vec::new();
        for (request, disposition) in &pending {
            if *disposition != ToolDisposition::Execute {
                continue;
            }
            let tool_call = request
                .tool_call
                .clone()
                .map_err(|e| anyhow!("tool call could not be parsed: {e}"))?;
            let (_, result) = self
                .agent
                .dispatch_tool_call(
                    tool_call,
                    request.id.clone(),
                    Some(emit.cancel_token().clone()),
                    session,
                )
                .await;
            let result = result.unwrap_or_else(|error_data| ToolCallResult::from(Err(error_data)));

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
            match disposition {
                ToolDisposition::Execute => {}
                ToolDisposition::Decline => {
                    response.add_tool_response_with_metadata(
                        request.id.clone(),
                        Ok(CallToolResult::error(vec![Content::text(
                            DECLINED_RESPONSE,
                        )])),
                        request.metadata.as_ref(),
                    );
                }
                ToolDisposition::ParseError(parse_error) => {
                    response.add_tool_response_with_metadata(
                        request.id.clone(),
                        Ok(CallToolResult::error(vec![Content::text(format!(
                            "The tool call could not be parsed: {parse_error}. \
                             Correct the arguments and try again."
                        ))])),
                        request.metadata.as_ref(),
                    );
                }
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
                            if manage_extensions_ids.contains(request_id.as_str())
                                && output.is_err()
                            {
                                extension_change_failed = true;
                            }
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
                            if let Err(e) = self
                                .agent
                                .config
                                .session_manager
                                .add_message(&session.id, &msg)
                                .await
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

        if !manage_extensions_ids.is_empty() && !extension_change_failed {
            if let Err(e) = self.agent.persist_extension_state(&session.id).await {
                tracing::warn!("Failed to save extension state after runtime changes: {e}");
            }
        }

        emit.emit(AgentEvent::Message(response.clone())).await;
        Ok(OperationResult::Applied(vec![response.into()]))
    }
}
