use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::operation::{Emitter, Operation, OperationResult, TurnEffect};
use crate::agents::{Agent, AgentEvent};
use crate::config::Config;
use crate::context_mgmt::{compact_messages, DEFAULT_COMPACTION_THRESHOLD};
use crate::conversation::message::{Message, MessageErrorKind, SystemNotificationType};
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::session::Session;
use goose_providers::model::ModelConfig;

const COMPACTION_THINKING_TEXT: &str = "goose is compacting the conversation...";

const MAX_CONTEXT_ERROR_RETRIES: usize = 2;

pub struct CompactionOperation<'a> {
    agent: &'a Agent,
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
    schedule_id: Option<String>,
    context_limit: usize,
    threshold: f64,
    manages_own_context: bool,
    failed_compact_retries: AtomicUsize,
}

impl<'a> CompactionOperation<'a> {
    pub fn new(
        agent: &'a Agent,
        provider: Arc<dyn Provider>,
        model_config: ModelConfig,
        schedule_id: Option<String>,
    ) -> Self {
        let context_limit = model_config.context_limit();
        let manages_own_context = provider.manages_own_context();
        let threshold = Config::global()
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(DEFAULT_COMPACTION_THRESHOLD);
        Self {
            agent,
            provider,
            model_config,
            schedule_id,
            context_limit,
            threshold,
            manages_own_context,
            failed_compact_retries: AtomicUsize::new(0),
        }
    }

    fn over_threshold(&self, tokens: usize) -> bool {
        if self.threshold <= 0.0 || self.threshold >= 1.0 {
            return false;
        }
        (tokens as f64 / self.context_limit as f64) > self.threshold
    }
}

#[async_trait]
impl Operation for CompactionOperation<'_> {
    fn name(&self) -> &'static str {
        "compaction"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        if self.manages_own_context {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let last = conversation.last();

        if last.is_some_and(|m| m.role == rmcp::model::Role::Assistant && m.error_kind().is_none())
        {
            self.failed_compact_retries.store(0, Ordering::Relaxed);
        }

        let reactive_context_error = matches!(
            last.and_then(|m| m.error_kind()),
            Some(MessageErrorKind::ContextLengthExceeded)
        );

        if reactive_context_error {
            if self.failed_compact_retries.load(Ordering::Relaxed) >= MAX_CONTEXT_ERROR_RETRIES {
                return Ok(OperationResult::NotApplicable(emit));
            }
            self.failed_compact_retries.fetch_add(1, Ordering::Relaxed);
        } else {
            let last_is_user = conversation
                .last()
                .map(|m| m.role == rmcp::model::Role::User && !m.is_tool_response())
                .unwrap_or(false);
            if !last_is_user {
                return Ok(OperationResult::NotApplicable(emit));
            }
            match session.usage.total_tokens {
                Some(tokens) if tokens > 0 && self.over_threshold(tokens as usize) => {}
                _ => return Ok(OperationResult::NotApplicable(emit)),
            }
        }

        let trimmed;
        let conversation = if reactive_context_error {
            let mut messages = conversation.messages().to_vec();
            messages.pop();
            trimmed = Conversation::new_unvalidated(messages);
            &trimmed
        } else {
            conversation
        };

        let threshold_percentage = (self.threshold * 100.0) as u32;
        emit.emit(AgentEvent::Message(
            Message::assistant().with_system_notification(
                SystemNotificationType::InlineMessage,
                format!(
                    "Exceeded auto-compact threshold of {threshold_percentage}%. \
                     Performing auto-compaction..."
                ),
            ),
        ))
        .await;
        emit.emit(AgentEvent::Message(
            Message::assistant().with_system_notification(
                SystemNotificationType::ThinkingMessage,
                COMPACTION_THINKING_TEXT,
            ),
        ))
        .await;

        match compact_messages(
            self.provider.as_ref(),
            &self.model_config,
            &session.id,
            conversation,
            false,
        )
        .await
        {
            Ok((compacted, usage)) => {
                self.agent
                    .update_session_metrics(&session.id, self.schedule_id.clone(), &usage, true)
                    .await?;
                emit.emit(AgentEvent::Message(
                    Message::assistant().with_system_notification(
                        SystemNotificationType::InlineMessage,
                        "Compaction complete",
                    ),
                ))
                .await;
                Ok(OperationResult::Applied(vec![compacted.into()]))
            }
            Err(e) => {
                emit.emit(AgentEvent::Message(Message::assistant().with_text(
                    format!(
                        "Ran into this error trying to compact: {e}.\n\n\
                     Please try again or create a new session"
                    ),
                )))
                .await;
                Ok(OperationResult::Applied(vec![TurnEffect::YieldToClient]))
            }
        }
    }
}
