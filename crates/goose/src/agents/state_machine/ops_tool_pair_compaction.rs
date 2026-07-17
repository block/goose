use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Role;

use crate::agents::state_machine::operation::{Emitter, Operation, OperationResult, TurnEffect};
use crate::config::Config;
use crate::context_mgmt::{
    compute_tool_call_cutoff, summarize_tool_call, tool_ids_to_summarize,
    tool_pair_summarization_enabled, DEFAULT_COMPACTION_THRESHOLD,
};
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
}

impl ToolPairCompactionOperation {
    pub fn new(provider: Arc<dyn Provider>, model_config: ModelConfig) -> Self {
        let enabled = tool_pair_summarization_enabled() && !provider.manages_own_context();
        let cutoff = Config::global()
            .get_param::<usize>("GOOSE_TOOL_CALL_CUTOFF")
            .unwrap_or_else(|_| {
                let threshold = Config::global()
                    .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
                    .unwrap_or(DEFAULT_COMPACTION_THRESHOLD);
                compute_tool_call_cutoff(model_config.context_limit(), threshold)
            });
        Self {
            provider,
            model_config,
            cutoff,
            enabled,
        }
    }
}

fn trailing_tool_requests(conversation: &Conversation) -> usize {
    let mut count = 0;
    for message in conversation.messages().iter().rev() {
        let is_tool_activity = match message.role {
            Role::Assistant => message.is_tool_call(),
            Role::User => message.is_tool_response() || !message.is_user_visible(),
        };
        if !is_tool_activity {
            break;
        }
        count += message
            .content
            .iter()
            .filter(|c| matches!(c, MessageContent::ToolRequest(_)))
            .count();
    }
    count
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
        if !self.enabled {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let protected = trailing_tool_requests(conversation);
        let tool_ids = tool_ids_to_summarize(conversation, self.cutoff, protected);
        if tool_ids.is_empty() {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let mut effects: Vec<TurnEffect> = Vec::new();
        for tool_id in tool_ids {
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

            for message in pair {
                effects.push(TurnEffect::SetMessageVisibility {
                    message_id: message.id.clone().expect("filtered on id presence"),
                    user_visible: message.is_user_visible(),
                    agent_visible: false,
                });
            }
            effects.push(summary.into());
        }

        if effects.is_empty() {
            Ok(OperationResult::NotApplicable(emit))
        } else {
            Ok(OperationResult::Applied(effects))
        }
    }
}
