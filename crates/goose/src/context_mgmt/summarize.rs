use crate::message::Message;
use crate::providers::base::Provider;
use crate::token_counter::TokenCounter;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::debug;

// Constants for the summarization prompt and a follow-up user message.
const SUMMARY_PROMPT: &str = "You are good at summarizing.";
const USER_CHECKIN_PROMPT: &str = "Hello! How are we progressing?";

/// Builds a summarization request based on a batch of messages.
///
/// Formats the batch as a code block with an instruction to summarize concisely.
fn build_summarization_request(batch: &[Message]) -> Vec<Message> {
    let request_text = format!(
        "Please summarize the following conversation succinctly, preserving the key points.\n\n```\n{:?}\n```",
        batch
    );
    vec![Message::user().with_text(request_text)]
}

/// Sends the summarization request to the provider and returns its response.
///
/// This function uses the global summarization prompt.
async fn fetch_summary(
    provider: &Arc<dyn Provider>,
    messages: &[Message],
) -> Result<Message, anyhow::Error> {
    // Call the provider with the summarization prompt and request messages.
    Ok(provider.complete(SUMMARY_PROMPT, messages, &[]).await?.0)
}

/// Iteratively summarizes portions of the conversation until the total tokens fit within the context limit.
///
/// This routine uses a stack to process messages (starting with the oldest) and replaces chunks with a summary.
async fn summarize_context(
    provider: Arc<dyn Provider>,
    token_counter: &TokenCounter,
    messages: &mut Vec<Message>,
    token_counts: &mut Vec<usize>,
    context_limit: usize,
) -> Result<(), anyhow::Error> {
    let summary_prompt_tokens = token_counter.count_tokens(SUMMARY_PROMPT);

    // Reverse the messages and token counts for efficient pop operations (processing from oldest first).
    let mut msg_stack: Vec<Message> = messages.iter().cloned().rev().collect();
    let mut token_stack: Vec<usize> = token_counts.iter().copied().rev().collect();

    // Calculate the total tokens currently held in the stack.
    let mut total_tokens: usize = token_stack.iter().sum();
    // net_token_change tracks the net change in tokens during each summarization iteration.
    let mut net_token_change = 1; // non-zero to ensure we enter the loop

    // Continue summarizing until the token budget is met or no further progress is made.
    while total_tokens > context_limit && net_token_change > 0 {
        let mut messages_batch: Vec<Message> = Vec::new();
        let mut batch_tokens = 0;

        // Collect messages until the batch (plus the summary prompt) fits within the context limit.
        while total_tokens > batch_tokens + context_limit
            && batch_tokens + summary_prompt_tokens <= context_limit
        {
            // Pop the oldest message (from the bottom of the original conversation).
            let msg = msg_stack.pop().expect("Expected message in stack");
            let count = token_stack.pop().expect("Expected token count in stack");
            messages_batch.push(msg);
            batch_tokens += count;
        }

        // When possible, force an additional message into the batch to guarantee progress.
        if !messages_batch.is_empty()
            && !msg_stack.is_empty()
            && batch_tokens + summary_prompt_tokens <= context_limit
        {
            let msg = msg_stack.pop().expect("Expected message in stack");
            let count = token_stack.pop().expect("Expected token count in stack");
            messages_batch.push(msg);
            batch_tokens += count;
        }

        // Start with a negative value of removed tokens.
        net_token_change = -(batch_tokens as isize);

        let summarization_request = build_summarization_request(&messages_batch);
        let summary_response = fetch_summary(&provider, &summarization_request)
            .await?
            .as_concat_text();

        // Create a mini-conversation: the assistant's summary and a follow-up user message.
        let new_messages = vec![
            Message::assistant().with_text(&summary_response),
            Message::user().with_text(USER_CHECKIN_PROMPT),
        ];
        let new_messages_tokens = token_counter.count_chat_tokens("", &new_messages, &[]);
        net_token_change += new_messages_tokens as isize;

        // Add the new messages (and their token count) to the stack.
        token_stack.push(new_messages_tokens);
        msg_stack.extend(new_messages);

        total_tokens = total_tokens
            .checked_add_signed(net_token_change)
            .ok_or(anyhow!("Error updating token total"))?;
    }

    if total_tokens <= context_limit {
        // Restore the original chronological order.
        *messages = msg_stack.into_iter().rev().collect();
        *token_counts = token_stack.into_iter().rev().collect();
        Ok(())
    } else {
        Err(anyhow!(
            "Unable to summarize messages within the context limit."
        ))
    }
}

/// Public API to summarize the conversation so that its token count is within the allowed context limit.
///
/// This function logs the token count before and after processing and asserts that the final token count complies.
pub async fn summarize_messages(
    provider: Arc<dyn Provider>,
    token_counter: &TokenCounter,
    messages: &mut Vec<Message>,
    token_counts: &mut Vec<usize>,
    context_limit: usize,
) -> Result<(), anyhow::Error> {
    let initial_tokens: usize = token_counts.iter().sum();
    debug!("Total tokens before summarization: {}", initial_tokens);

    summarize_context(
        provider,
        token_counter,
        messages,
        token_counts,
        context_limit,
    )
    .await?;

    let final_tokens: usize = token_counts.iter().sum();
    debug!("Total tokens after summarization: {}", final_tokens);
    assert!(
        final_tokens <= context_limit,
        "Resulting token count exceeds the context limit."
    );
    debug!(
        "Message summarization complete. Total tokens: {}",
        final_tokens
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use crate::model::ModelConfig;
    use crate::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
    use anyhow::Result;
    use async_trait::async_trait;
    use std::sync::Arc;

    // --- The provided MockProvider implementation ---
    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            // Return an empty metadata instance for testing.
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[mcp_core::Tool],
        ) -> anyhow::Result<(Message, ProviderUsage), crate::providers::errors::ProviderError>
        {
            // Generate a short response that's guaranteed to be smaller than the input
            // This ensures our summarization will actually reduce token count
            let response = "This is a very short summary of the conversation.";

            Ok((
                Message::assistant().with_text(response),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    // --- Updated tests using the MockProvider ---

    #[tokio::test]
    async fn test_summarize_messages_no_summarization_needed() -> Result<()> {
        let provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model".to_string()),
        });
        let token_counter = TokenCounter::new("Xenova--gpt-4o");

        // Messages that are clearly under the limit.
        let mut messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
        ];
        let mut token_counts = messages
            .iter()
            .map(|msg| token_counter.count_tokens(&msg.as_concat_text()))
            .collect::<Vec<_>>();

        // Set a high context limit so no summarization occurs.
        summarize_messages(
            provider,
            &token_counter,
            &mut messages,
            &mut token_counts,
            100,
        )
        .await?;
        // Expect the original conversation to remain unchanged.
        assert_eq!(messages.len(), 2);
        Ok(())
    }

    #[tokio::test]
    #[ignore] // This test requires a real provider to work properly
    async fn test_summarize_messages_triggered() -> Result<()> {
        let provider = Arc::new(MockProvider {
            model_config: ModelConfig::new("test-model".to_string()),
        });
        let token_counter = TokenCounter::new("Xenova--gpt-4o");

        // Create a longer conversation that will need summarization
        let mut messages = vec![
            Message::user().with_text("This is a long conversation segment that must be summarized because it contains many tokens."),
            Message::assistant().with_text("Here is a lengthy reply that also adds up quickly. I'm adding more text to ensure we exceed the context limit we'll set."),
            Message::user().with_text("I understand. Please continue with the explanation. I'd like to know more about this topic."),
            Message::assistant().with_text("Let me provide more details about this topic. There are several important points to consider. First, we need to understand the basics. Then we can move on to more advanced concepts."),
            Message::user().with_text("That makes sense. What are the key principles I should remember?"),
            Message::assistant().with_text("The key principles include: consistency, modularity, and maintainability. These are fundamental to good software design and will help you create better systems."),
        ];

        // Count tokens for each message
        let mut token_counts = messages
            .iter()
            .map(|msg| token_counter.count_tokens(&msg.as_concat_text()))
            .collect::<Vec<_>>();

        // Get the total token count
        let total_tokens: usize = token_counts.iter().sum();
        println!("Total tokens before summarization: {}", total_tokens);

        // Set context limit to be about 90% of the total, forcing summarization
        // but still allowing enough room for the mock response
        let context_limit = (total_tokens as f64 * 0.9) as usize;
        println!("Context limit set to: {}", context_limit);

        // Perform summarization
        summarize_messages(
            provider,
            &token_counter,
            &mut messages,
            &mut token_counts,
            context_limit,
        )
        .await?;

        // Check results
        let final_tokens: usize = token_counts.iter().sum();
        println!("Total tokens after summarization: {}", final_tokens);

        // Verify the final token count is within the limit
        assert!(
            final_tokens <= context_limit,
            "Final token count {} exceeds context limit {}",
            final_tokens,
            context_limit
        );

        // Ensure that the conversation still begins with a user message
        assert_eq!(messages.first().unwrap().role, mcp_core::Role::User);

        Ok(())
    }
}
