//! Replaces one batch of old tool request and response pairs with compact summaries.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::operation::{
    applied, messages_since_kickoff, not_applicable, Emitter, Operation, OperationResult,
    TurnEffect,
};
use crate::context_mgmt::{summarize_tool_call, tool_ids_to_summarize};
use crate::conversation::message::MessageContent;
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::session::Session;
use goose_providers::model::ModelConfig;

pub struct ToolPairCompactionOperation {
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
    cutoff: usize,
    enabled: bool,
    batch_attempted: AtomicBool,
}

impl ToolPairCompactionOperation {
    pub fn new(
        provider: Arc<dyn Provider>,
        model_config: ModelConfig,
        cutoff: usize,
        enabled: bool,
    ) -> Self {
        Self {
            provider,
            model_config,
            cutoff,
            enabled,
            batch_attempted: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl Operation for ToolPairCompactionOperation {
    fn name(&self) -> &'static str {
        "tool_pair_compaction"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        if !self.enabled || self.batch_attempted.load(Ordering::Relaxed) {
            return not_applicable(emit);
        }

        let protected = messages_since_kickoff(conversation)?
            .iter()
            .flat_map(|message| &message.content)
            .filter(|content| matches!(content, MessageContent::ToolRequest(_)))
            .count();
        let tool_ids = tool_ids_to_summarize(conversation, self.cutoff, protected);
        if tool_ids.is_empty() {
            return not_applicable(emit);
        }
        self.batch_attempted.store(true, Ordering::Relaxed);

        let mut effects: Vec<TurnEffect> = Vec::new();
        let mut hidden_messages: std::collections::HashSet<String> = Default::default();
        for tool_id in tool_ids {
            let pair: Vec<_> = conversation
                .messages()
                .iter()
                .filter(|message| {
                    message.id.is_some()
                        && message.content.iter().any(|content| match content {
                            MessageContent::ToolRequest(request) => request.id == tool_id,
                            MessageContent::ToolResponse(response) => response.id == tool_id,
                            _ => false,
                        })
                })
                .collect();
            if pair.len() != 2 {
                tracing::warn!(
                    "Expected a tool request/response pair for '{tool_id}', found {} messages",
                    pair.len()
                );
                continue;
            }

            // A message can carry several tool calls (parallel requests, and
            // the execution op batches a turn's responses into one message).
            // Hiding is per message, so the pair is summarized as a group: the
            // summary formats both messages wholesale, covering every sibling
            // call — which then must not be summarized (or hidden) again.
            if pair.iter().any(|message| {
                message
                    .id
                    .as_ref()
                    .is_some_and(|id| hidden_messages.contains(id))
            }) {
                continue;
            }
            let request_ids: std::collections::HashSet<&str> = pair
                .iter()
                .flat_map(|message| message.get_tool_request_ids())
                .collect();
            let response_ids: std::collections::HashSet<&str> = pair
                .iter()
                .flat_map(|message| message.get_tool_response_ids())
                .collect();
            if request_ids != response_ids {
                tracing::warn!(
                    "Tool pair for '{tool_id}' has siblings answered elsewhere; skipping"
                );
                continue;
            }

            let summary = match summarize_tool_call(
                self.provider.as_ref(),
                &self.model_config,
                &session.id,
                conversation,
                &tool_id,
            )
            .await
            {
                Ok(summary) => summary,
                Err(e) => {
                    tracing::warn!("Failed to summarize tool pair: {e}");
                    continue;
                }
            };

            for message in pair {
                let Some(message_id) = message.id.clone() else {
                    continue;
                };
                hidden_messages.insert(message_id.clone());
                effects.push(TurnEffect::SetMessageVisibility {
                    message_id,
                    user_visible: message.is_user_visible(),
                    agent_visible: false,
                });
            }
            effects.push(summary.into());
        }

        if effects.is_empty() {
            not_applicable(emit)
        } else {
            applied(effects)
        }
    }
}
