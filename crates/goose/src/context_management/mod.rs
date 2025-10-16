use crate::conversation::message::Message;
use crate::conversation::message::MessageMetadata;
use crate::conversation::Conversation;
use crate::prompt_template::render_global_file;
use crate::providers::base::{Provider, ProviderUsage};
use crate::{config::Config, token_counter::create_token_counter};
use anyhow::Result;
use rmcp::model::Role;
use serde::Serialize;
use tracing::{debug, info};

pub const DEFAULT_COMPACTION_THRESHOLD: f64 = 0.8;

const COMPACTION_MESSAGE: &str =
    "The previous message contains a summary that was prepared because a context limit was reached.
Do not mention that you read a summary or that conversation summarization occurred
Just continue the conversation naturally based on the summarized context";

/// Result of auto-compaction check
#[derive(Debug)]
pub struct AutoCompactResult {
    /// Whether compaction was performed
    pub compacted: bool,
    /// The messages after potential compaction
    pub messages: Conversation,
    /// Provider usage from summarization (if compaction occurred)
    /// This contains the actual token counts after compaction
    pub summarization_usage: Option<ProviderUsage>,
}

impl AutoCompactResult {
    fn compacted(conversation: Conversation, summarization_usage: ProviderUsage) -> Self {
        Self {
            compacted: true,
            messages: conversation,
            summarization_usage: Some(summarization_usage),
        }
    }

    fn not_compacted(conversation: Conversation) -> Self {
        Self {
            compacted: false,
            messages: conversation,
            summarization_usage: None,
        }
    }
}

/// Result of checking if compaction is needed
#[derive(Debug)]
pub struct CompactionCheckResult {
    /// Whether compaction is needed
    pub needs_compaction: bool,
    /// Current token count
    pub current_tokens: usize,
    /// Context limit being used
    pub context_limit: usize,
    /// Current usage ratio (0.0 to 1.0)
    pub usage_ratio: f64,
    /// Remaining tokens before compaction threshold
    pub remaining_tokens: usize,
    /// Percentage until compaction threshold (0.0 to 100.0)
    pub percentage_until_compaction: f64,
}

#[derive(Serialize)]
struct SummarizeContext {
    messages: String,
}

/// Check if messages need compaction and compact them if necessary
///
/// This function combines checking and compaction. It first checks if compaction
/// is needed based on the threshold, and if so, performs the compaction by
/// summarizing messages and updating their visibility metadata.
pub async fn check_and_compact_messages(
    provider: &dyn Provider,
    messages_with_user_message: &[Message],
    force_compact: bool,
    preserve_last_user_message: bool,
    threshold_override: Option<f64>,
    session_metadata: Option<&crate::session::Session>,
) -> Result<AutoCompactResult> {
    if !force_compact {
        let check_result = check_compaction_needed(
            provider,
            messages_with_user_message,
            threshold_override,
            session_metadata,
        )
        .await?;

        if !check_result.needs_compaction {
            debug!(
                "No compaction needed (usage: {:.1}% <= {:.1}% threshold)",
                check_result.usage_ratio * 100.0,
                check_result.percentage_until_compaction
            );
            return Ok(AutoCompactResult::not_compacted(
                Conversation::new_unvalidated(messages_with_user_message.to_vec()),
            ));
        }

        info!(
            "Performing message compaction (usage: {:.1}%)",
            check_result.usage_ratio * 100.0
        );
    } else {
        info!("Forcing message compaction due to context limit exceeded");
    }

    let (messages, preserved_user_message) =
        if let Some(last_message) = messages_with_user_message.last() {
            if matches!(last_message.role, rmcp::model::Role::User) {
                (
                    &messages_with_user_message[..messages_with_user_message.len() - 1],
                    Some(last_message.clone()),
                )
            } else if preserve_last_user_message {
                let most_recent_user_message = messages_with_user_message
                    .iter()
                    .rev()
                    .find(|msg| matches!(msg.role, rmcp::model::Role::User))
                    .cloned();
                (messages_with_user_message, most_recent_user_message)
            } else {
                (messages_with_user_message, None)
            }
        } else {
            (messages_with_user_message, None)
        };

    let summary = summarize(provider, messages).await?;

    let (summary_message, summarization_usage) = match summary {
        Some((summary_message, provider_usage)) => (summary_message, provider_usage),
        None => {
            tracing::warn!("Summarization failed. Returning empty messages.");
            return Ok(AutoCompactResult::not_compacted(Conversation::empty()));
        }
    };

    let mut final_messages = Vec::new();

    for msg in messages.iter().cloned() {
        let updated_metadata = msg.metadata.with_agent_invisible();
        let updated_msg = msg.with_metadata(updated_metadata);
        final_messages.push(updated_msg);
    }

    let compaction_marker = Message::assistant()
        .with_conversation_compacted("Conversation compacted and summarized")
        .with_metadata(MessageMetadata::user_only());
    final_messages.push(compaction_marker);

    let summary_msg = summary_message.with_metadata(MessageMetadata::agent_only());
    final_messages.push(summary_msg);

    let assistant_message = Message::assistant()
        .with_text(COMPACTION_MESSAGE)
        .with_metadata(MessageMetadata::agent_only());
    final_messages.push(assistant_message);

    // Add back the preserved user message if it exists
    if let Some(user_message) = preserved_user_message {
        final_messages.push(user_message);
    }

    Ok(AutoCompactResult::compacted(
        Conversation::new_unvalidated(final_messages),
        summarization_usage,
    ))
}

/// Check if messages need compaction without performing the compaction
///
/// This function analyzes the current token usage and returns detailed information
/// about whether compaction is needed and how close we are to the threshold.
/// It prioritizes actual token counts from session metadata when available,
/// falling back to estimated counts if needed.
async fn check_compaction_needed(
    provider: &dyn Provider,
    messages: &[Message],
    threshold_override: Option<f64>,
    session_metadata: Option<&crate::session::Session>,
) -> Result<CompactionCheckResult> {
    // Get threshold from config or use override
    let config = Config::global();
    // TODO(Douwe): check the default here; it seems to reset to 0.3 sometimes
    let threshold = threshold_override.unwrap_or_else(|| {
        config
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(DEFAULT_COMPACTION_THRESHOLD)
    });

    let context_limit = provider.get_model_config().context_limit();

    let (current_tokens, token_source) = match session_metadata.and_then(|m| m.total_tokens) {
        Some(tokens) => (tokens as usize, "session metadata"),
        None => {
            let token_counter = create_token_counter()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create token counter: {}", e))?;

            let token_counts: Vec<_> = messages
                .iter()
                .filter(|m| m.is_agent_visible())
                .map(|msg| token_counter.count_chat_tokens("", std::slice::from_ref(msg), &[]))
                .collect();

            (token_counts.iter().sum(), "estimated")
        }
    };

    let usage_ratio = current_tokens as f64 / context_limit as f64;

    let threshold_tokens = (context_limit as f64 * threshold) as usize;
    let remaining_tokens = threshold_tokens.saturating_sub(current_tokens);

    let percentage_until_compaction = if usage_ratio < threshold {
        (threshold - usage_ratio) * 100.0
    } else {
        0.0
    };

    let needs_compaction = if threshold <= 0.0 || threshold >= 1.0 {
        usage_ratio > DEFAULT_COMPACTION_THRESHOLD
    } else {
        usage_ratio > threshold
    };
    eprintln!("{needs_compaction} {usage_ratio} {threshold}");

    debug!(
        "Compaction check: {} / {} tokens ({:.1}%), threshold: {:.1}%, needs compaction: {}, source: {}",
        current_tokens,
        context_limit,
        usage_ratio * 100.0,
        threshold * 100.0,
        needs_compaction,
        token_source
    );

    Ok(CompactionCheckResult {
        needs_compaction,
        current_tokens,
        context_limit,
        usage_ratio,
        remaining_tokens,
        percentage_until_compaction,
    })
}

async fn summarize(
    provider: &dyn Provider,
    messages: &[Message],
) -> anyhow::Result<Option<(Message, ProviderUsage)>, anyhow::Error> {
    if messages.is_empty() {
        return Ok(None);
    }

    // Format all messages as a single string for the summarization prompt
    let messages_text = messages
        .iter()
        .map(|msg| format!("{:?}", msg))
        .collect::<Vec<_>>()
        .join("\n\n");

    let context = SummarizeContext {
        messages: messages_text,
    };

    // Render the one-shot summarization prompt
    let system_prompt = render_global_file("summarize_oneshot.md", &context)?;

    // Create a simple user message requesting summarization
    let user_message = Message::user()
        .with_text("Please summarize the conversation history provided in the system prompt.");
    let summarization_request = vec![user_message];

    // Send the request to the provider and fetch the response
    let (mut response, mut provider_usage) = provider
        .complete_fast(&system_prompt, &summarization_request, &[])
        .await?;

    // Set role to user as it will be used in following conversation as user content
    response.role = Role::User;

    // Ensure we have token counts, estimating if necessary
    provider_usage
        .ensure_tokens(&system_prompt, &summarization_request, &response, &[])
        .await
        .map_err(|e| anyhow::anyhow!("Failed to ensure usage tokens: {}", e))?;

    Ok(Some((response, provider_usage)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::ModelConfig,
        providers::{
            base::{ProviderMetadata, Usage},
            errors::ProviderError,
        },
    };
    use async_trait::async_trait;
    use rmcp::model::Tool;
    use test_case::test_case;

    struct MockProvider {
        message: Message,
        config: ModelConfig,
    }

    impl MockProvider {
        fn new(message: Message, context_limit: usize) -> Self {
            Self {
                message,
                config: ModelConfig {
                    model_name: "test".to_string(),
                    context_limit: Some(context_limit),
                    temperature: None,
                    max_tokens: None,
                    toolshim: false,
                    toolshim_model: None,
                    fast_model: None,
                },
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::new("mock", "", "", "", vec![""], "", vec![])
        }

        async fn complete_with_model(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                self.message.clone(),
                ProviderUsage::new("mock-model".to_string(), Usage::default()),
            ))
        }

        fn get_model_config(&self) -> ModelConfig {
            self.config.clone()
        }
    }

    fn basic_conversation() -> [Message; 3] {
        [
            Message::user().with_text("hello"),
            Message::assistant().with_text("hello"),
            Message::user().with_text("one more hello"),
        ]
    }

    #[test_case(
        basic_conversation().as_slice(),
        1,
        true
    )]
    #[test_case(
        basic_conversation().as_slice(),
        100,
        false
    )]
    #[tokio::test]
    async fn test_compact(
        original_messages: &[Message],
        token_limit: usize,
        expect_compaction: bool,
    ) {
        let response_message = Message::assistant().with_text("<mock summary>");
        let provider = MockProvider::new(response_message, token_limit);
        let AutoCompactResult {
            compacted,
            messages,
            ..
        } = check_and_compact_messages(&provider, original_messages, false, false, None, None)
            .await
            .unwrap();

        assert_eq!(compacted, expect_compaction);
        if expect_compaction {
            // the original conversation, minus last user message, should be there
            let original_conversation_len = original_messages.len();
            for (lhs, rhs) in original_messages
                .iter()
                .take(original_conversation_len - 1)
                .zip(messages.iter())
            {
                assert_eq!(&lhs.clone().user_only(), rhs);
            }

            assert_eq!(
                messages.messages()[original_conversation_len],
                Message::user().with_text("<mock summary>").agent_only()
            );

            assert_eq!(
                messages.messages()[original_conversation_len + 1],
                // TODO(jack) should this be ConversationCompacted message? a user message?
                // Message::assistant().with_conversation_compacted("The conversation was compacted")
                Message::assistant()
                    .with_text(COMPACTION_MESSAGE)
                    .agent_only()
            );
        } else {
            assert_eq!(original_messages, messages.iter().as_slice());
        }
    }
}
