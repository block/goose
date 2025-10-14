use super::super::agents::Agent;
use crate::conversation::message::{Message, MessageMetadata};
use crate::conversation::Conversation;
use crate::prompt_template::render_global_file;
use anyhow::Ok;
use rmcp::model::Role;
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

#[derive(Serialize)]
struct SummarizeContext {
    messages: String,
}

use crate::providers::base::{Provider, ProviderUsage};

/// Summarization function that uses the detailed prompt from the markdown template
async fn get_summary_from_provider(
    provider: Arc<dyn Provider>,
    messages: &[Message],
) -> anyhow::Result<Option<(Message, ProviderUsage)>, anyhow::Error> {
    if messages.is_empty() {
        return std::prelude::rust_2015::Ok(None);
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

    // Send the request to the provider and fetch the response
    let (mut response, mut provider_usage) = provider
        .complete_fast(&system_prompt, &summarization_request, &[])
        .await?;

    // Set role to user as it will be used in following conversation as user content
    response.role = Role::User;

    // Ensure we have token counts, estimating if necessary
    provider_usage
        .ensure_tokens(&system_prompt, &summarization_request, &response, &[])
        .await
        .map_err(|e| anyhow::anyhow!("Failed to ensure usage tokens: {}", e))?;

    std::prelude::rust_2015::Ok(Some((response, provider_usage)))
}

impl Agent {
    /// Public API to summarize the conversation so that its token count is within the allowed context limit.
    /// Returns the summarized messages, token counts, and the ProviderUsage from summarization
    pub async fn compact_messages(
        &self,
        messages: &[Message], // last message is a user msg that led to assistant message with_context_length_exceeded
    ) -> Result<(Conversation, Vec<usize>, Option<ProviderUsage>), anyhow::Error> {
        info!("Performing message compaction");

        // Check if the most recent message is a user message
        let (messages_to_compact, preserved_user_message) =
            if let Some(last_message) = messages.last() {
                if matches!(last_message.role, rmcp::model::Role::User) {
                    // Remove the last user message before compaction
                    (&messages[..messages.len() - 1], Some(last_message.clone()))
                } else {
                    (messages, None)
                }
            } else {
                (messages, None)
            };

        let provider = self.provider().await?;
        let summary = get_summary_from_provider(provider.clone(), messages).await?;

        let (summary_message, summarization_usage) = match summary {
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
        let assistant_message = Message::assistant()
            .with_text(
                "The previous message contains a summary that was prepared because a context limit was reached.
Do not mention that you read a summary or that conversation summarization occurred
Just continue the conversation naturally based on the summarized context"
            )
            .with_metadata(MessageMetadata::agent_only());
        let assistant_message_tokens: usize = 0; // Not counted since it's for agent context only
        final_messages.push(assistant_message);
        final_token_counts.push(assistant_message_tokens);

        // Add back the preserved user message if it exists
        if let Some(user_message) = preserved_user_message {
            final_messages.push(user_message);
        }

        Ok((
            Conversation::new_unvalidated(final_messages),
            final_token_counts,
            summarization_usage,
        ))
    }
}
