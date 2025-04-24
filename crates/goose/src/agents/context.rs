use anyhow::Ok;

use crate::message::Message;
use crate::token_counter::TokenCounter;

use crate::context_mgmt::summarize::summarize_messages;
use crate::context_mgmt::truncate::{truncate_messages, OldestFirstTruncation};
use crate::context_mgmt::{estimate_target_context_limit, get_messages_token_counts};

use super::super::agents::Agent;

impl Agent {
    /// Public API to truncate oldest messages so that the conversation's token count is within the allowed context limit.
    pub async fn truncate_context(
        &self,
        messages: &[Message], // last message is a user msg that led to assistant message with_context_length_exceeded
    ) -> Result<(Vec<Message>, Vec<usize>), anyhow::Error> {
        let provider = self.provider().await?;
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        let target_context_limit = estimate_target_context_limit(provider);
        let token_counts = get_messages_token_counts(&token_counter, messages);

        let (mut new_messages, mut new_token_counts) = truncate_messages(
            messages,
            &token_counts,
            target_context_limit,
            &OldestFirstTruncation,
        )?;

        // Add an assistant message to the truncated messages
        // to ensure the assistant's response is included in the context.
        let assistant_message = Message::assistant().with_text("I had run into a context length exceeded error so I truncated some of the oldest messages in our conversation.");
        new_messages.push(assistant_message.clone());
        new_token_counts.push(token_counter.count_chat_tokens("", &[assistant_message], &[]));

        Ok((new_messages, new_token_counts))
    }

    /// Public API to summarize the conversation so that its token count is within the allowed context limit.
    pub async fn summarize_context(
        &self,
        messages: &[Message], // last message is a user msg that led to assistant message with_context_length_exceeded
    ) -> Result<(Vec<Message>, Vec<usize>), anyhow::Error> {
        let provider = self.provider().await?;
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        let target_context_limit = estimate_target_context_limit(provider.clone());

        let (mut new_messages, mut new_token_counts) =
            summarize_messages(provider, messages, &token_counter, target_context_limit).await?;

        // If the summarized messages only contains one message, it means no tool request and response message in the summarized messages,
        // Add an assistant message to the summarized messages to ensure the assistant's response is included in the context.
        if new_messages.len() == 1 {
            let assistant_message = Message::assistant().with_text(
                "I had run into a context length exceeded error so I summarized our conversation.",
            );
            new_messages.push(assistant_message.clone());
            new_token_counts.push(token_counter.count_chat_tokens("", &[assistant_message], &[]));
        }

        Ok((new_messages, new_token_counts))
    }
}
