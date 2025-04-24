use super::common::get_messages_token_counts;
use crate::message::{Message, MessageContent};
use crate::providers::base::Provider;
use crate::token_counter::TokenCounter;
use anyhow::Result;
use mcp_core::Role;
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
    let mut response = provider
        .complete(SUMMARY_PROMPT, &summarization_request, &[])
        .await?
        .0;
    // Set role to user as it will be used in following conversation as user content.
    response.role = Role::User;

    // Return the summary as the new accumulated summary.
    Ok(vec![response])
}

/// Preprocesses the messages to handle edge cases involving tool responses.
///
/// This function separates messages into two groups:
/// 1. Messages to be summarized (`preprocessed_messages`)
/// 2. Messages to be temporarily removed (`removed_messages`), which include:
///    - The last tool response message.
///    - The corresponding tool request message that immediately precedes the last tool response message (if present).
///
/// The function only considers the last tool response message and its pair for removal.
fn preprocess_messages(messages: &[Message]) -> (Vec<Message>, Vec<Message>) {
    let mut preprocessed_messages = messages.to_owned();
    let mut removed_messages = Vec::new();

    if let Some((last_index, last_message)) = messages.iter().enumerate().rev().find(|(_, m)| {
        m.content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolResponse(_)))
    }) {
        // Check for the corresponding tool request message
        if last_index > 0 {
            if let Some(previous_message) = messages.get(last_index - 1) {
                if previous_message
                    .content
                    .iter()
                    .any(|c| matches!(c, MessageContent::ToolRequest(_)))
                {
                    // Add the tool request message to removed_messages
                    removed_messages.push(previous_message.clone());
                }
            }
        }
        // Add the last tool response message to removed_messages
        removed_messages.push(last_message.clone());

        // Calculate the correct start index for removal
        let start_index = last_index + 1 - removed_messages.len();

        // Remove the tool response and its paired tool request from preprocessed_messages
        preprocessed_messages.drain(start_index..=last_index);
    }

    (preprocessed_messages, removed_messages)
}

/// Reinserts removed messages into the summarized output.
///
/// This function appends messages that were temporarily removed during preprocessing
/// back into the summarized message list. This ensures that important context,
/// such as tool responses, is not lost.
fn reintegrate_removed_messages(
    summarized_messages: &[Message],
    removed_messages: &[Message],
) -> Vec<Message> {
    let mut final_messages = summarized_messages.to_owned();
    final_messages.extend_from_slice(removed_messages);
    final_messages
}

// Summarization steps:
// 1. Break down large text into smaller chunks (roughly 30% of the model’s context window).
// 2. For each chunk:
//    a. Combine it with the previous summary (or leave blank for the first iteration).
//    b. Summarize the combined text, focusing on extracting only the information we need.
// 3. Generate a final summary using a tailored prompt.
pub async fn summarize_messages(
    provider: Arc<dyn Provider>,
    messages: &[Message],
    token_counter: &TokenCounter,
    context_limit: usize,
) -> Result<(Vec<Message>, Vec<usize>), anyhow::Error> {
    let chunk_size = context_limit / 3; // 33% of the context window.
    let summary_prompt_tokens = token_counter.count_tokens(SUMMARY_PROMPT);
    let mut accumulated_summary = Vec::new();

    // Preprocess messages to handle tool response edge case.
    let (preprocessed_messages, removed_messages) = preprocess_messages(messages);

    // Get token counts for each message.
    let token_counts = get_messages_token_counts(token_counter, &preprocessed_messages);

    // Tokenize and break messages into chunks.
    let mut current_chunk: Vec<Message> = Vec::new();
    let mut current_chunk_tokens = 0;

    for (message, message_tokens) in preprocessed_messages.iter().zip(token_counts.iter()) {
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

    // Add back removed messages.
    let final_summary = reintegrate_removed_messages(&accumulated_summary, &removed_messages);

    Ok((
        final_summary.clone(),
        get_messages_token_counts(token_counter, &final_summary),
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
    use mcp_core::{tool::Tool, Role};
    use mcp_core::{Content, TextContent, ToolCall};
    use serde_json::json;
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
            set_up_text_message("Message 1", Role::User),
            set_up_text_message("Message 2", Role::Assistant),
            set_up_text_message("Message 3", Role::User),
        ]
    }

    fn set_up_text_message(text: &str, role: Role) -> Message {
        Message {
            role,
            created: 0,
            content: vec![MessageContent::text(text.to_string())],
        }
    }

    fn set_up_tool_request_message(id: &str, tool_call: ToolCall) -> Message {
        Message {
            role: Role::Assistant,
            created: 0,
            content: vec![MessageContent::tool_request(id.to_string(), Ok(tool_call))],
        }
    }

    fn set_up_tool_response_message(id: &str, tool_response: Vec<Content>) -> Message {
        Message {
            role: Role::User,
            created: 0,
            content: vec![MessageContent::tool_response(
                id.to_string(),
                Ok(tool_response),
            )],
        }
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

    #[tokio::test]
    async fn test_preprocess_messages_without_tool_response() {
        let messages = create_test_messages();
        let (preprocessed_messages, removed_messages) = preprocess_messages(&messages);

        assert_eq!(
            preprocessed_messages.len(),
            3,
            "Only the user message should remain after preprocessing."
        );
        assert_eq!(
            removed_messages.len(),
            0,
            "The tool request and tool response messages should be removed."
        );
    }

    #[tokio::test]
    async fn test_preprocess_messages_with_tool_response() {
        let arguments = json!({
            "param1": "value1"
        });
        let messages = vec![
            set_up_text_message("Message 1", Role::User),
            set_up_tool_request_message("id", ToolCall::new("tool_name", json!(arguments))),
            set_up_tool_response_message("id", vec![Content::text("tool done")]),
        ];

        let (preprocessed_messages, removed_messages) = preprocess_messages(&messages);

        assert_eq!(
            preprocessed_messages.len(),
            1,
            "Only the user message should remain after preprocessing."
        );
        assert_eq!(
            removed_messages.len(),
            2,
            "The tool request and tool response messages should be removed."
        );
    }

    #[tokio::test]
    async fn test_reintegrate_removed_messages() {
        let summarized_messages = vec![Message {
            role: Role::Assistant,
            created: Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: "Summary".to_string(),
                annotations: None,
            })],
        }];
        let arguments = json!({
            "param1": "value1"
        });
        let removed_messages = vec![
            set_up_tool_request_message("id", ToolCall::new("tool_name", json!(arguments))),
            set_up_tool_response_message("id", vec![Content::text("tool done")]),
        ];

        let final_messages = reintegrate_removed_messages(&summarized_messages, &removed_messages);

        assert_eq!(
            final_messages.len(),
            3,
            "The final message list should include the summary and removed messages."
        );
    }
}
