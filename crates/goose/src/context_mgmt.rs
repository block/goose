pub use goose_providers::context_mgmt::{
    compute_tool_call_cutoff, format_message_for_compacting, structured, tool_ids_to_summarize,
    CompactionModel, CompactionResult, CompactionSettings, DEFAULT_COMPACTION_THRESHOLD,
};

use crate::config::Config;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use anyhow::Result;
use async_trait::async_trait;
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use rmcp::model::Tool;
use std::sync::Arc;
use tokio::task::JoinHandle;

fn tool_pair_summarization_enabled() -> bool {
    Config::global()
        .get_param::<bool>("GOOSE_TOOL_PAIR_SUMMARIZATION")
        .unwrap_or(true)
}

/// Routes compaction/summarization completions through the provider's fast
/// model (with fallback) and tags them with the session id.
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

/// Compact messages by summarizing them. See
/// [`goose_providers::context_mgmt::compact_messages`].
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
    goose_providers::context_mgmt::compact_messages(
        &model,
        &compaction_settings(),
        conversation,
        manual_compact,
    )
    .await
}

/// Check if messages exceed the auto-compaction threshold.
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

    goose_providers::context_mgmt::check_if_compaction_needed(
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
    goose_providers::context_mgmt::summarize_tool_call(&model, conversation, tool_id).await
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

    struct OwnedFastModelCompaction {
        provider: Arc<dyn Provider>,
        model_config: ModelConfig,
        session_id: String,
    }

    #[async_trait]
    impl CompactionModel for OwnedFastModelCompaction {
        async fn complete(
            &self,
            system: &str,
            messages: &[Message],
            tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            crate::model_config::complete_fast(
                self.provider.as_ref(),
                &self.model_config,
                &self.session_id,
                system,
                messages,
                tools,
            )
            .await
        }
    }

    let model = Arc::new(OwnedFastModelCompaction {
        provider,
        model_config,
        session_id,
    });

    goose_providers::context_mgmt::maybe_summarize_tool_pairs(
        model,
        conversation,
        cutoff,
        protect_last_n,
    )
}
