use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio_util::sync::CancellationToken;
use tracing_futures::Instrument;

use crate::agents::state_machine::operation::{
    messages_since_kickoff, Emitter, Inference, InferenceInput, Operation, OperationFuture,
    OperationResult, StateEffect, StepResult,
};
use crate::agents::state_machine::usage;
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
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
}

impl<'a> StateMachine<'a> {
    pub fn new(steps: Vec<Step<'a>>, cancel: CancellationToken) -> Self {
        Self { steps, cancel }
    }

    pub async fn step(&self, session: &Session, emit: Emitter) -> Result<Option<StepResult>> {
        if self.cancel.is_cancelled() {
            return Ok(None);
        }
        let conversation = session
            .conversation
            .as_ref()
            .ok_or_else(|| anyhow!("state-machine session loaded without conversation"))?;
        let mut emitter = Some(emit);

        for step in &self.steps {
            let emit = emitter
                .take()
                .ok_or_else(|| anyhow!("step did not return the event emitter"))?;
            let name = step.operation().name();
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

            match step_fut.await? {
                OperationResult::NotApplicable(emit) => emitter = Some(emit),
                OperationResult::Applied(result) => {
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
                    message_id,
                    tool_call_id,
                    patch,
                } => {
                    session_manager
                        .update_tool_request_meta(
                            &session.id,
                            message_id,
                            tool_call_id,
                            patch.clone(),
                        )
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
                        .recipe(recipe.clone())
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
        emit: Emitter,
    ) -> Result<Session> {
        let span = tracing::info_span!(
            target: "goose::state_machine",
            "invoke_agent goose",
            "gen_ai.operation.name" = "invoke_agent",
            "gen_ai.agent.name" = "goose",
            "gen_ai.conversation.id" = %session_id,
            trace_input = tracing::field::Empty,
            trace_output = tracing::field::Empty,
            session.id = %session_id,
            session.user = %crate::session_context::session_user(),
            session.host = %crate::session_context::session_host(),
            session.agent_type = "goose",
        );

        async {
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

            loop {
                if self.cancel.is_cancelled() {
                    break;
                }
                let session = session_manager.get_session(session_id, true).await?;
                let Some(mut result) = self.step(&session, emit.clone()).await? else {
                    break;
                };
                self.apply(session_manager, &session, &mut result, &emit)
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
                tracing::Span::current().record("trace_output", last_assistant_text.as_str());
            }

            Ok(session)
        }
        .instrument(span)
        .await
    }
}
