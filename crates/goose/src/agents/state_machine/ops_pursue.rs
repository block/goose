//! Runs the /pursue slash command: like /grind but unbounded — the agent works
//! toward the goal until it confirms completion by calling the pursue__done tool.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rmcp::model::{CallToolResult, ContentBlock, ErrorCode, ErrorData, Tool, ToolAnnotations};

use crate::agents::state_machine::ops_recipe::has_sibling_tool_calls;
use crate::agents::state_machine::ops_toolcalling::{
    pending_advertised_tool_requests, ToolDisposition,
};
use crate::agents::state_machine::{
    applied, ends_turn, messages_since_kickoff, not_applicable, yielded_with, ConversationEffect,
    Emitter, GooseEffect, Operation, OperationResult, SlashCommand,
};
use crate::config::GooseMode;
use crate::conversation::message::{Message, SystemNotificationType, ToolRequest};
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::session::{GoalState, Session};

pub const PURSUE_DONE_TOOL_NAME: &str = "pursue__done";
pub const PURSUE_DONE_SUCCESS_MESSAGE: &str = "Pursue goal marked complete.";
const PURSUE_DONE_NOT_ALONE_MESSAGE: &str =
    "pursue__done was not accepted because it was called together with other tools. \
     Read the results of those tools, then call pursue__done on its own.";
const PURSUE_DONE_MISSING_SUMMARY_MESSAGE: &str =
    "pursue__done requires a non-empty `summary` string argument.";

fn pursue_done_tool() -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "summary": {
                "type": "string",
                "description": "A summary of what was accomplished toward the goal."
            }
        },
        "required": ["summary"]
    });
    Tool::new(
        PURSUE_DONE_TOOL_NAME.to_string(),
        "Confirm the pursue goal is fully complete and end the pursuit. \
         Call this only once the goal has been entirely accomplished, on its own, \
         in a reply with no other tool calls.",
        schema.as_object().unwrap().clone(),
    )
    .annotate(
        ToolAnnotations::with_title("Pursue Done".to_string())
            .read_only(false)
            .destructive(false)
            .idempotent(true)
            .open_world(false),
    )
}

fn pursue_prompt(goal: &str) -> String {
    format!(
        "# Pursue Mode\n\n\
         You are pursuing this goal and must not stop until it is fully complete:\n\n\
         **Goal:** {goal}\n\n\
         Do not end your turn to ask whether you should continue; keep working. \
         When — and only when — the goal is fully accomplished, call the \
         `{PURSUE_DONE_TOOL_NAME}` tool with a summary of what you did. \
         That call ends the pursuit."
    )
}

pub struct PursueOperation {
    provider: Arc<dyn Provider>,
}

impl PursueOperation {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    fn done_tool_response(
        request: ToolRequest,
        result: Result<CallToolResult, ErrorData>,
    ) -> Message {
        let mut response = Message::user().with_generated_id_if_missing();
        response.add_tool_response_with_metadata(request.id, result, request.metadata.as_ref());
        response
    }
}

#[async_trait]
impl Operation<Session, GooseEffect> for PursueOperation {
    fn name(&self) -> &'static str {
        "pursue"
    }

    async fn run_command(
        &self,
        command: &SlashCommand<'_>,
        session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<GooseEffect>> {
        if command.command != "pursue" {
            return not_applicable();
        }
        let mut state = GoalState::of(session);
        let params = command.params_str;
        let clearing = matches!(params, "off" | "clear" | "none");
        let mut starts_turn = !params.is_empty() && !clearing;
        let mut changed = false;

        let refusal = if starts_turn && session.goose_mode == GooseMode::Chat {
            starts_turn = false;
            Some(
                "/pursue is not available in chat mode: the agent needs tools to confirm completion."
                    .to_string(),
            )
        } else if starts_turn && !self.provider.supports_builtin_tools() {
            starts_turn = false;
            Some(format!(
                "/pursue is not available for provider '{}' because it does not receive goose's \
                 built-in tools, so the agent could never confirm completion.",
                self.provider.get_name()
            ))
        } else {
            None
        };

        let response = if let Some(refusal) = refusal {
            Message::assistant().with_text(refusal)
        } else if params.is_empty() {
            match &state.pursue {
                Some(goal) => {
                    Message::assistant().with_text(format!("Current pursue goal: {goal}"))
                }
                None => Message::assistant()
                    .with_text("No pursue goal set. Use `/pursue <description>` to set one."),
            }
        } else if clearing {
            state.pursue = None;
            changed = true;
            Message::assistant().with_text("Pursue cleared. The agent will finish normally.")
        } else {
            state.pursue = Some(params.to_string());
            changed = true;
            Message::assistant().with_text(format!(
                "Pursue goal set. The agent will keep working until it calls the \
                 `{PURSUE_DONE_TOOL_NAME}` tool to confirm completion:\n\n> {params}"
            ))
        };

        let command_message = messages_since_kickoff(conversation)?
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("slash command conversation has no kickoff message"))?;
        let message_id = command_message
            .id
            .clone()
            .ok_or_else(|| anyhow!("Persisted slash command message has no id"))?;
        let command_message = command_message.with_visibility(true, false);
        let response = response.with_visibility(true, false);
        emit.message(command_message);
        let response = emit.message(response);

        let mut effects = vec![
            ConversationEffect::SetMessageVisibility {
                message_id,
                user_visible: true,
                agent_visible: false,
            }
            .into(),
            response.into(),
        ];
        if changed {
            effects.push(GooseEffect::extension_state(&state)?);
        }
        if !starts_turn {
            return yielded_with(effects);
        }
        effects.push(
            Message::user()
                .with_text(format!(
                    "Start working toward this goal now:\n\n**Goal:** {params}\n\n\
                     Keep going until it is fully done, then call the `{PURSUE_DONE_TOOL_NAME}` \
                     tool with a summary of what you accomplished."
                ))
                .with_visibility(false, true)
                .into(),
        );
        applied(effects)
    }

    async fn inference_tools(&self, session: &Session) -> Result<Vec<Tool>> {
        if GoalState::of(session).pursue.is_none() {
            return Ok(Vec::new());
        }
        Ok(vec![pursue_done_tool()])
    }

    async fn prompt_parts(
        &self,
        session: &Session,
        _conversation: &Conversation,
    ) -> Result<Vec<(String, String)>> {
        let state = GoalState::of(session);
        Ok(state
            .pursue
            .as_deref()
            .map(|goal| ("pursue".to_string(), pursue_prompt(goal)))
            .into_iter()
            .collect())
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<GooseEffect>> {
        let mut state = GoalState::of(session);
        let Some(goal) = state.pursue.clone() else {
            return not_applicable();
        };
        let messages = messages_since_kickoff(conversation)?;

        let pending = pending_advertised_tool_requests(messages).into_iter().find(
            |(request, disposition)| {
                *disposition == ToolDisposition::Execute
                    && request
                        .tool_call
                        .as_ref()
                        .is_ok_and(|tool_call| tool_call.name == PURSUE_DONE_TOOL_NAME)
            },
        );
        if let Some((request, _)) = pending {
            if has_sibling_tool_calls(messages, &request.id) {
                let response = emit.message(Self::done_tool_response(
                    request,
                    Err(ErrorData::new(
                        ErrorCode::INVALID_REQUEST,
                        PURSUE_DONE_NOT_ALONE_MESSAGE,
                        None,
                    )),
                ));
                return applied([response.into()]);
            }
            let summary = request
                .tool_call
                .as_ref()
                .ok()
                .and_then(|tool_call| tool_call.arguments.as_ref())
                .and_then(|arguments| arguments.get("summary"))
                .and_then(|summary| summary.as_str())
                .map(str::trim)
                .filter(|summary| !summary.is_empty())
                .map(str::to_string);
            let Some(summary) = summary else {
                let response = emit.message(Self::done_tool_response(
                    request,
                    Err(ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        PURSUE_DONE_MISSING_SUMMARY_MESSAGE,
                        None,
                    )),
                ));
                return applied([response.into()]);
            };
            let response = emit.message(Self::done_tool_response(
                request,
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    PURSUE_DONE_SUCCESS_MESSAGE,
                )])),
            ));
            state.pursue = None;
            let done = emit.message(
                Message::assistant().with_text(format!("Pursue goal complete:\n\n{summary}")),
            );
            return yielded_with([
                response.into(),
                done.into(),
                GooseEffect::extension_state(&state)?,
            ]);
        }

        if ends_turn(messages) {
            let nudge = format!(
                "Keep working. The pursue goal is not yet complete:\n\n\
                 **Goal:** {goal}\n\n\
                 When it is fully done, call the `{PURSUE_DONE_TOOL_NAME}` tool with a summary \
                 of what you accomplished. Until then, continue."
            );
            let message = Message::user()
                .with_text(&nudge)
                .with_visibility(false, true);
            emit.message(Message::assistant().with_system_notification(
                SystemNotificationType::InlineMessage,
                format!("Pursue: {goal}"),
            ));
            return applied([message.into()]);
        }

        not_applicable()
    }
}
