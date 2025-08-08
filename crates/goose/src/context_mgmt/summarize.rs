use super::common::get_messages_token_counts_async;
use crate::context_mgmt::get_messages_token_counts;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::prompt_template::render_global_file;
use crate::providers::base::Provider;
use crate::token_counter::{AsyncTokenCounter, TokenCounter};
use anyhow::Result;
use rmcp::model::Role;
use serde::Serialize;
use std::sync::Arc;

// Constants for the summarization prompt and a follow-up user message.
const SUMMARY_PROMPT: &str = "You are good at summarizing conversations";

#[derive(Serialize)]
struct SummarizeContext {
    messages: String,
}

/// Summarize the combined messages from the accumulated summary and the current chunk.
///
/// This method builds the summarization request, sends it to the provider, and returns the summarized response.
async fn summarize_combined_messages(
    provider: &Arc<dyn Provider>,
    accumulated_summary: &[Message],
    current_chunk: &[Message],
) -> Result<Conversation, anyhow::Error> {
    // Combine the accumulated summary and current chunk into a single batch.
    let combined_messages = Conversation::new_unvalidated(
        accumulated_summary
            .iter()
            .cloned()
            .chain(current_chunk.iter().cloned())
            .collect::<Vec<_>>(),
    );

    // Format the batch as a summarization request.
    let request_text = format!(
        "Please summarize the following conversation history, preserving the key points. This summarization will be used for the later conversations.\n\n```\n{:?}\n```",
        combined_messages
    );
    let summarization_request = vec![Message::user().with_text(&request_text)];

    // Send the request to the provider and fetch the response.
    let mut response = provider
        .complete(SUMMARY_PROMPT, &summarization_request, &[])
        .await?
        .0;
    // Set role to user as it will be used in following conversation as user content.
    response.role = Role::User;

    // Return the summary as the new accumulated summary.
    Ok(Conversation::new_unvalidated(vec![response]))
}

// Summarization steps:
//    Using a single tailored prompt, summarize the entire conversation history.
pub async fn summarize_messages_oneshot(
    provider: Arc<dyn Provider>,
    messages: &[Message],
    token_counter: &TokenCounter,
    _context_limit: usize,
) -> Result<(Conversation, Vec<usize>), anyhow::Error> {
    if messages.is_empty() {
        // If no messages to summarize, return empty
        return Ok((Conversation::empty(), vec![]));
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

    // Send the request to the provider and fetch the response.
    let mut response = provider
        .complete(&system_prompt, &summarization_request, &[])
        .await?
        .0;

    // Set role to user as it will be used in following conversation as user content.
    response.role = Role::User;

    // Return just the summary without any tool response preservation
    let final_summary = Conversation::new_unvalidated([response].into_iter());
    let counts = get_messages_token_counts(token_counter, final_summary.messages());

    Ok((final_summary, counts))
}

// Summarization steps:
// 1. Break down large text into smaller chunks (roughly 30% of the model’s context window).
// 2. For each chunk:
//    a. Combine it with the previous summary (or leave blank for the first iteration).
//    b. Summarize the combined text, focusing on extracting only the information we need.
// 3. Generate a final summary using a tailored prompt.
pub async fn summarize_messages_chunked(
    provider: Arc<dyn Provider>,
    messages: &[Message],
    token_counter: &TokenCounter,
    context_limit: usize,
) -> Result<(Conversation, Vec<usize>), anyhow::Error> {
    let chunk_size = context_limit / 3; // 33% of the context window.
    let summary_prompt_tokens = token_counter.count_tokens(SUMMARY_PROMPT);
    let mut accumulated_summary = Conversation::empty();

    // Get token counts for each message.
    let token_counts = get_messages_token_counts(token_counter, messages);

    // Tokenize and break messages into chunks.
    let mut current_chunk: Vec<Message> = Vec::new();
    let mut current_chunk_tokens = 0;

    for (message, message_tokens) in messages.iter().zip(token_counts.iter()) {
        if current_chunk_tokens + message_tokens > chunk_size - summary_prompt_tokens {
            // Summarize the current chunk with the accumulated summary.
            accumulated_summary = summarize_combined_messages(
                &provider,
                accumulated_summary.messages(),
                &current_chunk,
            )
            .await?;

            // Reset for the next chunk.
            current_chunk.clear();
            current_chunk_tokens = 0;
        }

        // Add message to the current chunk.
        current_chunk.push(message.clone());
        current_chunk_tokens += message_tokens;
    }

    // Summarize the final chunk if it exists.
    if !current_chunk.is_empty() {
        accumulated_summary =
            summarize_combined_messages(&provider, accumulated_summary.messages(), &current_chunk)
                .await?;
    }

    // Return just the summary without any tool response preservation
    Ok((
        accumulated_summary.clone(),
        get_messages_token_counts(token_counter, accumulated_summary.messages()),
    ))
}

/// Main summarization function that chooses the best algorithm based on context size.
///
/// This function will:
/// 1. First try the one-shot summarization if there's enough context window available
/// 2. Fall back to the chunked approach if the one-shot fails or if context is too limited
/// 3. Choose the algorithm based on absolute token requirements rather than percentages
pub async fn summarize_messages(
    provider: Arc<dyn Provider>,
    messages: &[Message],
    token_counter: &TokenCounter,
    context_limit: usize,
) -> Result<(Conversation, Vec<usize>), anyhow::Error> {
    // Calculate total tokens in messages
    let total_tokens: usize = get_messages_token_counts(token_counter, messages)
        .iter()
        .sum();

    // Calculate absolute token requirements (future-proof for large context models)
    let system_prompt_overhead = 1000; // Conservative estimate for the summarization prompt
    let response_overhead = 4000; // Generous buffer for response generation
    let safety_buffer = 1000; // Small safety margin for tokenization variations
    let total_required = total_tokens + system_prompt_overhead + response_overhead + safety_buffer;

    // Use one-shot if we have enough absolute space (no percentage-based limits)
    if total_required <= context_limit {
        match summarize_messages_oneshot(
            Arc::clone(&provider),
            messages,
            token_counter,
            context_limit,
        )
        .await
        {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Log the error but continue to fallback
                tracing::warn!(
                    "One-shot summarization failed, falling back to chunked approach: {}",
                    e
                );
            }
        }
    }

    // Fall back to the chunked approach
    summarize_messages_chunked(provider, messages, token_counter, context_limit).await
}

/// Async version using AsyncTokenCounter for better performance
pub async fn summarize_messages_async(
    provider: Arc<dyn Provider>,
    messages: &[Message],
    token_counter: &AsyncTokenCounter,
    context_limit: usize,
) -> Result<(Conversation, Vec<usize>), anyhow::Error> {
    let chunk_size = context_limit / 3; // 33% of the context window.
    let summary_prompt_tokens = token_counter.count_tokens(SUMMARY_PROMPT);
    let mut accumulated_summary = Conversation::empty();

    // Get token counts for each message.
    let token_counts = get_messages_token_counts_async(token_counter, messages);

    // Tokenize and break messages into chunks.
    let mut current_chunk = Vec::new();
    let mut current_chunk_tokens = 0;

    for (message, message_tokens) in messages.iter().zip(token_counts.iter()) {
        if current_chunk_tokens + message_tokens > chunk_size - summary_prompt_tokens {
            // Summarize the current chunk with the accumulated summary.
            accumulated_summary = summarize_combined_messages(
                &provider,
                accumulated_summary.messages(),
                &current_chunk,
            )
            .await?;

            // Reset for the next chunk.
            current_chunk.clear();
            current_chunk_tokens = 0;
        }

        // Add message to the current chunk.
        current_chunk.push(message.clone());
        current_chunk_tokens += message_tokens;
    }

    // Summarize the final chunk if it exists.
    if !current_chunk.is_empty() {
        accumulated_summary =
            summarize_combined_messages(&provider, accumulated_summary.messages(), &current_chunk)
                .await?;
    }

    let count = get_messages_token_counts_async(token_counter, accumulated_summary.messages());

    // Return just the summary without any tool response preservation
    Ok((accumulated_summary.clone(), count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{Message, MessageContent};
    use crate::model::ModelConfig;
    use crate::providers::base::{ProviderMetadata, ProviderUsage, Usage};
    use crate::providers::errors::ProviderError;
    use chrono::Utc;
    use rmcp::model::Role;
    use rmcp::model::Tool;
    use rmcp::model::{AnnotateAble, RawTextContent};
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
            Ok((
                Message::new(
                    Role::Assistant,
                    Utc::now().timestamp(),
                    vec![MessageContent::Text(
                        RawTextContent {
                            text: "Summarized content".to_string(),
                        }
                        .no_annotation(),
                    )],
                ),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    fn create_mock_provider() -> Result<Arc<dyn Provider>> {
        let mock_model_config = ModelConfig::new("test-model")?.with_context_limit(200_000.into());

        Ok(Arc::new(MockProvider {
            model_config: mock_model_config,
        }))
    }

    fn create_test_messages() -> Vec<Message> {
        vec![
            set_up_text_message("Message 1", Role::User),
            set_up_text_message("Message 2", Role::Assistant),
            set_up_text_message("Message 3", Role::User),
        ]
    }

    fn set_up_text_message(text: &str, role: Role) -> Message {
        Message::new(role, 0, vec![MessageContent::text(text.to_string())])
    }

    #[tokio::test]
    async fn test_summarize_messages_single_chunk() {
        let provider = create_mock_provider().expect("failed to create mock provider");
        let token_counter = TokenCounter::new();
        let context_limit = 10_000; // Higher limit to avoid underflow
        let messages = create_test_messages();

        let result = summarize_messages(
            Arc::clone(&provider),
            &messages,
            &token_counter,
            context_limit,
        )
        .await;

        assert!(result.is_ok(), "The function should return Ok.");
        let (summarized_messages, token_counts) = result.unwrap();

        assert_eq!(
            summarized_messages.len(),
            1,
            "The summary should contain one message."
        );
        assert_eq!(
            summarized_messages.first().unwrap().role,
            Role::User,
            "The summarized message should be from the user."
        );

        assert_eq!(
            token_counts.len(),
            1,
            "Token counts should match the number of summarized messages."
        );
    }

    #[tokio::test]
    async fn test_summarize_messages_multiple_chunks() {
        let provider = create_mock_provider().expect("failed to create mock provider");
        let token_counter = TokenCounter::new();
        let context_limit = 10_000; // Higher limit to avoid underflow
        let messages = create_test_messages();

        let result = summarize_messages(
            Arc::clone(&provider),
            &messages,
            &token_counter,
            context_limit,
        )
        .await;

        assert!(result.is_ok(), "The function should return Ok.");
        let (summarized_messages, token_counts) = result.unwrap();

        assert_eq!(
            summarized_messages.len(),
            1,
            "There should be one final summarized message."
        );
        assert_eq!(
            summarized_messages.first().unwrap().role,
            Role::User,
            "The summarized message should be from the user."
        );

        assert_eq!(
            token_counts.len(),
            1,
            "Token counts should match the number of summarized messages."
        );
    }

    #[tokio::test]
    async fn test_summarize_messages_empty_input() {
        let provider = create_mock_provider().expect("failed to create mock provider");
        let token_counter = TokenCounter::new();
        let context_limit = 10_000; // Higher limit to avoid underflow
        let messages: Vec<Message> = Vec::new();

        let result = summarize_messages(
            Arc::clone(&provider),
            &messages,
            &token_counter,
            context_limit,
        )
        .await;

        assert!(result.is_ok(), "The function should return Ok.");
        let (summarized_messages, token_counts) = result.unwrap();

        assert_eq!(
            summarized_messages.len(),
            0,
            "The summary should be empty for an empty input."
        );
        assert!(
            token_counts.is_empty(),
            "Token counts should be empty for an empty input."
        );
    }

    #[tokio::test]
    async fn test_summarize_messages_uses_oneshot_for_small_context() {
        let provider = create_mock_provider().expect("failed to create mock provider");
        let token_counter = TokenCounter::new();
        let context_limit = 100_000; // Large context limit
        let messages = create_test_messages(); // Small message set

        let result = summarize_messages(
            Arc::clone(&provider),
            &messages,
            &token_counter,
            context_limit,
        )
        .await;

        assert!(result.is_ok(), "The function should return Ok.");
        let (summarized_messages, _) = result.unwrap();

        // Should use one-shot and return a single summarized message
        assert_eq!(
            summarized_messages.len(),
            1,
            "Should use one-shot summarization for small context."
        );
    }

    #[tokio::test]
    async fn test_summarize_messages_uses_chunked_for_large_context() {
        let provider = create_mock_provider().expect("failed to create mock provider");
        let token_counter = TokenCounter::new();
        let context_limit = 10_000; // Higher limit to avoid underflow
        let messages = create_test_messages();

        let result = summarize_messages(
            Arc::clone(&provider),
            &messages,
            &token_counter,
            context_limit,
        )
        .await;

        assert!(result.is_ok(), "The function should return Ok.");
        let (summarized_messages, _) = result.unwrap();

        // Should fall back to chunked approach
        assert_eq!(
            summarized_messages.len(),
            1,
            "Should use chunked summarization for large context."
        );
    }

    // Mock provider that fails on one-shot but succeeds on chunked
    #[derive(Clone)]
    struct FailingOneshotProvider {
        model_config: ModelConfig,
        call_count: Arc<std::sync::Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl Provider for FailingOneshotProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;

            // Fail if this looks like a one-shot request
            if system.contains("reasoning in `<analysis>` tags") {
                return Err(ProviderError::RateLimitExceeded(
                    "Simulated one-shot failure".to_string(),
                ));
            }

            // Succeed for chunked requests (uses the old SUMMARY_PROMPT)
            Ok((
                Message::new(
                    Role::Assistant,
                    Utc::now().timestamp(),
                    vec![MessageContent::Text(
                        RawTextContent {
                            text: "Chunked summary".to_string(),
                        }
                        .no_annotation(),
                    )],
                ),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    #[tokio::test]
    async fn test_summarize_messages_fallback_on_oneshot_failure() {
        let call_count = Arc::new(std::sync::Mutex::new(0));
        let provider = Arc::new(FailingOneshotProvider {
            model_config: ModelConfig::new("test-model")
                .unwrap()
                .with_context_limit(200_000.into()),
            call_count: Arc::clone(&call_count),
        });
        let token_counter = TokenCounter::new();
        let context_limit = 100_000; // Large enough to try one-shot first
        let messages = create_test_messages();

        let result = summarize_messages(provider, &messages, &token_counter, context_limit).await;

        assert!(
            result.is_ok(),
            "The function should return Ok after fallback."
        );
        let (summarized_messages, _) = result.unwrap();

        // Should have fallen back to chunked approach
        assert_eq!(
            summarized_messages.len(),
            1,
            "Should successfully fall back to chunked approach."
        );

        // Verify the content comes from the chunked approach
        if let MessageContent::Text(text_content) = &summarized_messages.first().unwrap().content[0]
        {
            assert_eq!(text_content.text, "Chunked summary");
        } else {
            panic!("Expected text content");
        }

        // Should have made multiple calls (one-shot attempt + chunked calls)
        let final_count = *call_count.lock().unwrap();
        assert!(
            final_count > 1,
            "Should have made multiple provider calls during fallback"
        );
    }

    #[tokio::test]
    async fn test_summarize_messages_oneshot_direct_call() {
        let provider = create_mock_provider().expect("failed to create mock provider");
        let token_counter = TokenCounter::new();
        let context_limit = 100_000;
        let messages = create_test_messages();

        let result = summarize_messages_oneshot(
            Arc::clone(&provider),
            &messages,
            &token_counter,
            context_limit,
        )
        .await;

        assert!(
            result.is_ok(),
            "One-shot summarization should work directly."
        );
        let (summarized_messages, token_counts) = result.unwrap();

        assert_eq!(
            summarized_messages.len(),
            1,
            "One-shot should return a single summary message."
        );
        assert_eq!(
            summarized_messages.first().unwrap().role,
            Role::User,
            "Summary should be from user role for context."
        );
        assert_eq!(
            token_counts.len(),
            1,
            "Should have token count for the summary."
        );
    }

    #[tokio::test]
    async fn test_summarize_messages_chunked_direct_call() {
        let provider = create_mock_provider().expect("failed to create mock provider");
        let token_counter = TokenCounter::new();
        let context_limit = 10_000; // Higher limit to avoid underflow
        let messages = create_test_messages();

        let result = summarize_messages_chunked(
            Arc::clone(&provider),
            &messages,
            &token_counter,
            context_limit,
        )
        .await;

        assert!(
            result.is_ok(),
            "Chunked summarization should work directly."
        );
        let (summarized_messages, token_counts) = result.unwrap();

        assert_eq!(
            summarized_messages.len(),
            1,
            "Chunked should return a single final summary."
        );
        assert_eq!(
            summarized_messages.first().unwrap().role,
            Role::User,
            "Summary should be from user role for context."
        );
        assert_eq!(
            token_counts.len(),
            1,
            "Should have token count for the summary."
        );
    }

    #[tokio::test]
    async fn test_absolute_token_threshold_calculation() {
        let provider = create_mock_provider().expect("failed to create mock provider");
        let token_counter = TokenCounter::new();

        // Test with a context limit where absolute token calculation matters
        let context_limit = 10_000;
        let system_prompt_overhead = 1000;
        let response_overhead = 4000;
        let safety_buffer = 1000;
        let max_message_tokens =
            context_limit - system_prompt_overhead - response_overhead - safety_buffer; // 4000 tokens

        // Create messages that are just under the absolute threshold
        let mut large_messages = Vec::new();
        let base_message = set_up_text_message("x".repeat(50).as_str(), Role::User);

        // Add enough messages to approach but not exceed the absolute threshold
        let message_tokens = token_counter.count_tokens(&format!("{:?}", base_message));
        let num_messages = (max_message_tokens / message_tokens).saturating_sub(1);

        for i in 0..num_messages {
            large_messages.push(set_up_text_message(&format!("Message {}", i), Role::User));
        }

        let result = summarize_messages(
            Arc::clone(&provider),
            &large_messages,
            &token_counter,
            context_limit,
        )
        .await;

        assert!(
            result.is_ok(),
            "Should handle absolute threshold calculation correctly."
        );
        let (summarized_messages, _) = result.unwrap();
        assert_eq!(summarized_messages.len(), 1, "Should produce a summary.");
    }
}
