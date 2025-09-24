use anyhow::Ok;

use crate::conversation::message::{Message, MessageMetadata};
use crate::conversation::Conversation;
use crate::token_counter::create_async_token_counter;

use crate::context_mgmt::summarize::summarize_messages;
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
            token_counter.count_chat_tokens("", std::slice::from_ref(&assistant_message), &[]);

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
    /// Returns the summarized messages, token counts, and the ProviderUsage from summarization
    pub async fn summarize_context(
        &self,
        messages: &[Message], // last message is a user msg that led to assistant message with_context_length_exceeded
    ) -> Result<
        (
            Conversation,
            Vec<usize>,
            Option<crate::providers::base::ProviderUsage>,
        ),
        anyhow::Error,
    > {
        let provider = self.provider().await?;
        let summary_result = summarize_messages(provider.clone(), messages).await?;

        let (summary_message, summarization_usage) = match summary_result {
            Some((summary_message, provider_usage)) => (summary_message, Some(provider_usage)),
            None => {
                // No summary was generated (empty input)
                tracing::warn!("Summarization failed. Returning empty messages.");
                return Ok((Conversation::empty(), vec![], None));
            }
        };

        // Create the final message list with updated visibility metadata:
        // 1. Original messages become user_visible but not agent_visible
        // 2. Summary message becomes agent_visible but not user_visible
        // 3. Assistant messages to continue the conversation remain both user_visible and agent_visible

        let mut final_messages = Vec::new();
        let mut final_token_counts = Vec::new();

        // Add all original messages with updated visibility (preserve user_visible, set agent_visible=false)
        for msg in messages.iter().cloned() {
            let updated_metadata = msg.metadata.with_agent_invisible();
            let updated_msg = msg.with_metadata(updated_metadata);
            final_messages.push(updated_msg);
            // Token count doesn't matter for agent_visible=false messages, but we'll use 0
            final_token_counts.push(0);
        }

        // Add the compaction marker (user_visible=true, agent_visible=false)
        let compaction_marker = Message::assistant()
            .with_summarization_requested("Conversation compacted and summarized")
            .with_metadata(MessageMetadata::user_only());
        let compaction_marker_tokens: usize = 0; // Not counted since agent_visible=false
        final_messages.push(compaction_marker);
        final_token_counts.push(compaction_marker_tokens);

        // Add the summary message (agent_visible=true, user_visible=false)
        let summary_msg = summary_message.with_metadata(MessageMetadata::agent_only());
        // For token counting purposes, we use the output tokens (the actual summary content)
        // since that's what will be in the context going forward
        let summary_tokens = summarization_usage
            .as_ref()
            .and_then(|usage| usage.usage.output_tokens)
            .unwrap_or(0) as usize;
        final_messages.push(summary_msg);
        final_token_counts.push(summary_tokens);

        // Add an assistant message to continue the conversation (agent_visible=true, user_visible=false)
        let assistant_message = Message::assistant().with_text("
            The previous message contains a summary that was prepared because a context limit was reached.
            Do not mention that you read a summary or that conversation summarization occurred
            Just continue the conversation naturally based on the summarized context
        ").with_metadata(MessageMetadata::agent_only());
        let assistant_message_tokens: usize = 0; // Not counted since it's for agent context only
        final_messages.push(assistant_message);
        final_token_counts.push(assistant_message_tokens);

        Ok((
            Conversation::new_unvalidated(final_messages),
            final_token_counts,
            summarization_usage,
        ))
    }
}
