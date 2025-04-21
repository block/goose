use super::common::get_messages_token_counts;
use crate::message::Message;
use crate::providers::base::Provider;
use crate::token_counter::TokenCounter;
use anyhow::Result;
use std::sync::Arc;

// Constants for the summarization prompt and a follow-up user message.
const SUMMARY_PROMPT: &str = "You are good at summarizing conversations";

/// Summarize the combined messages from the accumulated summary and the current chunk.
///
/// This method builds the summarization request, sends it to the provider, and returns the summarized response.
async fn summarize_combined_messages(
    provider: &Arc<dyn Provider>,
    accumulated_summary: &[Message],
    current_chunk: &[Message],
) -> Result<Vec<Message>, anyhow::Error> {
    // Combine the accumulated summary and current chunk into a single batch.
    let combined_messages: Vec<Message> = accumulated_summary
        .iter()
        .cloned()
        .chain(current_chunk.iter().cloned())
        .collect();

    // Format the batch as a summarization request.
    let request_text = format!(
        "Please summarize the following conversation history, preserving the key points. This summarization will be used for the later conversations.\n\n```\n{:?}\n```",
        combined_messages
    );
    let summarization_request = vec![Message::user().with_text(&request_text)];

    // Send the request to the provider and fetch the response.
    let response = provider
        .complete(SUMMARY_PROMPT, &summarization_request, &[])
        .await?
        .0;

    // Return the summary as the new accumulated summary.
    Ok(vec![response])
}

// Summarization steps:
// 1. Break down large text into smaller chunks (roughly 30% of the model’s context window).
// 2. For each chunk:
//    a. Combine it with the previous summary (or leave blank for the first iteration).
//    b. Summarize the combined text, focusing on extracting only the information we need.
// 3. Generate a final summary using a tailored prompt.
pub async fn summarize_messages(
    provider: Arc<dyn Provider>,
    messages: &Vec<Message>,
    token_counter: &TokenCounter,
    context_limit: usize,
) -> Result<(Vec<Message>, Vec<usize>), anyhow::Error> {
    let chunk_size = context_limit / 3; // 30% of the context window.
    let summary_prompt_tokens = token_counter.count_tokens(SUMMARY_PROMPT);
    let mut accumulated_summary = Vec::new();

    // Get token counts for each message.
    let token_counts = get_messages_token_counts(token_counter, messages);

    // Tokenize and break messages into chunks.
    let mut current_chunk: Vec<Message> = Vec::new();
    let mut current_chunk_tokens = 0;

    for (message, message_tokens) in messages.iter().zip(token_counts.iter()) {
        if current_chunk_tokens + message_tokens > chunk_size - summary_prompt_tokens {
            // Summarize the current chunk with the accumulated summary.
            accumulated_summary =
                summarize_combined_messages(&provider, &accumulated_summary, &current_chunk)
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
            summarize_combined_messages(&provider, &accumulated_summary, &current_chunk).await?;
    }

    Ok((
        accumulated_summary.clone(),
        get_messages_token_counts(token_counter, &accumulated_summary),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Message, MessageContent};
    use crate::model::{ModelConfig, GPT_4O_TOKENIZER};
    use crate::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
    use crate::providers::errors::ProviderError;
    use chrono::Utc;
    use mcp_core::TextContent;
    use mcp_core::{tool::Tool, Role};
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
                Message {
                    role: Role::Assistant,
                    created: Utc::now().timestamp(),
                    content: vec![MessageContent::Text(TextContent {
                        text: "Summarized content".to_string(),
                        annotations: None,
                    })],
                },
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    fn create_mock_provider() -> Arc<dyn Provider> {
        let mock_model_config =
            ModelConfig::new("test-model".to_string()).with_context_limit(200_000.into());
        Arc::new(MockProvider {
            model_config: mock_model_config,
        })
    }

    fn create_test_messages() -> Vec<Message> {
        vec![
            Message {
                role: Role::User,
                created: Utc::now().timestamp(),
                content: vec![MessageContent::Text(TextContent {
                    text: "Message 1".to_string(),
                    annotations: None,
                })],
            },
            Message {
                role: Role::Assistant,
                created: Utc::now().timestamp(),
                content: vec![MessageContent::Text(TextContent {
                    text: "Message 2".to_string(),
                    annotations: None,
                })],
            },
            Message {
                role: Role::User,
                created: Utc::now().timestamp(),
                content: vec![MessageContent::Text(TextContent {
                    text: "Message 3".to_string(),
                    annotations: None,
                })],
            },
        ]
    }

    #[tokio::test]
    async fn test_summarize_messages_single_chunk() {
        let provider = create_mock_provider();
        let token_counter = TokenCounter::new(GPT_4O_TOKENIZER);
        let context_limit = 100; // Set a high enough limit to avoid chunking.
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
            summarized_messages[0].role,
            Role::Assistant,
            "The summarized message should be from the assistant."
        );

        assert_eq!(
            token_counts.len(),
            1,
            "Token counts should match the number of summarized messages."
        );
    }

    #[tokio::test]
    async fn test_summarize_messages_multiple_chunks() {
        let provider = create_mock_provider();
        let token_counter = TokenCounter::new(GPT_4O_TOKENIZER);
        let context_limit = 30;
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
            summarized_messages[0].role,
            Role::Assistant,
            "The summarized message should be from the assistant."
        );

        assert_eq!(
            token_counts.len(),
            1,
            "Token counts should match the number of summarized messages."
        );
    }

    #[tokio::test]
    async fn test_summarize_messages_empty_input() {
        let provider = create_mock_provider();
        let token_counter = TokenCounter::new(GPT_4O_TOKENIZER);
        let context_limit = 100;
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
}
