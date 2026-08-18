//! Provider inference operation for the unrolled agent loop.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use goose_provider_types::base::{MessageStream, Provider};
use goose_provider_types::conversation::message::{InferenceMetadata, Message, MessageContent};
use goose_provider_types::conversation::token_usage::{ProviderUsage, Usage};
use goose_provider_types::conversation::{effective_role, Conversation, EffectiveRole};
use goose_provider_types::errors::ProviderError;
use goose_provider_types::model::ModelConfig;
use tracing_futures::Instrument;

use crate::operation::{
    applied, messages_since_kickoff, not_applicable, trailing_error, yielded_with,
    ConversationEffect, Emitter, Inference, InferenceInput, Operation, OperationResult,
    SlashCommand,
};

pub struct PreparedInferenceRequest {
    pub system_prompt: String,
    pub tools: Vec<rmcp::model::Tool>,
    pub moim_parts: Vec<String>,
}

#[async_trait]
pub trait InferenceRequestPreparer<S>: Send + Sync {
    async fn prepare(&self, session: &S, input: InferenceInput)
        -> Result<PreparedInferenceRequest>;
}

pub struct IdentityInferenceRequestPreparer;

#[async_trait]
impl<S: Sync> InferenceRequestPreparer<S> for IdentityInferenceRequestPreparer {
    async fn prepare(
        &self,
        _session: &S,
        input: InferenceInput,
    ) -> Result<PreparedInferenceRequest> {
        Ok(PreparedInferenceRequest {
            system_prompt: input
                .prompt_parts
                .into_iter()
                .map(|(_, part)| part)
                .collect::<Vec<_>>()
                .join("\n\n"),
            tools: input.tools,
            moim_parts: input.moim_parts,
        })
    }
}

#[async_trait]
pub trait EffectAdapter<E>: Send + Sync {
    fn effect_from_message(&self, message: Message) -> E;
    fn effect_from_conversation(&self, conversation: Conversation) -> E;
    fn effect_from_conversation_effect(&self, effect: ConversationEffect) -> E;
    fn usage_effect(&self, usage: ProviderUsage) -> E;
    fn appended_message<'a>(&self, effect: &'a E) -> Option<&'a Message>;
}

#[async_trait]
pub trait InferenceHooks<S, E>: EffectAdapter<E> + Send + Sync
where
    E: Send + 'static,
{
    fn session_id<'a>(&self, session: &'a S) -> &'a str;
    fn status(&self, session: &S) -> (String, i64, i64);
    fn unclaimed_tool_error_key(&self) -> &'static str;
    fn latest_provider_session_id<'a>(
        &self,
        conversation: &'a Conversation,
        provider: &str,
    ) -> Option<&'a str>;
    async fn prepare_tools(
        &self,
        tools: Vec<rmcp::model::Tool>,
        system_prompt: String,
        model: &ModelConfig,
    ) -> (Vec<rmcp::model::Tool>, Vec<rmcp::model::Tool>, String);
    #[allow(clippy::too_many_arguments)]
    async fn stream(
        &self,
        provider: Arc<dyn Provider>,
        model: ModelConfig,
        session_id: &str,
        system_prompt: &str,
        messages: &[Message],
        tools: &[rmcp::model::Tool],
        toolshim_tools: &[rmcp::model::Tool],
    ) -> Result<MessageStream, ProviderError>;
    async fn turn_context(
        &self,
        session: &S,
        context_limit: usize,
        moim_parts: Vec<String>,
        conversation: &Conversation,
    ) -> Option<Message>;
    async fn ensure_usage(
        &self,
        usage: &mut ProviderUsage,
        system_prompt: &str,
        messages: &[Message],
        response: &Message,
        tools: &[rmcp::model::Tool],
    ) -> Result<()>;
    fn report_error(&self, error: &ProviderError);
}

const EMPTY_RESPONSE_MESSAGE: &str =
    "The model returned an empty response. Please resend your message to continue.";
const CANCELLED_TOOL_RESPONSE: &str = "Tool call was cancelled before execution";

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

pub fn chat_span(
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

pub fn record_chat_usage(span: &tracing::Span, usage: &ProviderUsage) {
    span.record("gen_ai.response.model", usage.model.as_str());
    if let Some(tokens) = usage.usage.input_tokens {
        span.record("gen_ai.usage.input_tokens", tokens);
    }
    if let Some(tokens) = usage.usage.output_tokens {
        span.record("gen_ai.usage.output_tokens", tokens);
    }
}

pub struct InferenceRunner<'a, S, E> {
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
    request_preparer: Arc<dyn InferenceRequestPreparer<S> + 'a>,
    hooks: Arc<dyn InferenceHooks<S, E> + 'a>,
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

impl<'a, S: Sync, E: Send + 'static> InferenceRunner<'a, S, E> {
    pub fn new(
        provider: Arc<dyn Provider>,
        model_config: ModelConfig,
        request_preparer: Option<Arc<dyn InferenceRequestPreparer<S> + 'a>>,
        hooks: Arc<dyn InferenceHooks<S, E> + 'a>,
    ) -> Self {
        Self {
            provider,
            model_config,
            request_preparer: request_preparer
                .unwrap_or_else(|| Arc::new(IdentityInferenceRequestPreparer)),
            hooks,
        }
    }

    async fn error_outcome(&self, err: &ProviderError, emit: &Emitter) -> Vec<E> {
        self.hooks.report_error(err);
        tracing::Span::current().record("error.type", err.telemetry_type());
        tracing::error!("LLM provider error: {err}");
        let message = Message::from_provider_error(err);
        let message = emit.message(message).await;
        vec![self.hooks.effect_from_message(message)]
    }
}

#[async_trait]
impl<S: Sync, E: Send + 'static> Operation<S, E> for InferenceRunner<'_, S, E> {
    fn name(&self) -> &'static str {
        "llm"
    }

    async fn cancel(
        &self,
        _session: &S,
        conversation: &Conversation,
        result: OperationResult<E>,
        emit: &Emitter,
    ) -> Result<OperationResult<E>> {
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
                if let Some(message) = self.hooks.appended_message(effect) {
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
            OperationResult::NotApplicable => applied([self.hooks.effect_from_message(response)]),
            OperationResult::Applied(mut step) => {
                step.effects.push(self.hooks.effect_from_message(response));
                Ok(OperationResult::Applied(step))
            }
        }
    }

    async fn run_command(
        &self,
        command: &SlashCommand<'_>,
        session: &S,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<E>> {
        if command.command != "status" {
            return not_applicable();
        }

        let context_limit = self
            .provider
            .get_context_limit(&self.model_config)
            .await
            .unwrap_or_else(|_| self.model_config.context_limit());
        let (mode, context_tokens, lifetime_tokens) = self.hooks.status(session);
        let context_tokens = context_tokens.max(0) as usize;
        let lifetime_tokens = lifetime_tokens.max(0) as usize;
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
                mode,
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
            self.hooks
                .effect_from_conversation_effect(ConversationEffect::SetMessageVisibility {
                    message_id,
                    user_visible: true,
                    agent_visible: false,
                }),
            self.hooks.effect_from_message(response),
        ])
    }
}

#[async_trait]
impl<S: Sync, E: Send + 'static> Inference<S, E> for InferenceRunner<'_, S, E> {
    fn applies(&self, conversation: &Conversation) -> bool {
        let Ok(turn) = messages_since_kickoff(conversation) else {
            return false;
        };
        trailing_error(conversation).is_none()
            && ends_with_provider_turn(&messages_for_provider(conversation, turn))
    }

    async fn infer(
        &self,
        session: &S,
        conversation: &Conversation,
        input: InferenceInput,
        emit: &Emitter,
    ) -> Result<OperationResult<E>> {
        let messages = messages_since_kickoff(conversation)?;
        if trailing_error(conversation).is_some() {
            return not_applicable();
        }

        let mut messages_for_provider = messages_for_provider(conversation, messages);
        if !ends_with_provider_turn(&messages_for_provider) {
            return not_applicable();
        }

        let span = chat_span(
            self.provider.as_ref(),
            &self.model_config,
            self.hooks.session_id(session),
            "inference",
        );

        async {
            let prepared = self.request_preparer.prepare(session, input).await?;
            let (tools, toolshim_tools, system_prompt) = self.hooks
                .prepare_tools(prepared.tools, prepared.system_prompt, &self.model_config).await;
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
                    if metadata.remove(self.hooks.unclaimed_tool_error_key()).is_none() {
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
            if let Some(session_id) = self.hooks.latest_provider_session_id(conversation, provider_name) {
                if let Err(error) = self.provider.resume(session_id).await {
                    tracing::warn!(
                        provider = provider_name,
                        %error,
                        "Could not resume provider session; continuing with a handoff"
                    );
                }
            }
            let turn_context = self.hooks
                .turn_context(session, context_limit, prepared.moim_parts, conversation).await;
            if let Some(event) = &turn_context {
                messages_for_provider.push(event.clone());
            }
            let conversation_for_provider = Conversation::new_unvalidated(messages_for_provider);
            let mut usage_effects: Vec<E> = turn_context
                .into_iter().map(|message| self.hooks.effect_from_message(message)).collect();

            let stream = self.hooks.stream(
                self.provider.clone(), self.model_config.clone(), self.hooks.session_id(session),
                &system_prompt, conversation_for_provider.messages(), &tools, &toolshim_tools,
            ).await;

            let mut stream = match stream {
                Ok(stream) => stream,
                Err(err) => {
                    usage_effects.extend(self.error_outcome(&err, emit).await);
                    return applied(usage_effects);
                }
            };

            let requested_model = self.model_config.model_name.clone();
            let resolved_model = self
                .provider
                .fetch_model_info(&requested_model)
                .await
                .ok()
                .and_then(|model_info| model_info.resolved_model);
            let provider_session_id = self.provider.provider_session_id();
            let inference = Some(InferenceMetadata {
                provider: self.provider.get_name().to_string(),
                requested_model,
                resolved_model,
                provider_session_id,
            });

            let mut accumulator = Conversation::empty();
            let mut has_recorded_usage = false;
            let mut tool_request_ids = std::collections::HashSet::new();
            loop {
                tokio::select! {
                    biased;
                    _ = emit.cancelled() => break,
                    next = stream.next() => {
                        let Some(result) = next else { break };
                        let (msg_opt, usage_opt) = match result {
                            Ok(chunk) => chunk,
                            Err(err) => {
                                usage_effects.extend(accumulator.into_iter().map(|message| self.hooks.effect_from_message(message)));
                                usage_effects.extend(self.error_outcome(&err, emit).await);
                                return applied(usage_effects);
                            }
                        };
                        if let Some(usage) = usage_opt {
                            has_recorded_usage = true;
                            let span = tracing::Span::current();
                            record_chat_usage(&span, &usage);
                            usage_effects.push(self.hooks.usage_effect(usage));
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
                }
            }

            let empty_response = !accumulator
                .iter()
                .any(|message| message.metadata.output_token_limit_reached)
                && accumulator.iter().all(|message| {
                    message.content.iter().all(|content| match content {
                        MessageContent::Text(text) => text.text.trim().is_empty(),
                        MessageContent::Thinking(thinking) => thinking.thinking.trim().is_empty(),
                        _ => false,
                    })
                });
            if empty_response {
                let message = Message::assistant().with_text(EMPTY_RESPONSE_MESSAGE);
                let message = emit.message(message).await;
                usage_effects.push(self.hooks.effect_from_message(message));
                return yielded_with(usage_effects);
            }

            if !has_recorded_usage {
                let mut usage = ProviderUsage::new(
                    self.model_config.model_name.clone(),
                    Usage::default(),
                );
                if let Some(response) = accumulator.last() {
                    self.hooks.ensure_usage(&mut usage, &system_prompt,
                        conversation_for_provider.messages(), response, &tools).await?;
                    record_chat_usage(&tracing::Span::current(), &usage);
                    usage_effects.push(self.hooks.usage_effect(usage));
                }
            }

            usage_effects.extend(accumulator.into_iter().map(|message| self.hooks.effect_from_message(message)));
            applied(usage_effects)
        }
        .instrument(span)
        .await
    }
}
