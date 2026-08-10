//! Applies recipe commands and enforces their structured final output.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use rmcp::model::Tool;
use tracing_futures::Instrument;

use crate::agents::final_output_tool::{
    FINAL_OUTPUT_CONTINUATION_MESSAGE, FINAL_OUTPUT_TOOL_NAME, FinalOutputTool,
};
use crate::agents::state_machine::operation::{
    Emitter, Operation, OperationResult, SlashCommand, StateEffect, applied, ends_turn,
    last_effective_role, messages_since_kickoff, not_applicable, yielded_with,
};
use crate::agents::state_machine::ops_toolcalling::{
    ToolDisposition, pending_tool_requests, tool_span,
};
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::{Conversation, EffectiveRole};
use crate::session::Session;

pub struct RecipeOperation;

impl RecipeOperation {
    fn final_output(session: &Session) -> Result<Option<FinalOutputTool>> {
        session
            .recipe
            .as_ref()
            .and_then(|recipe| recipe.response.clone())
            .map(FinalOutputTool::try_new)
            .transpose()
            .map_err(|error| anyhow!(error))
    }

    fn successful_final_output(messages: &[Message]) -> Option<String> {
        let mut request_id_counts = HashMap::new();
        for request in messages
            .iter()
            .flat_map(|message| &message.content)
            .filter_map(|content| match content {
                MessageContent::ToolRequest(request) => Some(request),
                _ => None,
            })
        {
            *request_id_counts.entry(request.id.as_str()).or_insert(0) += 1;
        }

        let successful_responses: HashSet<&str> = messages
            .iter()
            .flat_map(|message| &message.content)
            .filter_map(|content| match content {
                MessageContent::ToolResponse(response)
                    if response
                        .tool_result
                        .as_ref()
                        .is_ok_and(|result| result.is_error != Some(true)) =>
                {
                    Some(response.id.as_str())
                }
                _ => None,
            })
            .collect();

        messages
            .iter()
            .rev()
            .flat_map(|message| message.content.iter().rev())
            .find_map(|content| match content {
                MessageContent::ToolRequest(request)
                    if request_id_counts.get(request.id.as_str()) == Some(&1)
                        && successful_responses.contains(request.id.as_str()) =>
                {
                    request.tool_call.as_ref().ok().and_then(|tool_call| {
                        (tool_call.name == FINAL_OUTPUT_TOOL_NAME).then(|| {
                            serde_json::Value::Object(
                                tool_call.arguments.clone().unwrap_or_default(),
                            )
                            .to_string()
                        })
                    })
                }
                _ => None,
            })
    }

    async fn command_error(
        &self,
        conversation: &Conversation,
        message: String,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        let command = messages_since_kickoff(conversation)?
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("recipe command conversation has no kickoff message"))?;
        let message_id = command
            .id
            .clone()
            .ok_or_else(|| anyhow!("Persisted slash command message has no id"))?;
        let command = command.with_visibility(true, false);
        let response = Message::assistant()
            .with_text(message)
            .with_visibility(true, false);
        emit.message(command).await;
        let response = emit.message(response).await;
        yielded_with([
            StateEffect::SetMessageVisibility {
                message_id,
                user_visible: true,
                agent_visible: false,
            },
            response.into(),
        ])
    }
}

#[async_trait]
impl Operation for RecipeOperation {
    fn name(&self) -> &'static str {
        "recipe"
    }

    async fn run_command(
        &self,
        command: &SlashCommand<'_>,
        _session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        let (recipe, prompt) = match crate::slash_commands::recipe_slash_command::resolve_command(
            command.command,
            command.params_str,
        ) {
            Ok(Some(recipe)) => recipe,
            Ok(None) => return not_applicable(),
            Err(error) => return self.command_error(conversation, error, emit).await,
        };

        if let Some(response) = recipe.response.clone() {
            if let Err(error) = FinalOutputTool::try_new(response) {
                return self
                    .command_error(conversation, format!("Recipe is not valid: {error}"), emit)
                    .await;
            }
        }

        #[cfg(feature = "telemetry")]
        crate::posthog::emit_custom_slash_command_used();

        let command_message = messages_since_kickoff(conversation)?
            .first()
            .ok_or_else(|| anyhow!("recipe command conversation has no kickoff message"))?;
        let message_id = command_message
            .id
            .clone()
            .ok_or_else(|| anyhow!("Persisted slash command message has no id"))?;
        applied([
            StateEffect::SetMessageVisibility {
                message_id,
                user_visible: true,
                agent_visible: false,
            },
            StateEffect::SetRecipe(Box::new(Some(recipe))),
            Message::user()
                .with_text(prompt)
                .with_visibility(false, true)
                .into(),
        ])
    }

    async fn inference_tools(&self, session: &Session) -> Result<Vec<Tool>> {
        Ok(Self::final_output(session)?
            .as_ref()
            .map(FinalOutputTool::tool)
            .into_iter()
            .collect())
    }

    async fn prompt_parts(
        &self,
        session: &Session,
        _conversation: &Conversation,
    ) -> Result<Vec<(String, String)>> {
        Ok(Self::final_output(session)?
            .as_ref()
            .map(|tool| ("final_output".to_string(), tool.system_prompt()))
            .into_iter()
            .collect())
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        let Some(mut final_output) = Self::final_output(session)? else {
            return not_applicable();
        };

        let messages = messages_since_kickoff(conversation)?;
        let pending = pending_tool_requests(messages)
            .into_iter()
            .find(|(request, disposition)| {
                *disposition == ToolDisposition::Execute
                    && request
                        .tool_call
                        .as_ref()
                        .is_ok_and(|tool_call| tool_call.name == FINAL_OUTPUT_TOOL_NAME)
            });
        if let Some((request, _)) = pending {
            let tool_call = request
                .tool_call
                .map_err(|error| anyhow!("final output tool call could not be parsed: {error}"))?;
            let span = tool_span(&tool_call.name, &request.id, &session.id);
            let result = final_output
                .execute_tool_call(tool_call)
                .instrument(span.clone())
                .await;
            let output = result.result.instrument(span.clone()).await;
            match &output {
                Ok(result) if result.is_error == Some(true) => {
                    span.record("error.type", "tool_error");
                }
                Err(_) => {
                    span.record("error.type", "tool_execution_error");
                }
                _ => {}
            }
            let mut response = Message::user();
            response.add_tool_response_with_metadata(request.id, output, request.metadata.as_ref());
            let response = emit.message(response).await;
            return applied([response.into()]);
        }

        if let Some(output) = Self::successful_final_output(messages) {
            if last_effective_role(messages)? == EffectiveRole::Tool {
                let message = Message::assistant().with_text(output);
                let message = emit.message(message).await;
                return applied([message.into()]);
            }
            return not_applicable();
        }

        if ends_turn(messages) {
            let message = Message::user().with_text(FINAL_OUTPUT_CONTINUATION_MESSAGE);
            let message = emit.message(message).await;
            return applied([message.into()]);
        }

        not_applicable()
    }
}

#[cfg(test)]
mod tests {
    use rmcp::model::{CallToolRequestParams, CallToolResult};
    use serde_json::json;

    use super::*;

    fn final_output_request(id: &str, arguments: serde_json::Value) -> Message {
        Message::assistant().with_tool_request(
            id,
            Ok(
                CallToolRequestParams::new(FINAL_OUTPUT_TOOL_NAME).with_arguments(
                    arguments
                        .as_object()
                        .expect("final output arguments must be an object")
                        .clone(),
                ),
            ),
        )
    }

    fn successful_response(id: &str) -> Message {
        Message::user().with_tool_response(id, Ok(CallToolResult::success(Vec::new())))
    }

    #[test]
    fn duplicate_request_id_cannot_rebind_validated_final_output() {
        let messages = [
            final_output_request("reused-id", json!({ "result": "validated" })),
            final_output_request("reused-id", json!({ "unvalidated": true })),
            successful_response("reused-id"),
        ];

        assert_eq!(RecipeOperation::successful_final_output(&messages), None);
    }

    #[test]
    fn unique_request_id_preserves_final_output() {
        let messages = [
            final_output_request("unique-id", json!({ "result": "validated" })),
            successful_response("unique-id"),
        ];

        assert_eq!(
            RecipeOperation::successful_final_output(&messages),
            Some(r#"{"result":"validated"}"#.to_string())
        );
    }
}
