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

/// Check if messages need compaction and compact them if necessary
///
/// This function checks the current token usage against a configurable threshold
/// and automatically compacts the messages using the summarization algorithm if needed.
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
    // Get threshold from config or use override
    let config = Config::global();
    let threshold = threshold_override.unwrap_or_else(|| {
        config
            .get_param::<f64>("GOOSE_AUTO_COMPACT_THRESHOLD")
            .unwrap_or(0.3) // Default to 30%
    });

    // Check if auto-compaction is disabled
    if threshold <= 0.0 || threshold >= 1.0 {
        debug!("Auto-compaction disabled (threshold: {})", threshold);
        return Ok(AutoCompactResult {
            compacted: false,
            messages: messages.to_vec(),
            tokens_before: None,
            tokens_after: None,
        });
    }

    // Get provider and token counter
    let provider = agent.provider().await?;
    let token_counter = create_async_token_counter()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create token counter: {}", e))?;

    // Calculate current token usage
    let token_counts = get_messages_token_counts_async(&token_counter, messages);
    let total_tokens: usize = token_counts.iter().sum();
    let context_limit = estimate_target_context_limit(provider);

    // Calculate usage ratio
    let usage_ratio = total_tokens as f64 / context_limit as f64;

    debug!(
        "Context usage: {} / {} ({:.1}%)",
        total_tokens,
        context_limit,
        usage_ratio * 100.0
    );

    // Check if compaction is needed
    if usage_ratio <= threshold {
        debug!(
            "No compaction needed (usage: {:.1}% <= threshold: {:.1}%)",
            usage_ratio * 100.0,
            threshold * 100.0
        );
        return Ok(AutoCompactResult {
            compacted: false,
            messages: messages.to_vec(),
            tokens_before: None,
            tokens_after: None,
        });
    }

    info!(
        "Auto-compacting messages (usage: {:.1}% > threshold: {:.1}%)",
        usage_ratio * 100.0,
        threshold * 100.0
    );

    // Perform compaction
    let (compacted_messages, compacted_token_counts) = agent.summarize_context(messages).await?;
    let tokens_after: usize = compacted_token_counts.iter().sum();

    info!(
        "Compaction complete: {} tokens -> {} tokens ({:.1}% reduction)",
        total_tokens,
        tokens_after,
        (1.0 - (tokens_after as f64 / total_tokens as f64)) * 100.0
    );

    Ok(AutoCompactResult {
        compacted: true,
        messages: compacted_messages,
        tokens_before: Some(total_tokens),
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
    use mcp_core::tool::Tool;
    use rmcp::model::{AnnotateAble, RawTextContent, Role};
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
    async fn test_auto_compact_disabled() {
        let mock_provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model".to_string())
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
            model_config: ModelConfig::new("test-model".to_string())
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
            model_config: ModelConfig::new("test-model".to_string())
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
            model_config: ModelConfig::new("test-model".to_string())
                .with_context_limit(50_000.into()), // Realistic context limit that won't underflow
        });

        let agent = Agent::new();
        let _ = agent.update_provider(mock_provider).await;

        // Create enough messages to trigger compaction with low threshold
        let mut messages = Vec::new();
        // Need to create more messages since we have a 27k usable token limit
        // 10% of 27k = 2.7k tokens
        for i in 0..150 {
            messages.push(create_test_message(&format!(
                "Message {} with enough content to ensure we exceed 10% of the context limit. Adding more content.",
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
