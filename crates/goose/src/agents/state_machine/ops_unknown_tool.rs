//! Turns tool requests that no operation can handle into tool errors.

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{CallToolResult, ContentBlock};

use crate::agents::state_machine::effects::GooseEffect;
use crate::agents::state_machine::ops_toolcalling::{
    pending_tool_requests, tool_span, ToolDisposition,
};
use crate::agents::state_machine::{
    applied, messages_since_kickoff, not_applicable, Emitter, Operation, OperationResult,
};
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::session::Session;

pub(super) const UNCLAIMED_TOOL_ERROR: &str = "goose.unclaimed_tool";

pub struct UnknownToolOperation;

#[async_trait]
impl Operation<Session, GooseEffect> for UnknownToolOperation {
    fn name(&self) -> &'static str {
        "unknown_tool"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<GooseEffect>> {
        let pending = pending_tool_requests(messages_since_kickoff(conversation)?);
        if pending.is_empty() {
            return not_applicable();
        }

        let mut response = Message::user();
        for (request, disposition) in pending {
            let tool_name = request
                .tool_call
                .as_ref()
                .map(|tool_call| tool_call.name.as_ref())
                .unwrap_or("unknown");
            let span = tool_span(tool_name, &request.id, &session.id);
            span.record("error.type", "tool_not_available");
            let (result, unclaimed) = match disposition {
                ToolDisposition::ParseError(error) => (
                    Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                        "The tool call could not be parsed: {error}. Correct the arguments and try again."
                    ))])),
                    false,
                ),
                ToolDisposition::Execute | ToolDisposition::Decline => request
                    .tool_call
                    .as_ref()
                    .map(|tool_call| {
                        (
                            Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                                "Tool '{}' is not available.",
                                tool_call.name
                            ))])),
                            true,
                        )
                    })
                    .unwrap_or_else(|error| {
                        (
                            Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                                "The tool call could not be parsed: {error}."
                            ))])),
                            false,
                        )
                    }),
            };
            let mut metadata = request.metadata.clone();
            if unclaimed {
                metadata
                    .get_or_insert_default()
                    .insert(UNCLAIMED_TOOL_ERROR.to_string(), true.into());
            }
            response.add_tool_response_with_metadata(request.id, result, metadata.as_ref());
        }

        let response = emit.message(response).await;
        applied([response.into()])
    }
}
