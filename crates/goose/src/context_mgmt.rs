pub use goose_compaction::{
    compute_tool_call_cutoff, format_message_for_compacting, structured, tool_ids_to_summarize,
    CompactionModel, CompactionResult, CompactionSettings, DEFAULT_COMPACTION_THRESHOLD,
    TOOLCALL_SUMMARIZATION_BATCH_SIZE,
};

use crate::config::Config;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::token_counter::create_token_counter;
use anyhow::Result;
use async_trait::async_trait;
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use rmcp::model::Tool;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub(crate) fn tool_pair_summarization_enabled() -> bool {
    Config::global()
        .get_param::<bool>("GOOSE_TOOL_PAIR_SUMMARIZATION")
        .unwrap_or(true)
}

/// Routes compaction completions through the provider's fast model (with
/// fallback), tags them with the session id, and counts tokens with the
/// shared tokenizer.
struct FastModelCompaction<'a> {
    provider: &'a dyn Provider,
    model_config: ModelConfig,
    session_id: String,
}

#[async_trait]
impl CompactionModel for FastModelCompaction<'_> {
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        crate::model_config::complete_fast(
            self.provider,
            &self.model_config,
            &self.session_id,
            system,
            messages,
            tools,
        )
        .await
    }

    async fn count_tokens(&self, system: &str, messages: &[Message]) -> Result<usize> {
        let counter = create_token_counter().await.map_err(anyhow::Error::msg)?;
        Ok(counter.count_chat_tokens(system, messages, &[]))
    }
}

/// Estimate the tokens of the agent-visible conversation, counted the same way
/// as the fallback estimation in `check_if_compaction_needed`.
pub(crate) async fn count_context_tokens(conversation: &Conversation) -> Result<i32> {
    let counter = create_token_counter()
        .await
        .map_err(|error| anyhow::anyhow!("Failed to create token counter: {error}"))?;
    let total: usize = conversation
        .messages()
        .iter()
        .filter(|message| message.is_agent_visible())
        .map(|message| counter.count_chat_tokens("", std::slice::from_ref(message), &[]))
        .sum();
    Ok(total.try_into()?)
}

/// Compaction settings honoring user template overrides in
/// `~/.config/goose/prompts/`.
fn compaction_settings() -> CompactionSettings {
    CompactionSettings {
        compaction_prompt_override: crate::prompt_template::user_template_override("compaction.md"),
        summary_template_override: crate::prompt_template::user_template_override(
            "compaction_summary.md",
        ),
    }
}

pub async fn compact_messages(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
    conversation: &Conversation,
    manual_compact: bool,
) -> Result<CompactionResult> {
    let model = FastModelCompaction {
        provider,
        model_config: model_config.clone(),
        session_id: session_id.to_string(),
    };
    goose_compaction::compact_messages(&model, &compaction_settings(), conversation, manual_compact)
        .await
}

pub async fn check_if_compaction_needed(
    provider: &dyn Provider,
    conversation: &Conversation,
    threshold_override: Option<f64>,
    session: &crate::session::Session,
) -> Result<bool> {
    if provider.manages_own_context() {
        return Ok(false);
    }

    let threshold = threshold_override.unwrap_or_else(|| {
        Config::global()
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(DEFAULT_COMPACTION_THRESHOLD)
    });

    let model_config = session
        .model_config
        .clone()
        .unwrap_or_else(|| ModelConfig::new("unknown"));
    let context_limit = provider
        .get_context_limit(&model_config)
        .await
        .unwrap_or_else(|_| model_config.context_limit());

    let model = FastModelCompaction {
        provider,
        model_config,
        session_id: session.id.clone(),
    };

    goose_compaction::check_if_compaction_needed(
        &model,
        conversation,
        context_limit,
        session.usage.total_tokens.map(|t| t as usize),
        threshold,
    )
    .await
}

pub async fn summarize_tool_call(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
    conversation: &Conversation,
    tool_id: &str,
) -> Result<Message> {
    let model = FastModelCompaction {
        provider,
        model_config: model_config.clone(),
        session_id: session_id.to_string(),
    };
    goose_compaction::summarize_tool_call(&model, conversation, tool_id).await
}

pub fn maybe_summarize_tool_pairs(
    provider: Arc<dyn Provider>,
    model_config: ModelConfig,
    session_id: String,
    conversation: Conversation,
    cutoff: usize,
    protect_last_n: usize,
) -> Option<JoinHandle<Vec<(Message, String)>>> {
    if !tool_pair_summarization_enabled() || provider.manages_own_context() {
        return None;
    }

    let tool_ids = tool_ids_to_summarize(&conversation, cutoff, protect_last_n);
    if tool_ids.is_empty() {
        return None;
    }

    Some(tokio::spawn(async move {
        let model = FastModelCompaction {
            provider: provider.as_ref(),
            model_config,
            session_id,
        };
        goose_compaction::summarize_tool_calls(&model, &conversation, tool_ids).await
    }))
}
