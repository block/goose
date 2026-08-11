use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio_util::sync::CancellationToken;

use crate::agents::state_machine::operation::{
    messages_since_kickoff, Emitter, Inference, InferenceInput, Operation, OperationFuture,
    OperationResult, StateEffect, StepResult,
};
use crate::agents::state_machine::usage;
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::hooks::{HookContext, HookEvent, HookManager};
use crate::session::{Session, SessionManager};

pub enum Step<'a> {
    Operation(Arc<dyn Operation + 'a>),
    Inference(Arc<dyn Inference + 'a>),
}

impl Step<'_> {
    fn operation(&self) -> &dyn Operation {
        match self {
            Step::Operation(operation) => operation.as_ref(),
            Step::Inference(inference) => inference.as_ref(),
        }
    }
}

pub struct StateMachine<'a> {
    steps: Vec<Step<'a>>,
    cancel: CancellationToken,
    hook_manager: HookManager,
}

impl<'a> StateMachine<'a> {
    pub fn new(steps: Vec<Step<'a>>, cancel: CancellationToken) -> Self {
        Self {
            steps,
            cancel,
            hook_manager: HookManager::default(),
        }
    }

    pub fn with_hook_manager(mut self, hook_manager: HookManager) -> Self {
        self.hook_manager = hook_manager;
        self
    }

    async fn emit_entry_hooks(&self, session: &Session, conversation: &Conversation) -> Result<()> {
        let messages = messages_since_kickoff(conversation)?;
        if Self::has_agent_reply(messages) {
            return Ok(());
        }

        let messages_before_kickoff =
            &conversation.messages()[..conversation.len() - messages.len()];
        if !messages_before_kickoff.iter().any(|message| {
            message.role == rmcp::model::Role::User
                && message.is_user_visible()
                && !message.is_tool_response()
        }) {
            self.hook_manager
                .emit(
                    HookEvent::SessionStart,
                    HookContext::new(HookEvent::SessionStart, &session.id)
                        .with_working_dir(session.working_dir.to_string_lossy().to_string()),
                )
                .await;
        }

        let prompt = messages
            .first()
            .map(Message::as_concat_text)
            .unwrap_or_default();
        if !prompt.is_empty() {
            self.hook_manager
                .emit(
                    HookEvent::UserPromptSubmit,
                    HookContext::new(HookEvent::UserPromptSubmit, &session.id)
                        .with_message(prompt)
                        .with_working_dir(session.working_dir.to_string_lossy().to_string()),
                )
                .await;
        }
        Ok(())
    }

    fn has_agent_reply(messages: &[Message]) -> bool {
        messages.iter().any(|message| {
            message.role == rmcp::model::Role::Assistant
                && ((message.is_user_visible() && message.is_agent_visible())
                    || message.error_kind().is_some())
        })
    }

    pub async fn step(&self, session: &Session, emit: &Emitter) -> Result<Option<StepResult>> {
        let conversation = session
            .conversation
            .as_ref()
            .ok_or_else(|| anyhow!("state-machine session loaded without conversation"))?;

        self.emit_entry_hooks(session, conversation).await?;

        for step in &self.steps {
            let name = step.operation().name();
            let result = if self.cancel.is_cancelled() {
                OperationResult::NotApplicable
            } else {
                let step_fut: OperationFuture<'_, Result<OperationResult>> = match step {
                    Step::Operation(operation) => operation.run(session, conversation, emit),
                    Step::Inference(inference) => {
                        let mut input = InferenceInput::default();
                        for operation in self.steps.iter().map(|step| step.operation()) {
                            input
                                .tools
                                .extend(operation.inference_tools(session).await?);
                            input
                                .prompt_parts
                                .extend(operation.prompt_parts(session, conversation).await?);
                            input
                                .moim_parts
                                .extend(operation.moim_parts(session, conversation).await?);
                        }
                        inference.infer(session, conversation, input, emit)
                    }
                };
                step_fut.await?
            };
            let cancelled = self.cancel.is_cancelled();
            let result = if cancelled {
                step.operation()
                    .cancel(session, conversation, result, emit)
                    .await?
            } else {
                result
            };

            match result {
                OperationResult::NotApplicable => {}
                OperationResult::Applied(mut result) => {
                    // `step` and `apply` are separate entry points; a caller that
                    // drives steps itself still gets effects it can persist.
                    result.ensure_message_ids();
                    if cancelled {
                        result.yield_to_client = true;
                    }
                    tracing::debug!(target: "goose::state_machine", step = name, "applied step");
                    return Ok(Some(result));
                }
            }
        }

        Ok(None)
    }

    pub async fn apply(
        &self,
        session_manager: &SessionManager,
        session: &Session,
        result: &mut StepResult,
        emit: &Emitter,
    ) -> Result<()> {
        result.ensure_message_ids();
        usage::enrich(session, &mut result.effects);

        for effect in &result.effects {
            match effect {
                StateEffect::AppendMessage(message) => {
                    session_manager.add_message(&session.id, message).await?;
                }
                StateEffect::ReplaceConversation {
                    conversation,
                    usage: replacement_usage,
                } => {
                    if let Some(usage) = replacement_usage {
                        usage::record(session_manager, session, usage, true).await?;
                    }
                    session_manager
                        .replace_conversation(&session.id, conversation)
                        .await?;
                    session_manager
                        .update(&session.id)
                        .usage(usage::estimate_context(conversation).await?)
                        .apply()
                        .await?;
                }
                StateEffect::PatchToolRequestMeta {
                    tool_call_id,
                    patch,
                } => {
                    session_manager
                        .update_tool_request_meta(&session.id, tool_call_id, patch.clone())
                        .await?;
                }
                StateEffect::SetMessageVisibility {
                    message_id,
                    user_visible,
                    agent_visible,
                } => {
                    session_manager
                        .update_message_metadata(&session.id, message_id, |mut metadata| {
                            metadata.user_visible = *user_visible;
                            metadata.agent_visible = *agent_visible;
                            metadata
                        })
                        .await?;
                }
                StateEffect::SetRecipe(recipe) => {
                    session_manager
                        .update(&session.id)
                        .recipe(recipe.as_ref().clone())
                        .apply()
                        .await?;
                }
                StateEffect::SetExtensionData(extension_data) => {
                    session_manager
                        .update(&session.id)
                        .extension_data(extension_data.clone())
                        .apply()
                        .await?;
                }
                StateEffect::RecordUsage(usage) => {
                    usage::record(session_manager, session, usage, false).await?;
                }
            }
        }

        for effect in &result.effects {
            match effect {
                StateEffect::AppendMessage(message) => {
                    if let Some(usage) = message
                        .metadata
                        .usage
                        .as_deref()
                        .filter(|_| !message.user_visible_content().content.is_empty())
                        .cloned()
                    {
                        emit.emit(AgentEvent::MessageUsage {
                            message_id: message.id.clone(),
                            usage,
                        })
                        .await;
                    }
                }
                StateEffect::ReplaceConversation { conversation, .. } => {
                    emit.emit(AgentEvent::HistoryReplaced(conversation.clone()))
                        .await;
                }
                StateEffect::RecordUsage(usage) => {
                    emit.emit(AgentEvent::Usage(usage.clone())).await
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn run(
        &self,
        session_manager: &SessionManager,
        session_id: &str,
        emit: &Emitter,
    ) -> Result<Session> {
        let entry_session = session_manager.get_session(session_id, true).await?;
        if let Some(input) = entry_session
            .conversation
            .as_ref()
            .and_then(|conversation| messages_since_kickoff(conversation).ok())
            .and_then(|messages| messages.first())
            .map(Message::user_visible_content)
            .map(|message| message.as_concat_text())
            .filter(|text| !text.is_empty())
        {
            tracing::Span::current().record("trace_input", input.as_str());
        }

        let mut turn_usage = goose_providers::conversation::token_usage::Usage::default();
        loop {
            let session = session_manager.get_session(session_id, true).await?;
            let Some(mut result) = self.step(&session, emit).await? else {
                break;
            };
            for effect in &result.effects {
                match effect {
                    StateEffect::RecordUsage(usage)
                    | StateEffect::ReplaceConversation {
                        usage: Some(usage), ..
                    } => turn_usage += usage.usage,
                    _ => {}
                }
            }
            self.apply(session_manager, &session, &mut result, emit)
                .await?;
            if result.yield_to_client {
                break;
            }
        }

        let session = session_manager.get_session(session_id, true).await?;
        let last_assistant_text = session
            .conversation
            .as_ref()
            .and_then(|conversation| messages_since_kickoff(conversation).ok())
            .into_iter()
            .flatten()
            .rev()
            .filter(|message| message.role == rmcp::model::Role::Assistant)
            .map(Message::user_visible_content)
            .map(|message| message.as_concat_text())
            .find(|text| !text.is_empty())
            .unwrap_or_default();
        if !last_assistant_text.is_empty() {
            let span = tracing::Span::current();
            span.record("trace_output", last_assistant_text.as_str());
            if crate::agents::gen_ai_telemetry::capture_message_content() {
                let output =
                    crate::agents::gen_ai_telemetry::simple_output_json(&last_assistant_text);
                span.record("gen_ai.output.messages", output.as_str());
            }
        }
        crate::agents::gen_ai_telemetry::record_usage(&tracing::Span::current(), &turn_usage);

        Ok(session)
    }
}
