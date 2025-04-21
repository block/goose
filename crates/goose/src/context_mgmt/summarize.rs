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

    println!("accumulated_summary: {:?}", accumulated_summary);

    Ok((
        accumulated_summary.clone(),
        get_messages_token_counts(token_counter, &accumulated_summary),
    ))
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::message::Message;
//     use crate::providers::base::{Provider, ProviderMetadata};
//     use crate::token_counter::TokenCounter;
//     use async_trait::async_trait;
//     use anyhow::Result;
//     use std::sync::Arc;

//     // --- Dummy types for testing purposes ---
//     #[derive(Clone, Debug)]
//     struct DummyModelConfig;

//     #[derive(Debug)]
//     struct DummyUsage;
//     impl Default for DummyUsage {
//         fn default() -> Self {
//             DummyUsage
//         }
//     }

//     #[derive(Debug)]
//     struct DummyProviderError;
//     impl std::fmt::Display for DummyProviderError {
//         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//             write!(f, "DummyProviderError")
//         }
//     }
//     impl std::error::Error for DummyProviderError {}

//     // Assume Tool is defined somewhere in your project. For testing, we can use an empty struct.
//     #[derive(Clone, Debug)]
//     struct Tool;

//     // For provider usage, we create a dummy struct with a constructor.
//     #[derive(Debug)]
//     struct DummyProviderUsage;
//     impl DummyProviderUsage {
//         fn new(source: String, _usage: DummyUsage) -> Self {
//             DummyProviderUsage
//         }
//     }
//     // For brevity, we alias the dummy types to those expected by the provider trait.
//     type ProviderUsage = DummyProviderUsage;
//     type ProviderError = DummyProviderError;
//     type ModelConfig = DummyModelConfig;
//     type Usage = DummyUsage;

//     // --- The provided MockProvider implementation ---
//     #[derive(Clone)]
//     struct MockProvider {
//         model_config: ModelConfig,
//     }

//     #[async_trait]
//     impl Provider for MockProvider {
//         fn metadata() -> ProviderMetadata {
//             // Return an empty metadata instance for testing.
//             ProviderMetadata::empty()
//         }

//         fn get_model_config(&self) -> ModelConfig {
//             self.model_config.clone()
//         }

//         async fn complete(
//             &self,
//             _system: &str,
//             _messages: &[Message],
//             _tools: &[Tool],
//         ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
//             Ok((
//                 Message::assistant().with_text("Mock response"),
//                 ProviderUsage::new("mock".to_string(), Usage::default()),
//             ))
//         }
//     }

//     // --- Dummy TokenCounter that uses word counts as a proxy for token counts ---
//     struct DummyTokenCounter;
//     impl TokenCounter for DummyTokenCounter {
//         fn count_tokens(&self, text: &str) -> usize {
//             text.split_whitespace().count()
//         }
//         fn count_chat_tokens(
//             &self,
//             _system_prompt: &str,
//             messages: &[Message],
//             _options: &[&str],
//         ) -> usize {
//             messages.iter().map(|msg| msg.get_text().split_whitespace().count()).sum()
//         }
//     }

//     // --- Updated tests using the MockProvider ---

//     #[tokio::test]
//     async fn test_summarize_messages_no_summarization_needed() -> Result<()> {
//         let provider = Arc::new(MockProvider { model_config: DummyModelConfig });
//         let token_counter = DummyTokenCounter;
//         // Messages that are clearly under the limit.
//         let mut messages = vec![
//             Message::user().with_text("Hello"),
//             Message::assistant().with_text("Hi there!"),
//         ];
//         let mut token_counts = messages
//             .iter()
//             .map(|msg| token_counter.count_tokens(&msg.get_text()))
//             .collect::<Vec<_>>();

//         // Set a high context limit so no summarization occurs.
//         summarize_messages(provider, &token_counter, &mut messages, &mut token_counts, 100).await?;
//         // Expect the original conversation to remain unchanged.
//         assert_eq!(messages.len(), 2);
//         Ok(())
//     }

//     #[tokio::test]
//     async fn test_summarize_messages_triggered() -> Result<()> {
//         let provider = Arc::new(MockProvider { model_config: DummyModelConfig });
//         let token_counter = DummyTokenCounter;
//         // Craft messages whose token counts force the summarization logic.
//         let mut messages = vec![
//             Message::user().with_text("This is a long conversation segment that must be summarized."),
//             Message::assistant().with_text("Here is a lengthy reply that also adds up quickly."),
//         ];
//         let mut token_counts = messages
//             .iter()
//             .map(|msg| token_counter.count_tokens(&msg.get_text()))
//             .collect::<Vec<_>>();

//         // Use a very low context limit to force summarization.
//         summarize_messages(provider, &token_counter, &mut messages, &mut token_counts, 10).await?;
//         // Verify the final token count is within the limit.
//         let final_tokens: usize = token_counts.iter().sum();
//         assert!(final_tokens <= 10);
//         // Ensure that the conversation still begins with a user message.
//         assert_eq!(messages.first().unwrap().role(), "user");
//         Ok(())
//     }
// }
