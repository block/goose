use anyhow::Ok;

use crate::message::Message;
use crate::token_counter::TokenCounter;

use crate::context_mgmt::summarize::summarize_messages;
use crate::context_mgmt::truncate::{truncate_messages, OldestFirstTruncation};
use crate::context_mgmt::{estimate_target_context_limit, get_messages_token_counts};

use super::super::agents::Agent;

// Sample Conversation
// USER: whats your name?

// ---> provider
// ASSISTANT: my name is Goose

// USER: call shell tool

// ---> provider (if error is here, we don't need to include last user msg)
// ASSISTANT - tool call: tool: shell, cmd: ls

// USER - tool result: here's the output:
// 		- file 1
// 		- file 2
// 		- pdf 1

// ---> provider (if error is here, we MUST include last user msg - matching tool result)
// ASSISTANT: i have called the shell tool for you!

impl Agent {
    /// Public API to truncate oldest messages so that the conversation's token count is within the allowed context limit.
    pub async fn truncate_context(
        &self,
        messages: &Vec<Message>, // last message is a user msg that led to assistant message with_context_length_exceeded
    ) -> Result<(Vec<Message>, Vec<usize>), anyhow::Error> {
        let provider = self.provider.clone();
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        let target_context_limit = estimate_target_context_limit(provider);
        let token_counts = get_messages_token_counts(&token_counter, &messages);

        let (mut new_messages, mut new_token_counts) = truncate_messages(
            messages,
            &token_counts,
            target_context_limit,
            &OldestFirstTruncation,
        )?;

        // Add an assistant message to the truncated messages
        // to ensure the assistant's response is included in the context.
        new_messages.push(Message::assistant().with_text("I had run into a context length exceeded error so I truncated some of the oldest messages in our conversation."));
        new_token_counts.push(28);

        Ok((new_messages, new_token_counts))
    }

    /// Public API to summarize the conversation so that its token count is within the allowed context limit.
    pub async fn summarize_context(
        &self,
        messages: &Vec<Message>, // last message is a user msg that led to assistant message with_context_length_exceeded
    ) -> Result<(Vec<Message>, Vec<usize>), anyhow::Error> {
        let provider = self.provider.clone();
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        let target_context_limit = estimate_target_context_limit(provider.clone());

        let (mut new_messages, mut new_token_counts) =
            summarize_messages(provider, messages, &token_counter, target_context_limit).await?;

        // Add an assistant message to the truncated messages
        // to ensure the assistant's response is included in the context.
        new_messages.push(Message::assistant().with_text(
            "I had run into a context length exceeded error so I summarized our conversation.",
        ));
        new_token_counts.push(22);

        Ok((new_messages, new_token_counts))
    }
}
