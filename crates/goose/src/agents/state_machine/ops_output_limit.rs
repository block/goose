//! One-shot recovery from recoverable output-token length stops.
//!
//! Inference hides a bare output-token-limit marker from the user when the
//! stop is recoverable (output below the intended cap). This operation then
//! compacts the conversation once and lets the turn continue; the marker —
//! and with it the incomplete-response warning — only becomes visible when
//! recovery is not applicable or has already been used.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use goose_agent::operation::{
    OUTPUT_LIMIT_RECOVERY_ATTEMPTED_NOTE, OUTPUT_LIMIT_RECOVERY_OPERATION,
};
use tracing_futures::Instrument;

use crate::agents::state_machine::ops_llm::{chat_span, record_chat_usage};
use crate::agents::state_machine::ops_toolcalling::pending_tool_requests;
use crate::agents::state_machine::{
    applied, messages_since_kickoff, not_applicable, yielded, Emitter, GooseEffect, Operation,
    OperationResult,
};
use crate::context_mgmt::compact_messages;
use crate::conversation::message::{Message, SystemNotificationType};
use crate::providers::base::Provider;
use crate::session::Session;
use goose_providers::model::ModelConfig;

const RECOVERY_THINKING_TEXT: &str = "goose is compacting the conversation...";

pub struct OutputLimitRecoveryOperation {
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
}

impl OutputLimitRecoveryOperation {
    pub fn new(provider: Arc<dyn Provider>, model_config: ModelConfig) -> Self {
        Self {
            provider,
            model_config,
        }
    }
}

#[async_trait]
impl Operation<Session, GooseEffect> for OutputLimitRecoveryOperation {
    fn name(&self) -> &'static str {
        OUTPUT_LIMIT_RECOVERY_OPERATION
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &crate::conversation::Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<GooseEffect>> {
        let messages = messages_since_kickoff(conversation)?;

        let already_recovered = messages.iter().any(|message| {
            message
                .metadata
                .operation_note(
                    OUTPUT_LIMIT_RECOVERY_OPERATION,
                    OUTPUT_LIMIT_RECOVERY_ATTEMPTED_NOTE,
                )
                .is_some()
        });
        if already_recovered {
            return not_applicable();
        }

        let hidden_marker = messages.last().is_some_and(|last| {
            last.role == rmcp::model::Role::Assistant
                && last.metadata.output_token_limit_reached
                && !last.is_user_visible()
        });
        if !hidden_marker || !pending_tool_requests(messages).is_empty() {
            return not_applicable();
        }

        emit.message(Message::assistant().with_system_notification(
            SystemNotificationType::InlineMessage,
            "Output token limit reached early. Compacting to continue conversation...",
        ))
        .await;
        emit.message(Message::assistant().with_system_notification(
            SystemNotificationType::ThinkingMessage,
            RECOVERY_THINKING_TEXT,
        ))
        .await;

        let span = chat_span(
            self.provider.as_ref(),
            &self.model_config,
            &session.id,
            "output_limit_recovery",
        );
        match compact_messages(
            self.provider.as_ref(),
            &self.model_config,
            &session.id,
            conversation,
            false,
        )
        .instrument(span.clone())
        .await
        {
            Ok(result) => {
                let mut compacted = result.conversation;
                let usage = result.usage;
                record_chat_usage(&span, &usage);
                if let Some(last) = compacted.messages_mut().last_mut() {
                    self.set_message_meta(
                        last,
                        OUTPUT_LIMIT_RECOVERY_ATTEMPTED_NOTE,
                        serde_json::Value::Bool(true),
                    );
                }
                emit.message(Message::assistant().with_system_notification(
                    SystemNotificationType::InlineMessage,
                    "Compaction complete",
                ))
                .await;
                applied([GooseEffect::ReplaceConversation {
                    conversation: compacted,
                    usage: Some(usage),
                }])
            }
            Err(e) => {
                span.record("error.type", "compaction_error");
                emit.message(Message::assistant().with_text(format!(
                    "Ran into this error trying to compact: {e}.\n\n\
                     Please try again or create a new session"
                )))
                .await;
                yielded()
            }
        }
    }
}
