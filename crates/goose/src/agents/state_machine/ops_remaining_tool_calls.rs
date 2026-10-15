//! Answers tool requests that no other operation handled, and waits while any await approval.

use std::collections::HashSet;

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{CallToolResult, ContentBlock, ErrorCode, ErrorData};
use tracing_futures::Instrument;

use crate::agents::final_output_tool::FINAL_OUTPUT_TOOL_NAME;
use crate::agents::state_machine::effects::GooseEffect;
use crate::agents::state_machine::ops_toolcalling::{
    emit_extended_pre_hooks, emit_post_tool_use, pending_tool_requests, request_was_advertised,
    run_pre_tool_hooks, tool_span, ToolDisposition,
};
use crate::agents::state_machine::{
    applied, messages_since_kickoff, not_applicable, pending_tool_confirmations,
    unanswered_tool_requests, yielded, Emitter, Operation, OperationResult,
};
use crate::agents::tool_execution::{CHAT_MODE_TOOL_SKIPPED_RESPONSE, DECLINED_RESPONSE};
use crate::config::GooseMode;
use crate::conversation::message::{Message, ToolRequest};
use crate::conversation::Conversation;
use crate::hooks::HookManager;
use crate::session::Session;

pub(super) const UNCLAIMED_TOOL_ERROR: &str = "goose.unclaimed_tool";
const NOT_HANDLED_RESPONSE: &str = "This tool call was not handled.";

fn not_handled_response(unhandled: &[&ToolRequest]) -> Message {
    let mut response = Message::user();
    for request in unhandled {
        tracing::error!(tool_call_id = %request.id, "tool call was not answered by any operation");
        response.add_tool_response_with_metadata(
            request.id.clone(),
            Ok(CallToolResult::error(vec![ContentBlock::text(
                NOT_HANDLED_RESPONSE,
            )])),
            request.metadata.as_ref(),
        );
    }
    response
}

pub struct RemainingToolCallsOperation {
    hook_manager: HookManager,
}

impl RemainingToolCallsOperation {
    pub fn new(hook_manager: HookManager) -> Self {
        Self { hook_manager }
    }
}

#[async_trait]
impl Operation<Session, GooseEffect> for RemainingToolCallsOperation {
    fn name(&self) -> &'static str {
        "remaining_tool_calls"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<GooseEffect>> {
        let active_final_output = session
            .recipe
            .as_ref()
            .is_some_and(|recipe| recipe.response.is_some());
        let messages = messages_since_kickoff(conversation)?;
        let pending: Vec<_> = pending_tool_requests(messages)
            .into_iter()
            .filter(|(request, disposition)| {
                // Reserve for RecipeOperation only the final-output calls it will
                // actually execute. A declined one is left here so it still gets a
                // response; RecipeOperation matches on Execute alone and would
                // otherwise leave the request unanswered, which strict providers can
                // reject on the next request.
                !(active_final_output
                    && *disposition == ToolDisposition::Execute
                    && request_was_advertised(messages, request)
                    && request
                        .tool_call
                        .as_ref()
                        .is_ok_and(|tool_call| tool_call.name == FINAL_OUTPUT_TOOL_NAME))
            })
            .collect();
        if pending.is_empty() {
            let awaiting_approval: HashSet<String> = pending_tool_confirmations(conversation)
                .into_iter()
                .map(|confirmation| confirmation.id)
                .collect();
            let unhandled: Vec<_> = unanswered_tool_requests(messages)
                .into_iter()
                .filter(|request| !awaiting_approval.contains(&request.id))
                .collect();
            if !unhandled.is_empty() {
                return applied([emit.message(not_handled_response(&unhandled)).into()]);
            }
            if !awaiting_approval.is_empty() {
                return yielded();
            }
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
            let (result, unclaimed) = match disposition {
                ToolDisposition::ParseError(error) => (
                    Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                        "The tool call could not be parsed: {error}. Correct the arguments and try again."
                    ))])),
                    false,
                ),
                ToolDisposition::Execute if !request_was_advertised(messages, &request) => {
                    span.record("error.type", "tool_not_available");
                    (
                        Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                            "Tool '{tool_name}' is not available."
                        ))])),
                        true,
                    )
                }
                ToolDisposition::Execute if session.goose_mode == GooseMode::Chat => (
                    Ok(CallToolResult::success(vec![ContentBlock::text(
                        CHAT_MODE_TOOL_SKIPPED_RESPONSE,
                    )])),
                    false,
                ),
                ToolDisposition::Execute => {
                    match request.tool_call.as_ref() {
                        Ok(tool_call) => {
                            let tool_input = tool_call
                                .arguments
                                .as_ref()
                                .map(|arguments| serde_json::Value::Object(arguments.clone()));
                            match run_pre_tool_hooks(
                                &self.hook_manager,
                                session,
                                &request.id,
                                &tool_call.name,
                                tool_input.as_ref(),
                            )
                            .instrument(span.clone())
                            .await
                            {
                                Err(denial) => (Err(denial), false),
                                Ok(()) => {
                                    emit_extended_pre_hooks(
                                        &self.hook_manager,
                                        &tool_call.name,
                                        tool_input.as_ref(),
                                        session,
                                    )
                                    .instrument(span.clone())
                                    .await;
                                    let (output, unclaimed) =
                                        if tool_call.name == FINAL_OUTPUT_TOOL_NAME
                                            && !active_final_output
                                        {
                                            span.record("error.type", "final_output_not_defined");
                                            (
                                                Err(ErrorData::new(
                                                    ErrorCode::INTERNAL_ERROR,
                                                    "Final output tool not defined".to_string(),
                                                    None,
                                                )),
                                                false,
                                            )
                                        } else {
                                            span.record("error.type", "tool_not_available");
                                            (
                                                Ok(CallToolResult::error(vec![
                                                    ContentBlock::text(format!(
                                                        "Tool '{}' is not available.",
                                                        tool_call.name
                                                    )),
                                                ])),
                                                true,
                                            )
                                        };
                                    emit_post_tool_use(
                                        &self.hook_manager,
                                        &session.id,
                                        &session.working_dir.to_string_lossy(),
                                        &tool_call.name,
                                        &request.id,
                                        tool_input.as_ref(),
                                        &output,
                                    )
                                    .instrument(span.clone())
                                    .await;
                                    (output, unclaimed)
                                }
                            }
                        }
                        Err(error) => (
                            Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                                "The tool call could not be parsed: {error}."
                            ))])),
                            false,
                        ),
                    }
                }
                ToolDisposition::Decline => (
                    Ok(CallToolResult::error(vec![ContentBlock::text(
                        DECLINED_RESPONSE,
                    )])),
                    false,
                ),
            };
            let mut metadata = request.metadata.clone();
            if unclaimed {
                metadata
                    .get_or_insert_default()
                    .insert(UNCLAIMED_TOOL_ERROR.to_string(), true.into());
            }
            response.add_tool_response_with_metadata(request.id, result, metadata.as_ref());
        }

        let response = emit.message(response);
        applied([response.into()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::state_machine::ops_tool_approval::TOOL_EXECUTABLE_KEY;
    use crate::agents::state_machine::{ConversationEffect, RunStatus};
    use crate::recipe::{Recipe, Response};
    use rmcp::model::CallToolRequestParams;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    async fn run(
        session: &Session,
        messages: Vec<Message>,
    ) -> Result<OperationResult<GooseEffect>> {
        let (tx, _rx) = mpsc::unbounded_channel();
        let emit = Emitter::new(tx, CancellationToken::new());
        RemainingToolCallsOperation::new(HookManager::default())
            .run(session, &Conversation::new_unvalidated(messages), &emit)
            .await
    }

    #[tokio::test]
    async fn a_call_awaiting_approval_yields() -> Result<()> {
        let messages = vec![
            Message::user().with_text("run it"),
            Message::assistant().with_tool_request_with_metadata(
                "call",
                Ok(CallToolRequestParams::new("shell")),
                None,
                Some(serde_json::json!({ TOOL_EXECUTABLE_KEY: false })),
            ),
            Message::assistant()
                .with_action_required("call", "shell".to_string(), Default::default(), None)
                .user_only(),
        ];

        let OperationResult::Applied(step) = run(&Session::default(), messages).await? else {
            panic!("expected the operation to yield");
        };
        assert_eq!(step.status, RunStatus::Yielded);
        assert!(step.effects.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn a_call_no_operation_answered_gets_an_error_and_the_turn_continues() -> Result<()> {
        let session = Session {
            recipe: Some(
                Recipe::builder()
                    .title("recipe")
                    .description("recipe")
                    .instructions("answer")
                    .response(Response {
                        json_schema: Some(serde_json::json!({ "type": "object" })),
                    })
                    .build()
                    .unwrap(),
            ),
            ..Session::default()
        };
        let messages = vec![
            Message::user().with_text("go"),
            Message::assistant().with_tool_request(
                "final",
                Ok(CallToolRequestParams::new(FINAL_OUTPUT_TOOL_NAME)),
            ),
        ];

        let OperationResult::Applied(step) = run(&session, messages).await? else {
            panic!("expected the operation to answer the call");
        };
        assert_eq!(step.status, RunStatus::Continuing);
        let [GooseEffect::Conversation(ConversationEffect::AppendMessage(message))] =
            step.effects.as_slice()
        else {
            panic!("expected one answer message");
        };
        let response = message.content[0].as_tool_response().unwrap();
        assert_eq!(response.id, "final");
        let result = response.tool_result.as_ref().unwrap();
        assert_eq!(result.is_error, Some(true));
        assert_eq!(
            result.content[0].as_text().unwrap().text,
            NOT_HANDLED_RESPONSE
        );
        Ok(())
    }
}
