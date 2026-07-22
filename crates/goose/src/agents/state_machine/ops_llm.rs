//! Builds a provider request and streams the next assistant response.

use std::sync::Arc;

use crate::agents::state_machine::operation::{
    applied, messages_since_kickoff, not_applicable, trailing_error, Emitter, Inference,
    InferenceInput, Operation, OperationResult, SlashCommand, TurnEffect, TurnOutcome,
};
use crate::agents::AgentEvent;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::{effective_role, Conversation, EffectiveRole};
use crate::providers::base::Provider;
use crate::session::Session;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;

pub struct InferenceRunner {
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
}

impl InferenceRunner {
    pub fn new(provider: Arc<dyn Provider>, model_config: ModelConfig) -> Self {
        Self {
            provider,
            model_config,
        }
    }

    async fn error_outcome(&self, err: &ProviderError, emit: &Emitter) -> TurnOutcome {
        #[cfg(feature = "telemetry")]
        crate::posthog::emit_error(err.telemetry_type(), &err.to_string());
        tracing::error!("LLM provider error: {err}");
        let message = Message::from_provider_error(err);
        emit.emit(AgentEvent::Message(message.clone())).await;
        vec![message.into()]
    }
}

#[async_trait]
impl Operation for InferenceRunner {
    fn name(&self) -> &'static str {
        "llm"
    }

    async fn run_command(
        &self,
        command: &SlashCommand<'_>,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        if command.command != "status" {
            return not_applicable(emit);
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
        emit.emit(AgentEvent::Message(
            command_message.with_visibility(true, false),
        ))
        .await;
        emit.emit(AgentEvent::Message(response.clone())).await;
        applied([
            TurnEffect::SetMessageVisibility {
                message_id,
                user_visible: true,
                agent_visible: false,
            },
            response.into(),
            TurnEffect::YieldToClient,
        ])
    }
}

#[async_trait]
impl Inference for InferenceRunner {
    async fn infer(
        &self,
        session: &Session,
        conversation: &Conversation,
        input: InferenceInput,
        emit: Emitter,
    ) -> Result<OperationResult> {
        let messages = messages_since_kickoff(conversation)?;
        if trailing_error(conversation).is_some() {
            return not_applicable(emit);
        }

        let answered: std::collections::HashSet<&str> = conversation
            .messages()
            .iter()
            .flat_map(|m| m.get_tool_response_ids())
            .collect();

        let start = conversation.len() - messages.len();
        let messages_for_provider: Vec<_> = conversation
            .messages()
            .iter()
            .enumerate()
            .filter(|(_, m)| m.is_agent_visible())
            .map(|(idx, m)| {
                let mut m = m.agent_visible_content();
                if idx < start {
                    m.content.retain(|c| match c {
                        MessageContent::ToolRequest(request) => {
                            answered.contains(request.id.as_str())
                        }
                        _ => true,
                    });
                }
                m
            })
            .filter(|m| !m.content.is_empty())
            .collect();

        if !messages_for_provider.last().is_some_and(|message| {
            matches!(
                effective_role(message),
                EffectiveRole::User | EffectiveRole::Tool
            )
        }) {
            return not_applicable(emit);
        }

        let context_limit = self
            .provider
            .get_context_limit(&self.model_config)
            .await
            .unwrap_or_else(|_| self.model_config.context_limit());
        let conversation_for_provider = crate::agents::moim::inject_moim_parts(
            Conversation::new_unvalidated(messages_for_provider),
            &session.working_dir,
            Some(context_limit),
            input.moim_parts,
        );

        let stream = crate::agents::reply_parts::stream_response_from_provider(
            self.provider.clone(),
            self.model_config.clone(),
            &session.id,
            &input.system_prompt,
            conversation_for_provider.messages(),
            &input.tools,
            &input.toolshim_tools,
        )
        .await;

        let mut stream = match stream {
            Ok(stream) => stream,
            Err(err) => return applied(self.error_outcome(&err, &emit).await),
        };

        let mut accumulator = Conversation::empty();
        let mut usage_effects = Vec::new();
        loop {
            tokio::select! {
                biased;
                _ = emit.cancelled() => break,
                next = stream.next() => {
                    let Some(result) = next else { break };
                    let (msg_opt, usage_opt) = match result {
                        Ok(chunk) => chunk,
                        Err(err) => {
                            usage_effects.extend(self.error_outcome(&err, &emit).await);
                            return applied(usage_effects);
                        }
                    };
                    if let Some(usage) = usage_opt {
                        usage_effects.push(TurnEffect::RecordUsage {
                            usage,
                            is_compaction: false,
                        });
                    }
                    if let Some(chunk) = msg_opt {
                        emit.emit(AgentEvent::Message(chunk.clone())).await;
                        accumulator.push(chunk);
                    }
                }
            }
        }

        if accumulator.is_empty() {
            return not_applicable(emit);
        }

        usage_effects.extend(accumulator.into_iter().map(Into::into));
        applied(usage_effects)
    }
}
