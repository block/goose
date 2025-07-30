use crate::{
    agents::Agent,
    config::Config,
    context_mgmt::{estimate_target_context_limit, get_messages_token_counts_async},
    message::Message,
    token_counter::create_async_token_counter,
};
use anyhow::Result;
use tracing::{debug, info};

/// Result of auto-compaction check
#[derive(Debug)]
pub struct AutoCompactResult {
    /// Whether compaction was performed
    pub compacted: bool,
    /// The messages after potential compaction
    pub messages: Vec<Message>,
    /// Token count before compaction (if compaction occurred)
    pub tokens_before: Option<usize>,
    /// Token count after compaction (if compaction occurred)
    pub tokens_after: Option<usize>,
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

/// Check if messages need compaction without performing the compaction
///
/// This function analyzes the current token usage and returns detailed information
/// about whether compaction is needed and how close we are to the threshold.
///
/// # Arguments
/// * `agent` - The agent to use for context management
/// * `messages` - The current message history
/// * `threshold_override` - Optional threshold override (defaults to GOOSE_AUTO_COMPACT_THRESHOLD config)
///
/// # Returns
/// * `CompactionCheckResult` containing detailed information about compaction needs
pub async fn check_compaction_needed(
    agent: &Agent,
    messages: &[Message],
    threshold_override: Option<f64>,
) -> Result<CompactionCheckResult> {
    // Get threshold from config or use override
    let config = Config::global();
    let threshold = threshold_override.unwrap_or_else(|| {
        config
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(0.3) // Default to 30%
    });

    // Get provider and token counter
    let provider = agent.provider().await?;
    let token_counter = create_async_token_counter()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create token counter: {}", e))?;

    // Calculate current token usage
    let token_counts = get_messages_token_counts_async(&token_counter, messages);
    let current_tokens: usize = token_counts.iter().sum();
    let context_limit = estimate_target_context_limit(provider);

    // Calculate usage ratio
    let usage_ratio = current_tokens as f64 / context_limit as f64;

    // Calculate threshold token count and remaining tokens
    let threshold_tokens = (context_limit as f64 * threshold) as usize;
    let remaining_tokens = threshold_tokens.saturating_sub(current_tokens);

    // Calculate percentage until compaction (how much more we can use before hitting threshold)
    let percentage_until_compaction = if usage_ratio < threshold {
        (threshold - usage_ratio) * 100.0
    } else {
        0.0
    };

    // Check if compaction is needed (disabled if threshold is invalid)
    let needs_compaction = if threshold <= 0.0 || threshold >= 1.0 {
        false
    } else {
        usage_ratio > threshold
    };

    debug!(
        "Compaction check: {} / {} tokens ({:.1}%), threshold: {:.1}%, needs compaction: {}",
        current_tokens,
        context_limit,
        usage_ratio * 100.0,
        threshold * 100.0,
        needs_compaction
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

/// Perform compaction on messages
///
/// This function performs the actual compaction using the agent's summarization
/// capabilities. It assumes compaction is needed and should be called after
/// `check_compaction_needed` confirms it's necessary.
///
/// # Arguments
/// * `agent` - The agent to use for context management
/// * `messages` - The current message history to compact
///
/// # Returns
/// * Tuple of (compacted_messages, tokens_before, tokens_after)
pub async fn perform_compaction(
    agent: &Agent,
    messages: &[Message],
) -> Result<(Vec<Message>, usize, usize)> {
    // Get token counter to measure before/after
    let token_counter = create_async_token_counter()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create token counter: {}", e))?;

    // Calculate tokens before compaction
    let token_counts_before = get_messages_token_counts_async(&token_counter, messages);
    let tokens_before: usize = token_counts_before.iter().sum();

    info!("Performing compaction on {} tokens", tokens_before);

    // Perform compaction
    let (compacted_messages, compacted_token_counts) = agent.summarize_context(messages).await?;
    let tokens_after: usize = compacted_token_counts.iter().sum();

    info!(
        "Compaction complete: {} tokens -> {} tokens ({:.1}% reduction)",
        tokens_before,
        tokens_after,
        (1.0 - (tokens_after as f64 / tokens_before as f64)) * 100.0
    );

    Ok((compacted_messages, tokens_before, tokens_after))
}

/// Check if messages need compaction and compact them if necessary
///
/// This is a convenience wrapper function that combines checking and compaction.
/// If the most recent message is a user message, it will be preserved by removing it
/// before compaction and adding it back afterwards.
///
/// # Arguments
/// * `agent` - The agent to use for context management
/// * `messages` - The current message history
/// * `threshold_override` - Optional threshold override (defaults to GOOSE_AUTO_COMPACT_THRESHOLD config)
///
/// # Returns
/// * `AutoCompactResult` containing the potentially compacted messages and metadata
pub async fn check_and_compact_messages(
    agent: &Agent,
    messages: &[Message],
    threshold_override: Option<f64>,
) -> Result<AutoCompactResult> {
    // First check if compaction is needed
    let check_result = check_compaction_needed(agent, messages, threshold_override).await?;

    // If no compaction is needed, return early
    if !check_result.needs_compaction {
        debug!(
            "No compaction needed (usage: {:.1}% <= {:.1}% threshold)",
            check_result.usage_ratio * 100.0,
            check_result.percentage_until_compaction
        );
        return Ok(AutoCompactResult {
            compacted: false,
            messages: messages.to_vec(),
            tokens_before: None,
            tokens_after: None,
        });
    }

    info!(
        "Auto-compacting messages (usage: {:.1}%)",
        check_result.usage_ratio * 100.0
    );

    // Check if the most recent message is a user message
    let (messages_to_compact, preserved_user_message) = if let Some(last_message) = messages.last()
    {
        if matches!(last_message.role, rmcp::model::Role::User) {
            // Remove the last user message before auto-compaction
            (&messages[..messages.len() - 1], Some(last_message.clone()))
        } else {
            (messages, None)
        }
    } else {
        (messages, None)
    };

    // Perform the compaction on messages excluding the preserved user message
    let (mut compacted_messages, tokens_before, tokens_after) =
        perform_compaction(agent, messages_to_compact).await?;

    // Add back the preserved user message if it exists
    if let Some(user_message) = preserved_user_message {
        compacted_messages.push(user_message);
    }

    Ok(AutoCompactResult {
        compacted: true,
        messages: compacted_messages,
        tokens_before: Some(tokens_before),
        tokens_after: Some(tokens_after),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agents::Agent,
        message::{Message, MessageContent},
        model::ModelConfig,
        providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage},
        providers::errors::ProviderError,
    };
    use chrono::Utc;
    use rmcp::model::{AnnotateAble, RawTextContent, Role, Tool};
    use std::sync::Arc;

    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            // Return a short summary message
            Ok((
                Message::new(
                    Role::Assistant,
                    Utc::now().timestamp(),
                    vec![MessageContent::Text(
                        RawTextContent {
                            text: "Summary of conversation".to_string(),
                        }
                        .no_annotation(),
                    )],
                ),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    fn create_test_message(text: &str) -> Message {
        Message::new(
            Role::User,
            Utc::now().timestamp(),
            vec![MessageContent::text(text.to_string())],
        )
    }

    #[tokio::test]
    async fn test_check_compaction_needed() {
        let mock_provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model")
                .unwrap()
                .with_context_limit(100_000.into()),
        });

        let agent = Agent::new();
        let _ = agent.update_provider(mock_provider).await;

        // Create small messages that won't trigger compaction
        let messages = vec![create_test_message("Hello"), create_test_message("World")];

        let result = check_compaction_needed(&agent, &messages, Some(0.3))
            .await
            .unwrap();

        assert!(!result.needs_compaction);
        assert!(result.current_tokens > 0);
        assert!(result.context_limit > 0);
        assert!(result.usage_ratio < 0.3);
        assert!(result.remaining_tokens > 0);
        assert!(result.percentage_until_compaction > 0.0);
    }

    #[tokio::test]
    async fn test_check_compaction_needed_disabled() {
        let mock_provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model")
                .unwrap()
                .with_context_limit(100_000.into()),
        });

        let agent = Agent::new();
        let _ = agent.update_provider(mock_provider).await;

        let messages = vec![create_test_message("Hello")];

        // Test with threshold 0 (disabled)
        let result = check_compaction_needed(&agent, &messages, Some(0.0))
            .await
            .unwrap();

        assert!(!result.needs_compaction);

        // Test with threshold 1.0 (disabled)
        let result = check_compaction_needed(&agent, &messages, Some(1.0))
            .await
            .unwrap();

        assert!(!result.needs_compaction);
    }

    #[tokio::test]
    async fn test_perform_compaction() {
        let mock_provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model")
                .unwrap()
                .with_context_limit(50_000.into()),
        });

        let agent = Agent::new();
        let _ = agent.update_provider(mock_provider).await;

        // Create some messages to compact
        let messages = vec![
            create_test_message("First message"),
            create_test_message("Second message"),
            create_test_message("Third message"),
        ];

        let (compacted_messages, tokens_before, tokens_after) =
            perform_compaction(&agent, &messages).await.unwrap();

        assert!(tokens_before > 0);
        assert!(tokens_after > 0);
        // Note: The mock provider returns a fixed summary, which might not always be smaller
        // In real usage, compaction should reduce tokens, but for testing we just verify it works
        assert!(!compacted_messages.is_empty());
    }

    #[tokio::test]
    async fn test_auto_compact_disabled() {
        let mock_provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model")
                .unwrap()
                .with_context_limit(10_000.into()),
        });

        let agent = Agent::new();
        let _ = agent.update_provider(mock_provider).await;

        let messages = vec![create_test_message("Hello"), create_test_message("World")];

        // Test with threshold 0 (disabled)
        let result = check_and_compact_messages(&agent, &messages, Some(0.0))
            .await
            .unwrap();

        assert!(!result.compacted);
        assert_eq!(result.messages.len(), messages.len());
        assert!(result.tokens_before.is_none());
        assert!(result.tokens_after.is_none());

        // Test with threshold 1.0 (disabled)
        let result = check_and_compact_messages(&agent, &messages, Some(1.0))
            .await
            .unwrap();

        assert!(!result.compacted);
    }

    #[tokio::test]
    async fn test_auto_compact_below_threshold() {
        let mock_provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model")
                .unwrap()
                .with_context_limit(100_000.into()), // Increased to ensure overhead doesn't dominate
        });

        let agent = Agent::new();
        let _ = agent.update_provider(mock_provider).await;

        // Create small messages that won't trigger compaction
        let messages = vec![create_test_message("Hello"), create_test_message("World")];

        let result = check_and_compact_messages(&agent, &messages, Some(0.3))
            .await
            .unwrap();

        assert!(!result.compacted);
        assert_eq!(result.messages.len(), messages.len());
    }

    #[tokio::test]
    async fn test_auto_compact_above_threshold() {
        let mock_provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model")
                .unwrap()
                .with_context_limit(50_000.into()), // Realistic context limit that won't underflow
        });

        let agent = Agent::new();
        let _ = agent.update_provider(mock_provider).await;

        // Create messages that will exceed 30% of the context limit
        // With 50k context limit, after overhead we have ~27k usable tokens
        // 30% of that is ~8.1k tokens, so we need messages that exceed that
        let mut messages = Vec::new();

        // Create longer messages with more content to reach the threshold
        for i in 0..200 {
            messages.push(create_test_message(&format!(
                "This is message number {} with significantly more content to increase token count. \
                 We need to ensure that our total token usage exceeds 30% of the available context \
                 limit after accounting for system prompt and tools overhead. This message contains \
                 multiple sentences to increase the token count substantially.",
                i
            )));
        }

        let result = check_and_compact_messages(&agent, &messages, Some(0.3))
            .await
            .unwrap();

        assert!(result.compacted);
        assert!(result.tokens_before.is_some());
        assert!(result.tokens_after.is_some());

        // Should have fewer tokens after compaction
        if let (Some(before), Some(after)) = (result.tokens_before, result.tokens_after) {
            assert!(
                after < before,
                "Token count should decrease after compaction"
            );
        }

        // Should have fewer messages (summarized)
        assert!(result.messages.len() <= messages.len());
    }

    #[tokio::test]
    async fn test_auto_compact_respects_config() {
        let mock_provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model")
                .unwrap()
                .with_context_limit(30_000.into()), // Smaller context limit to make threshold easier to hit
        });

        let agent = Agent::new();
        let _ = agent.update_provider(mock_provider).await;

        // Create enough messages to trigger compaction with low threshold
        let mut messages = Vec::new();
        // With 30k context limit, after overhead we have ~27k usable tokens
        // 10% of 27k = 2.7k tokens, so we need messages that exceed that
        for i in 0..200 {
            messages.push(create_test_message(&format!(
                "Message {} with enough content to ensure we exceed 10% of the context limit. \
                 Adding more content to increase token count substantially. This message contains \
                 multiple sentences to increase the token count. We need to ensure that our total \
                 token usage exceeds 10% of the available context limit after accounting for \
                 system prompt and tools overhead.",
                i
            )));
        }

        // Set config value
        let config = Config::global();
        config
            .set_param("GOOSE_AUTO_COMPACT_THRESHOLD", serde_json::Value::from(0.1))
            .unwrap();

        // Should use config value when no override provided
        let result = check_and_compact_messages(&agent, &messages, None)
            .await
            .unwrap();

        // Debug info if not compacted
        if !result.compacted {
            let provider = agent.provider().await.unwrap();
            let token_counter = create_async_token_counter().await.unwrap();
            let token_counts = get_messages_token_counts_async(&token_counter, &messages);
            let total_tokens: usize = token_counts.iter().sum();
            let context_limit = estimate_target_context_limit(provider);
            let usage_ratio = total_tokens as f64 / context_limit as f64;

            eprintln!(
                "Config test not compacted - tokens: {} / {} ({:.1}%)",
                total_tokens,
                context_limit,
                usage_ratio * 100.0
            );
        }

        // With such a low threshold (10%), it should compact
        assert!(result.compacted);

        // Clean up config
        config
            .set_param("GOOSE_AUTO_COMPACT_THRESHOLD", serde_json::Value::from(0.3))
            .unwrap();
    }
}
