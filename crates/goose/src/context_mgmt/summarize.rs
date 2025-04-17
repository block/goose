use crate::message::Message;
use crate::providers::base::Provider;
use crate::token_counter::TokenCounter;
use anyhow::Result;
use std::sync::Arc;

// TODO: remove afterwards
pub async fn summarize_messages(
    _provider: Arc<dyn Provider>,
    _messages: &Vec<Message>,
    _token_counter: &TokenCounter,
    _context_limit: usize,
) -> Result<(Vec<Message>, Vec<usize>), anyhow::Error> {
    let messages = vec![Message::user().with_text(
        "John speaks Bengali & English. Jane has been playing some tennis and golf recently.",
    )];
    let token_counts: Vec<usize> = vec![17];

    Ok((messages, token_counts))
}

// TODO: bring back a version of the synopsis summary
// However, note that original synopsis summary would run every turn. In this case, we know we're
// out of context so we have to chunk/batch them up or sth else if we wanna use LLMs to summarize
// https://github.com/block/goose/blame/92302c386225190c240c3c04ac651683c307276e/src/goose/synopsis/summarize.md
// https://github.com/block/goose/blob/92302c386225190c240c3c04ac651683c307276e/src/goose/synopsis/moderator.py#L82-L96

// Below is the old memory_condense.rs -> i am not sure we wanna keep this
// Also not totally sure why we loop and create summary

// use crate::message::Message;
// use crate::providers::base::Provider;
// use crate::token_counter::TokenCounter;
// use anyhow::{anyhow, Result};
// use std::sync::Arc;
// use tracing::debug;
// use super::common::{estimate_target_context_limit, get_messages_token_counts};

// // Constants for the summarization prompt and a follow-up user message.
// const SUMMARY_PROMPT: &str = "You are good at summarizing.";
// const USER_CHECKIN_PROMPT: &str = "Hello! How are we progressing?";

// /// Builds a summarization request based on a batch of messages.
// ///
// /// Formats the batch as a code block with an instruction to summarize concisely.
// fn build_summarization_request(batch: &[Message]) -> Vec<Message> {
//     let request_text = format!(
//         "Please summarize the following conversation succinctly, preserving the key points.\n\n```\n{:?}\n```",
//         batch
//     );
//     vec![Message::user().with_text(request_text)]
// }

// /// Sends the summarization request to the provider and returns its response.
// ///
// /// This function uses the global summarization prompt.
// async fn fetch_summary(
//     provider: &Arc<dyn Provider>,
//     messages: &[Message],
// ) -> Result<Message, anyhow::Error> {
//     // Call the provider with the summarization prompt and request messages.
//     Ok(provider.complete(SUMMARY_PROMPT, messages, &[]).await?.0)
// }

// /// Iteratively summarizes portions of the conversation until the total tokens fit within the context limit.
// ///
// /// This routine uses a stack to process messages (starting with the oldest) and replaces chunks with a summary.
// pub async fn summarize_messages(
//     provider: Arc<dyn Provider>,
//     messages: &mut Vec<Message>,
//     token_counter: &TokenCounter,
//     context_limit: usize,
// ) -> Result<(), anyhow::Error> {
//     let summary_prompt_tokens = token_counter.count_tokens(SUMMARY_PROMPT);

//     // Reverse the messages and token counts for efficient pop operations (processing from oldest first).
//     let mut msg_stack: Vec<Message> = messages.iter().cloned().rev().collect();
//     let mut token_counts = get_messages_token_counts(token_counter, messages);
//     let mut token_stack: Vec<usize> = token_counts.iter().copied().rev().collect();

//     // Calculate the total tokens currently held in the stack.
//     let mut total_tokens: usize = token_stack.iter().sum();
//     // net_token_change tracks the net change in tokens during each summarization iteration.
//     let mut net_token_change = 1; // non-zero to ensure we enter the loop

//     // Continue summarizing until the token budget is met or no further progress is made.
//     while total_tokens > context_limit && net_token_change > 0 {
//         let mut messages_batch: Vec<Message> = Vec::new();
//         let mut batch_tokens = 0;

//         // Collect messages until the batch (plus the summary prompt) fits within the context limit.
//         while total_tokens > batch_tokens + context_limit
//             && batch_tokens + summary_prompt_tokens <= context_limit
//         {
//             // Pop the oldest message (from the bottom of the original conversation).
//             let msg = msg_stack.pop().expect("Expected message in stack");
//             let count = token_stack.pop().expect("Expected token count in stack");
//             messages_batch.push(msg);
//             batch_tokens += count;
//         }

//         // When possible, force an additional message into the batch to guarantee progress.
//         if !messages_batch.is_empty()
//             && !msg_stack.is_empty()
//             && batch_tokens + summary_prompt_tokens <= context_limit
//         {
//             let msg = msg_stack.pop().expect("Expected message in stack");
//             let count = token_stack.pop().expect("Expected token count in stack");
//             messages_batch.push(msg);
//             batch_tokens += count;
//         }

//         // Start with a negative value of removed tokens.
//         net_token_change = -(batch_tokens as isize);

//         let summarization_request = build_summarization_request(&messages_batch);
//         let summary_response = fetch_summary(&provider, &summarization_request)
//             .await?
//             .as_concat_text();

//         // Create a mini-conversation: the assistant's summary and a follow-up user message.
//         let new_messages = vec![
//             Message::assistant().with_text(&summary_response),
//             Message::user().with_text(USER_CHECKIN_PROMPT),
//         ];
//         let new_messages_tokens =
//             token_counter.count_chat_tokens("", &new_messages, &[]);
//         net_token_change += new_messages_tokens as isize;

//         // Add the new messages (and their token count) to the stack.
//         token_stack.push(new_messages_tokens);
//         msg_stack.extend(new_messages);

//         total_tokens = total_tokens
//             .checked_add_signed(net_token_change)
//             .ok_or(anyhow!("Error updating token total"))?;
//     }

//     if total_tokens <= context_limit {
//         // Restore the original chronological order.
//         *messages = msg_stack.into_iter().rev().collect();
//         *token_counts = token_stack.into_iter().rev().collect();
//         Ok(())
//     } else {
//         Err(anyhow!("Unable to summarize messages within the context limit."))
//     }
// }

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
