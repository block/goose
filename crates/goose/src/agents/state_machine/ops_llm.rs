//! Builds a provider request and streams the next assistant response.

use std::sync::Arc;

use crate::agents::provider_stream_coordinator::{ProviderStreamCoordinator, ProviderStreamEvent};
use crate::agents::state_machine::operation::{
    applied, messages_since_kickoff, not_applicable, trailing_error, yielded_with, Emitter,
    Inference, InferenceInput, Operation, OperationResult, SlashCommand, StateEffect,
};
use crate::agents::state_machine::ops_unknown_tool::UNCLAIMED_TOOL_ERROR;
use crate::agents::steering::{
    mark_native_steer_delivered, tool_cancellation_response_for_steering,
    was_native_steer_delivered, SteeringQueue,
};
use crate::agents::{AgentEvent, ExtensionManager, PromptManager};
use crate::config::GooseMode;
use crate::conversation::message::{InferenceMetadata, Message, MessageContent, MessageUsage};
use crate::conversation::{effective_role, Conversation, EffectiveRole};
use crate::providers::base::{Provider, ProviderUsage};
use crate::session::{Session, SessionManager};
use crate::tool_inspection::ToolInspectionManager;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use tokio::sync::Mutex;
use tracing_futures::Instrument;

const EMPTY_RESPONSE_MESSAGE: &str =
    "The model returned an empty response. Please resend your message to continue.";
const CANCELLED_TOOL_RESPONSE: &str = "Tool call was cancelled before execution";
const OPERATION_NAME: &str = "llm";

fn is_thinking(content: &MessageContent) -> bool {
    matches!(
        content,
        MessageContent::Thinking(_) | MessageContent::RedactedThinking(_)
    )
}

fn normalize_tool_call_thinking(accumulator: &mut Conversation, chunk: &mut Message) {
    if !chunk
        .content
        .iter()
        .any(|content| matches!(content, MessageContent::ToolRequest(_)))
    {
        return;
    }

    let has_direct_thinking = chunk.content.iter().any(is_thinking);
    let mut prior_thinking = Vec::new();
    for message in accumulator.messages_mut() {
        if message.role != chunk.role
            || message
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::ToolRequest(_)))
        {
            continue;
        }
        prior_thinking.extend(
            message
                .content
                .iter()
                .filter(|content| is_thinking(content))
                .cloned(),
        );
        message.content.retain(|content| !is_thinking(content));
    }
    accumulator
        .messages_mut()
        .retain(|message| !message.content.is_empty());

    if !has_direct_thinking && !prior_thinking.is_empty() {
        if let Some(tool_request) = chunk
            .content
            .iter()
            .position(|content| matches!(content, MessageContent::ToolRequest(_)))
        {
            chunk
                .content
                .splice(tool_request..tool_request, prior_thinking);
        }
    }
}

fn is_empty_provider_response(messages: &Conversation) -> bool {
    !messages
        .iter()
        .any(|message| message.metadata.output_token_limit_reached)
        && messages.iter().all(|message| {
            message.content.iter().all(|content| match content {
                MessageContent::Text(text) => text.text.trim().is_empty(),
                MessageContent::Thinking(thinking) => thinking.thinking.trim().is_empty(),
                _ => false,
            })
        })
}

pub(super) fn chat_span(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
    purpose: &'static str,
) -> tracing::Span {
    tracing::info_span!(
        target: "goose::state_machine",
        "chat",
        "gen_ai.operation.name" = "chat",
        "gen_ai.provider.name" = %provider.get_name(),
        "gen_ai.request.model" = %model_config.model_name,
        "gen_ai.response.model" = tracing::field::Empty,
        "gen_ai.usage.input_tokens" = tracing::field::Empty,
        "gen_ai.usage.output_tokens" = tracing::field::Empty,
        "goose.chat.purpose" = purpose,
        "error.type" = tracing::field::Empty,
        session.id = %session_id,
    )
}

pub(super) fn record_chat_usage(span: &tracing::Span, usage: &ProviderUsage) {
    span.record("gen_ai.response.model", usage.model.as_str());
    if let Some(tokens) = usage.usage.input_tokens {
        span.record("gen_ai.usage.input_tokens", tokens);
    }
    if let Some(tokens) = usage.usage.output_tokens {
        span.record("gen_ai.usage.output_tokens", tokens);
    }
}

pub struct InferenceRunner<'a> {
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
    session_manager: Arc<SessionManager>,
    steering_queue: Arc<SteeringQueue>,
    #[cfg(feature = "code-mode")]
    extension_manager: Arc<ExtensionManager>,
    goose_mode: &'a Mutex<GooseMode>,
    prompt_manager: &'a Mutex<PromptManager>,
    tool_inspection_manager: &'a ToolInspectionManager,
    frontend_instructions: &'a Mutex<Option<String>>,
}

/// The agent-visible conversation as the provider sees it: tool requests left
/// unanswered by an earlier turn are dropped, since nothing will answer them now.
fn messages_for_provider(conversation: &Conversation, turn: &[Message]) -> Vec<Message> {
    let answered: std::collections::HashSet<&str> = conversation
        .messages()
        .iter()
        .flat_map(|message| message.get_tool_response_ids())
        .collect();
    let start = conversation.len() - turn.len();
    conversation
        .messages()
        .iter()
        .enumerate()
        .filter(|(_, message)| message.is_agent_visible())
        .map(|(index, message)| {
            let mut message = message.agent_visible_content();
            if index < start {
                message.content.retain(|content| match content {
                    MessageContent::ToolRequest(request) => answered.contains(request.id.as_str()),
                    _ => true,
                });
            }
            message
        })
        .filter(|message| !message.content.is_empty())
        .collect()
}

fn ends_with_provider_turn(messages: &[Message]) -> bool {
    messages.last().is_some_and(|message| {
        matches!(
            effective_role(message),
            EffectiveRole::User | EffectiveRole::Tool
        )
    })
}

fn ends_with_native_steer(conversation: &Conversation) -> bool {
    conversation
        .iter()
        .rev()
        .find(|message| !message.is_turn_context())
        .is_some_and(was_native_steer_delivered)
}

impl<'a> InferenceRunner<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: Arc<dyn Provider>,
        model_config: ModelConfig,
        session_manager: Arc<SessionManager>,
        steering_queue: Arc<SteeringQueue>,
        #[cfg(feature = "code-mode")] extension_manager: Arc<ExtensionManager>,
        #[cfg(not(feature = "code-mode"))] _extension_manager: Arc<ExtensionManager>,
        goose_mode: &'a Mutex<GooseMode>,
        prompt_manager: &'a Mutex<PromptManager>,
        tool_inspection_manager: &'a ToolInspectionManager,
        frontend_instructions: &'a Mutex<Option<String>>,
    ) -> Self {
        Self {
            provider,
            model_config,
            session_manager,
            steering_queue,
            #[cfg(feature = "code-mode")]
            extension_manager,
            goose_mode,
            prompt_manager,
            tool_inspection_manager,
            frontend_instructions,
        }
    }

    async fn error_outcome(&self, err: &ProviderError, emit: &Emitter) -> Vec<StateEffect> {
        #[cfg(feature = "telemetry")]
        crate::posthog::emit_error(err.telemetry_type(), &err.to_string());
        tracing::Span::current().record("error.type", err.telemetry_type());
        tracing::error!("LLM provider error: {err}");
        let message = Message::from_provider_error(err);
        let message = emit.message(message).await;
        vec![message.into()]
    }

    async fn persist_message_usage(
        &self,
        session: &Session,
        message: &Message,
        usage: &ProviderUsage,
        emit: &Emitter,
    ) -> Result<()> {
        let message_id = message
            .id
            .as_deref()
            .ok_or_else(|| anyhow!("persisted assistant message has no id"))?;
        let usage = crate::agents::state_machine::usage::enrich_provider_usage(session, usage);
        let message_usage = MessageUsage::from_provider_usage(&usage, false);
        let persisted_usage = message_usage.clone();
        self.session_manager
            .update_message_metadata(&session.id, message_id, move |mut metadata| {
                metadata.usage = Some(Box::new(persisted_usage));
                metadata
            })
            .await?;
        if !message.user_visible_content().content.is_empty() {
            emit.emit(AgentEvent::MessageUsage {
                message_id: message.id.clone(),
                usage: message_usage,
            })
            .await;
        }
        Ok(())
    }
}

#[async_trait]
impl Operation for InferenceRunner<'_> {
    fn name(&self) -> &'static str {
        OPERATION_NAME
    }

    async fn cancel(
        &self,
        _session: &Session,
        conversation: &Conversation,
        result: OperationResult,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        let mut answered = conversation
            .messages()
            .iter()
            .flat_map(Message::get_tool_response_ids)
            .map(str::to_string)
            .collect::<std::collections::HashSet<_>>();
        let mut requests = Vec::new();
        let mut request_ids = std::collections::HashSet::new();

        let mut collect = |message: &Message| {
            for content in &message.content {
                match content {
                    MessageContent::ToolRequest(request) => {
                        if request_ids.insert(request.id.clone()) {
                            requests.push(request.clone());
                        }
                    }
                    MessageContent::ToolResponse(response) => {
                        answered.insert(response.id.clone());
                    }
                    _ => {}
                }
            }
        };
        for message in messages_since_kickoff(conversation)? {
            collect(message);
        }
        if let OperationResult::Applied(step) = &result {
            for effect in &step.effects {
                if let StateEffect::AppendMessage(message) = effect {
                    collect(message);
                }
            }
        }

        let mut response = Message::user();
        for request in requests {
            if !answered.contains(&request.id) {
                response.add_tool_response_with_metadata(
                    request.id,
                    Ok(rmcp::model::CallToolResult::error(vec![
                        rmcp::model::ContentBlock::text(CANCELLED_TOOL_RESPONSE),
                    ])),
                    request.metadata.as_ref(),
                );
            }
        }
        if response.get_tool_response_ids().is_empty() {
            return Ok(result);
        }

        let response = emit.message(response).await;
        match result {
            OperationResult::NotApplicable => applied([response.into()]),
            OperationResult::Applied(mut step) => {
                step.effects.push(response.into());
                Ok(OperationResult::Applied(step))
            }
        }
    }

    async fn run_command(
        &self,
        command: &SlashCommand<'_>,
        session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        if command.command != "status" {
            return not_applicable();
        }

        let context_limit = self
            .provider
            .get_context_limit(&self.model_config)
            .await
            .unwrap_or_else(|_| self.model_config.context_limit());
        let context_tokens = session.usage.total_tokens.unwrap_or(0).max(0) as usize;
        let lifetime_tokens = session.accumulated_usage.total_tokens.unwrap_or(0).max(0) as usize;
        let context_pct = if context_limit > 0 {
            let pct = ((context_tokens as f64 / context_limit as f64) * 100.0).round() as usize;
            format!("{}%", pct.min(100))
        } else {
            "N/A".to_string()
        };
        let response = Message::assistant()
            .with_text(format!(
                "**Session status**\n\n\
                 - Model: {}\n\
                 - Provider: {}\n\
                 - Mode: {}\n\
                 - Tokens (lifetime): {}\n\
                 - Context: {} / {} tokens ({})",
                self.model_config.model_name,
                self.provider.get_name(),
                session.goose_mode,
                lifetime_tokens,
                context_tokens,
                context_limit,
                context_pct,
            ))
            .with_visibility(true, false);
        let command_message = messages_since_kickoff(conversation)?
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("status command conversation has no kickoff message"))?;
        let message_id = command_message
            .id
            .clone()
            .ok_or_else(|| anyhow!("Persisted slash command message has no id"))?;
        emit.message(command_message.with_visibility(true, false))
            .await;
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
impl Inference for InferenceRunner<'_> {
    fn applies(&self, conversation: &Conversation) -> bool {
        let Ok(turn) = messages_since_kickoff(conversation) else {
            return false;
        };
        trailing_error(conversation).is_none()
            && !ends_with_native_steer(conversation)
            && ends_with_provider_turn(&messages_for_provider(conversation, turn))
    }

    async fn infer(
        &self,
        session: &Session,
        conversation: &Conversation,
        mut input: InferenceInput,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        let messages = messages_since_kickoff(conversation)?;
        if trailing_error(conversation).is_some() || ends_with_native_steer(conversation) {
            return not_applicable();
        }

        let mut messages_for_provider = messages_for_provider(conversation, messages);
        if !ends_with_provider_turn(&messages_for_provider) {
            return not_applicable();
        }

        let span = chat_span(
            self.provider.as_ref(),
            &self.model_config,
            &session.id,
            "inference",
        );

        async {
            #[cfg(feature = "code-mode")]
            let code_execution_mode = self
                .extension_manager
                .is_extension_enabled(
                    crate::agents::platform_extensions::code_execution::EXTENSION_NAME,
                )
                .await;
            #[cfg(not(feature = "code-mode"))]
            let code_execution_mode = false;

            let goose_mode = *self.goose_mode.lock().await;
            if goose_mode == GooseMode::SmartApprove {
                self.tool_inspection_manager
                    .apply_tool_annotations(&input.tools);
            }
            let tools = crate::agents::reply_parts::prepare_inference_tools(
                input.tools,
                code_execution_mode,
            );
            if let Some(frontend_instructions) = self.frontend_instructions.lock().await.clone() {
                input
                    .prompt_parts
                    .push(("frontend".to_string(), frontend_instructions));
            }
            let system_prompt = self.prompt_manager.lock().await.build_system_prompt(
                &session.working_dir,
                input.prompt_parts,
                goose_mode,
            );
            let (tools, toolshim_tools, system_prompt) =
                crate::agents::reply_parts::prepare_tools_for_provider(
                    tools,
                    system_prompt,
                    &self.model_config,
                );
            let mut available_tools = tools
                .iter()
                .chain(toolshim_tools.iter())
                .map(|tool| tool.name.as_ref())
                .collect::<Vec<_>>();
            available_tools.sort_unstable();
            available_tools.dedup();
            let available_tools = available_tools.join(", ");
            for message in &mut messages_for_provider {
                for content in &mut message.content {
                    let MessageContent::ToolResponse(response) = content else {
                        continue;
                    };
                    let Some(metadata) = &mut response.metadata else {
                        continue;
                    };
                    if metadata.remove(UNCLAIMED_TOOL_ERROR).is_none() {
                        continue;
                    }
                    let Ok(result) = &mut response.tool_result else {
                        continue;
                    };
                    result.content.push(rmcp::model::ContentBlock::text(format!(
                        "Available tools: [{}].",
                        available_tools
                    )));
                }
            }

            let context_limit = self
                .provider
                .get_context_limit(&self.model_config)
                .await
                .unwrap_or_else(|_| self.model_config.context_limit());
            let provider_name = self.provider.get_name();
            if let Some(session_id) =
                super::super::latest_provider_session_id(conversation.messages(), provider_name)
            {
                if let Err(error) = self.provider.resume(session_id).await {
                    tracing::warn!(
                        provider = provider_name,
                        %error,
                        "Could not resume provider session; continuing with a handoff"
                    );
                }
            }
            let turn = messages_since_kickoff(conversation)?;
            let turn_start = turn
                .first()
                .and_then(|message| chrono::DateTime::from_timestamp(message.created, 0))
                .map(|timestamp| timestamp.with_timezone(&chrono::Local))
                .unwrap_or_else(chrono::Local::now);
            // Persist a fresh turn-context event only when its bytes changed
            // (e.g. the turn budget ticked); earlier events stay in place.
            let last_turn_context = turn
                .iter()
                .rev()
                .find(|message| message.is_turn_context())
                .map(Message::as_concat_text);
            let turn_context = crate::agents::moim::turn_context_event(
                &session.working_dir,
                Some(context_limit),
                input.moim_parts,
                turn_start,
            )
            .filter(|event| Some(event.as_concat_text()) != last_turn_context);
            if let Some(event) = turn_context {
                let event = event.with_generated_id_if_missing();
                self.session_manager
                    .add_message(&session.id, &event)
                    .await?;
                messages_for_provider.push(event);
            }
            let conversation_for_provider = Conversation::new_unvalidated(messages_for_provider);
            let mut usage_effects = Vec::new();

            let stream = crate::agents::reply_parts::stream_response_from_provider(
                self.provider.clone(),
                self.model_config.clone(),
                &session.id,
                &system_prompt,
                conversation_for_provider.messages(),
                &tools,
                &toolshim_tools,
            )
            .await;

            let stream = match stream {
                Ok(stream) => stream,
                Err(err) => {
                    usage_effects.extend(self.error_outcome(&err, emit).await);
                    return applied(usage_effects);
                }
            };
            let mut stream = ProviderStreamCoordinator::new(
                stream,
                Arc::clone(&self.provider),
                Arc::clone(&self.steering_queue),
                &session.id,
            );

            let requested_model = self.model_config.model_name.clone();
            let resolved_model = self
                .provider
                .fetch_model_info(&requested_model)
                .await
                .ok()
                .and_then(|model_info| model_info.resolved_model);
            let provider_session_id = self.provider.provider_session_id();
            let inference =
                (resolved_model.is_some() || provider_session_id.is_some()).then(|| {
                    InferenceMetadata {
                        provider: self.provider.get_name().to_string(),
                        requested_model,
                        resolved_model,
                        provider_session_id,
                    }
                });

            let mut accumulator = Conversation::empty();
            let mut tool_request_ids = std::collections::HashSet::new();
            let mut native_steer_delivered = false;
            let mut last_flushed_assistant = None;
            loop {
                let Some(event) = stream.next_event(emit.cancel_token()).await else {
                    break;
                };
                match event {
                    ProviderStreamEvent::ProviderOutput(result) => {
                        let (msg_opt, usage_opt) = match result {
                            Ok(chunk) => chunk,
                            Err(err) => {
                                let has_unflushed_assistant = accumulator.iter().any(|message| {
                                    message.role == rmcp::model::Role::Assistant
                                        && message.error_kind().is_none()
                                });
                                if !has_unflushed_assistant {
                                    if let (Some(message), Some(usage)) = (
                                        last_flushed_assistant.as_ref(),
                                        usage_effects.iter().rev().find_map(
                                            |effect| match effect {
                                                StateEffect::RecordUsage(usage) => Some(usage),
                                                _ => None,
                                            },
                                        ),
                                    ) {
                                        self.persist_message_usage(session, message, usage, emit)
                                            .await?;
                                    }
                                }
                                usage_effects
                                    .extend(accumulator.into_iter().map(StateEffect::from));
                                usage_effects.extend(self.error_outcome(&err, emit).await);
                                return applied(usage_effects);
                            }
                        };
                        if let Some(usage) = usage_opt {
                            let span = tracing::Span::current();
                            record_chat_usage(&span, &usage);
                            usage_effects.push(StateEffect::RecordUsage(usage));
                        }
                        if let Some(mut chunk) = msg_opt {
                            if let Some(inference) = &inference {
                                chunk = chunk.with_inference_if_assistant(inference.clone());
                            }
                            chunk.content.retain(|content| match content {
                                MessageContent::ToolRequest(request) => {
                                    tool_request_ids.insert(request.id.clone())
                                }
                                _ => true,
                            });
                            normalize_tool_call_thinking(&mut accumulator, &mut chunk);
                            if chunk.content.is_empty() {
                                if chunk.metadata.output_token_limit_reached {
                                    chunk = emit.message(chunk).await;
                                }
                                accumulator.push(chunk);
                                continue;
                            }
                            let chunk = emit.message(chunk).await;
                            accumulator.push(chunk);
                        }
                    }
                    ProviderStreamEvent::NativeSteerDelivered(mut message) => {
                        let cancelled_tool_response =
                            tool_cancellation_response_for_steering(&accumulator);
                        if is_empty_provider_response(&accumulator) {
                            accumulator = Conversation::empty();
                        } else {
                            let flushed =
                                std::mem::replace(&mut accumulator, Conversation::empty());
                            for message in flushed {
                                let message = message.with_generated_id_if_missing();
                                if message.role == rmcp::model::Role::Assistant
                                    && message.error_kind().is_none()
                                {
                                    last_flushed_assistant = Some(message.clone());
                                }
                                self.session_manager
                                    .add_message(&session.id, &message)
                                    .await?;
                            }
                        }

                        if let Some(response) = cancelled_tool_response {
                            let response = response.with_generated_id_if_missing();
                            self.session_manager
                                .add_message(&session.id, &response)
                                .await?;
                            emit.emit(AgentEvent::Message(response)).await;
                        }

                        mark_native_steer_delivered(&mut message);
                        let message = message.with_generated_id_if_missing();
                        self.session_manager
                            .add_message(&session.id, &message)
                            .await?;
                        emit.emit(AgentEvent::Message(message)).await;
                        native_steer_delivered = true;
                    }
                }
            }

            let empty_response = is_empty_provider_response(&accumulator);
            if empty_response && !native_steer_delivered {
                let message = Message::assistant().with_text(EMPTY_RESPONSE_MESSAGE);
                let message = emit.message(message).await;
                usage_effects.push(message.into());
                return yielded_with(usage_effects);
            }
            if empty_response {
                accumulator = Conversation::empty();
            }

            let has_recorded_usage = usage_effects
                .iter()
                .any(|effect| matches!(effect, StateEffect::RecordUsage(_)));
            if !has_recorded_usage {
                let mut usage = ProviderUsage::new(
                    self.model_config.model_name.clone(),
                    goose_providers::conversation::token_usage::Usage::default(),
                );
                if let Some(response) = accumulator.last().or(last_flushed_assistant.as_ref()) {
                    crate::providers::usage_estimator::ensure_usage_tokens(
                        &mut usage,
                        &system_prompt,
                        conversation_for_provider.messages(),
                        response,
                        &tools,
                    )
                    .await?;
                    record_chat_usage(&tracing::Span::current(), &usage);
                    usage_effects.push(StateEffect::RecordUsage(usage));
                }
            }

            let has_unflushed_assistant = accumulator.iter().any(|message| {
                message.role == rmcp::model::Role::Assistant && message.error_kind().is_none()
            });
            if !has_unflushed_assistant {
                if let (Some(message), Some(usage)) = (
                    last_flushed_assistant.as_ref(),
                    usage_effects.iter().rev().find_map(|effect| match effect {
                        StateEffect::RecordUsage(usage) => Some(usage),
                        _ => None,
                    }),
                ) {
                    self.persist_message_usage(session, message, usage, emit)
                        .await?;
                }
            }

            usage_effects.extend(accumulator.into_iter().map(Into::into));
            applied(usage_effects)
        }
        .instrument(span)
        .await
    }
}
