use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agents::state_machine::operation::{
    ConversationEffect, Emitter, GooseEffect, Inference, InferenceInput, MachineEffect, Operation,
    OperationFuture, OperationResult, StepResult,
};
use crate::agents::state_machine::usage;
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::session::{Session, SessionManager};

pub trait MachineSession: Send + Sync {
    fn id(&self) -> &str;
    fn conversation(&self) -> Option<&Conversation>;
}

impl MachineSession for Session {
    fn id(&self) -> &str {
        &self.id
    }

    fn conversation(&self) -> Option<&Conversation> {
        self.conversation.as_ref()
    }
}

#[async_trait]
pub trait SessionLoader<S>: Send + Sync {
    async fn load(&self, session_id: &str) -> Result<S>;
}

#[async_trait]
pub trait EffectHandler<S, E>: Send + Sync {
    async fn apply_effects(&self, session: &S, effects: &mut [E], emit: &Emitter) -> Result<()>;
}

pub trait EffectUsage<E>: Send + Sync {
    fn usage(&self, _effect: &E) -> Option<goose_providers::conversation::token_usage::Usage> {
        None
    }
}

pub enum Step<'a, S, E = ConversationEffect> {
    Operation(Arc<dyn Operation<S, E> + 'a>),
    Inference(Arc<dyn Inference<S, E> + 'a>),
}

impl<S, E: Send> Step<'_, S, E> {
    fn operation(&self) -> &dyn Operation<S, E> {
        match self {
            Step::Operation(operation) => operation.as_ref(),
            Step::Inference(inference) => inference.as_ref(),
        }
    }
}

pub struct StateMachine<'a, S, E = ConversationEffect> {
    steps: Vec<Step<'a, S, E>>,
    cancel: CancellationToken,
}

impl<'a, S, E> StateMachine<'a, S, E>
where
    S: MachineSession,
    E: MachineEffect + Send + 'static,
{
    pub fn new(steps: Vec<Step<'a, S, E>>, cancel: CancellationToken) -> Self {
        Self { steps, cancel }
    }

    pub async fn step(&self, session: &S, emit: &Emitter) -> Result<Option<StepResult<E>>> {
        let conversation = session
            .conversation()
            .ok_or_else(|| anyhow!("state-machine session loaded without conversation"))?;

        for step in &self.steps {
            let name = step.operation().name();
            let result = if self.cancel.is_cancelled() {
                OperationResult::NotApplicable
            } else {
                let step_fut: OperationFuture<'_, Result<OperationResult<E>>> = match step {
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
                    for effect in &mut result.effects {
                        effect.ensure_message_ids();
                    }
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

    pub async fn apply<R>(
        &self,
        runtime: &R,
        session: &S,
        result: &mut StepResult<E>,
        emit: &Emitter,
    ) -> Result<()>
    where
        R: EffectHandler<S, E>,
    {
        runtime
            .apply_effects(session, &mut result.effects, emit)
            .await
    }

    pub async fn run<R>(&self, runtime: &R, session_id: &str, emit: &Emitter) -> Result<S>
    where
        R: SessionLoader<S> + EffectHandler<S, E> + EffectUsage<E>,
    {
        let entry_session = runtime.load(session_id).await?;
        if let Some(input) = entry_session
            .conversation()
            .and_then(|conversation| {
                crate::agents::state_machine::operation::messages_since_kickoff(conversation).ok()
            })
            .and_then(|messages| messages.first())
            .map(Message::user_visible_content)
            .map(|message| message.as_concat_text())
            .filter(|text| !text.is_empty())
        {
            tracing::Span::current().record("trace_input", input.as_str());
        }

        let mut turn_usage = goose_providers::conversation::token_usage::Usage::default();
        loop {
            let session = runtime.load(session_id).await?;
            let Some(mut result) = self.step(&session, emit).await? else {
                break;
            };
            for effect in &result.effects {
                if let Some(usage) = runtime.usage(effect) {
                    turn_usage += usage;
                }
            }
            self.apply(runtime, &session, &mut result, emit).await?;
            if result.yield_to_client {
                break;
            }
        }

        let session = runtime.load(session_id).await?;
        let last_assistant_text = session
            .conversation()
            .and_then(|conversation| {
                crate::agents::state_machine::operation::messages_since_kickoff(conversation).ok()
            })
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

#[async_trait]
impl SessionLoader<Session> for SessionManager {
    async fn load(&self, session_id: &str) -> Result<Session> {
        self.get_session(session_id, true).await
    }
}

#[async_trait]
impl EffectHandler<Session, GooseEffect> for SessionManager {
    async fn apply_effects(
        &self,
        session: &Session,
        effects: &mut [GooseEffect],
        emit: &Emitter,
    ) -> Result<()> {
        for effect in effects.iter_mut() {
            effect.ensure_message_ids();
        }
        usage::enrich(session, effects);

        for effect in effects.iter_mut() {
            match effect {
                GooseEffect::Conversation(ConversationEffect::AppendMessage(message)) => {
                    self.add_message(&session.id, message).await?;
                }
                GooseEffect::Conversation(ConversationEffect::ReplaceConversation(
                    conversation,
                )) => {
                    self.replace_conversation(&session.id, conversation).await?;
                    self.update(&session.id)
                        .usage(usage::estimate_context(conversation).await?)
                        .apply()
                        .await?;
                }
                GooseEffect::ReplaceConversation {
                    conversation,
                    usage: replacement_usage,
                } => {
                    if let Some(provider_usage) = replacement_usage {
                        usage::record(self, session, provider_usage, true).await?;
                    }
                    self.replace_conversation(&session.id, conversation).await?;
                    self.update(&session.id)
                        .usage(usage::estimate_context(conversation).await?)
                        .apply()
                        .await?;
                }
                GooseEffect::Conversation(ConversationEffect::PatchToolRequestMeta {
                    tool_call_id,
                    patch,
                }) => {
                    self.update_tool_request_meta(&session.id, tool_call_id, patch.clone())
                        .await?;
                }
                GooseEffect::Conversation(ConversationEffect::SetMessageVisibility {
                    message_id,
                    user_visible,
                    agent_visible,
                }) => {
                    self.update_message_metadata(&session.id, message_id, |mut metadata| {
                        metadata.user_visible = *user_visible;
                        metadata.agent_visible = *agent_visible;
                        metadata
                    })
                    .await?;
                }
                GooseEffect::SetRecipe(recipe) => {
                    self.update(&session.id)
                        .recipe(recipe.as_ref().clone())
                        .apply()
                        .await?;
                }
                GooseEffect::SetExtensionData(extension_data) => {
                    self.update(&session.id)
                        .extension_data(extension_data.clone())
                        .apply()
                        .await?;
                }
                GooseEffect::RecordUsage(provider_usage) => {
                    usage::record(self, session, provider_usage, false).await?;
                }
            }
        }

        for effect in effects {
            match effect {
                GooseEffect::Conversation(ConversationEffect::AppendMessage(message)) => {
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
                GooseEffect::Conversation(ConversationEffect::ReplaceConversation(
                    conversation,
                ))
                | GooseEffect::ReplaceConversation { conversation, .. } => {
                    emit.emit(AgentEvent::HistoryReplaced(conversation.clone()))
                        .await;
                }
                GooseEffect::RecordUsage(usage) => {
                    emit.emit(AgentEvent::Usage(usage.clone())).await
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl EffectUsage<GooseEffect> for SessionManager {
    fn usage(
        &self,
        effect: &GooseEffect,
    ) -> Option<goose_providers::conversation::token_usage::Usage> {
        match effect {
            GooseEffect::RecordUsage(usage)
            | GooseEffect::ReplaceConversation {
                usage: Some(usage), ..
            } => Some(usage.usage),
            _ => None,
        }
    }
}
