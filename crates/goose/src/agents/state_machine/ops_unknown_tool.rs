//! Turns tool requests that no operation can handle into tool errors.

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{CallToolResult, Content};

use crate::agents::state_machine::operation::{
    applied, messages_since_kickoff, not_applicable, Emitter, Operation, OperationResult,
};
use crate::agents::state_machine::ops_toolcalling::{pending_tool_requests, ToolDisposition};
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::session::Session;

pub struct UnknownToolOperation;

#[async_trait]
impl Operation for UnknownToolOperation {
    fn name(&self) -> &'static str {
        "unknown_tool"
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        let pending = pending_tool_requests(messages_since_kickoff(conversation)?);
        if pending.is_empty() {
            return not_applicable(emit);
        }

        let mut response = Message::user().with_generated_id();
        for (request, disposition) in pending {
            let message = match disposition {
                ToolDisposition::ParseError(error) => {
                    format!("The tool call could not be parsed: {error}. Correct the arguments and try again.")
                }
                ToolDisposition::Execute | ToolDisposition::Decline => request
                    .tool_call
                    .as_ref()
                    .map(|tool_call| format!("Tool '{}' is not available.", tool_call.name))
                    .unwrap_or_else(|error| format!("The tool call could not be parsed: {error}.")),
            };
            response.add_tool_response_with_metadata(
                request.id,
                Ok(CallToolResult::error(vec![Content::text(message)])),
                request.metadata.as_ref(),
            );
        }

        emit.emit(AgentEvent::Message(response.clone())).await;
        applied([response.into()])
    }
}
