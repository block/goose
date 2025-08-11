use anyhow::Ok;

use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::token_counter::create_async_token_counter;

use crate::context_mgmt::summarize::summarize_messages_async;
use crate::context_mgmt::truncate::{truncate_messages, OldestFirstTruncation};
use crate::context_mgmt::{estimate_target_context_limit, get_messages_token_counts_async};

use super::super::agents::Agent;

impl Agent {
    /// Public API to truncate oldest messages so that the conversation's token count is within the allowed context limit.
    pub async fn truncate_context(
        &self,
        messages: &[Message], // last message is a user msg that led to assistant message with_context_length_exceeded
    ) -> Result<(Conversation, Vec<usize>), anyhow::Error> {
        let provider = self.provider().await?;
        let token_counter = create_async_token_counter()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create token counter: {}", e))?;
        let target_context_limit = estimate_target_context_limit(provider);
        let token_counts = get_messages_token_counts_async(&token_counter, messages);

        let (mut new_messages, mut new_token_counts) = truncate_messages(
            messages,
            &token_counts,
            target_context_limit,
            &OldestFirstTruncation,
        )?;

        // Only add an assistant message if we have room for it and it won't cause another overflow
        let assistant_message = Message::assistant().with_text("I had run into a context length exceeded error so I truncated some of the oldest messages in our conversation.");
        let assistant_tokens =
            token_counter.count_chat_tokens("", &[assistant_message.clone()], &[]);

        let current_total: usize = new_token_counts.iter().sum();
        if current_total + assistant_tokens <= target_context_limit {
            new_messages.push(assistant_message);
            new_token_counts.push(assistant_tokens);
        } else {
            // If we can't fit the assistant message, at least log what happened
            tracing::warn!("Cannot add truncation notice message due to context limits. Current: {}, Assistant: {}, Limit: {}", 
                          current_total, assistant_tokens, target_context_limit);
        }

        Ok((new_messages, new_token_counts))
    }

    /// Public API to summarize the conversation so that its token count is within the allowed context limit.
    pub async fn summarize_context(
        &self,
        messages: &[Message], // last message is a user msg that led to assistant message with_context_length_exceeded
    ) -> Result<(Conversation, Vec<usize>), anyhow::Error> {
        let provider = self.provider().await?;
        let token_counter = create_async_token_counter()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create token counter: {}", e))?;
        let target_context_limit = estimate_target_context_limit(provider.clone());

        let (mut new_messages, mut new_token_counts) =
            summarize_messages_async(provider, messages, &token_counter, target_context_limit)
                .await?;

        // If the summarized messages only contains one message, it means no tool request and response message in the summarized messages,
        // Add an assistant message to the summarized messages to ensure the assistant's response is included in the context.
        if new_messages.len() == 1 {
            let assistant_message = Message::assistant().with_text(
                "I had run into a context length exceeded error so I summarized our conversation.",
            );
            let assistant_tokens =
                token_counter.count_chat_tokens("", &[assistant_message.clone()], &[]);

            let current_total: usize = new_token_counts.iter().sum();
            if current_total + assistant_tokens <= target_context_limit {
                new_messages.push(assistant_message);
                new_token_counts.push(assistant_tokens);
            } else {
                // If we can't fit the assistant message, at least log what happened
                tracing::warn!("Cannot add summarization notice message due to context limits. Current: {}, Assistant: {}, Limit: {}", 
                              current_total, assistant_tokens, target_context_limit);
            }
        }

        Ok((new_messages, new_token_counts))
    }
}
