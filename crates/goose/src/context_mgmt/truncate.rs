use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::utils::safe_truncate;
use anyhow::{anyhow, Result};
use rmcp::model::{RawContent, ResourceContents, Role};
use std::collections::HashSet;
use std::ops::DerefMut;
use tracing::{debug, warn};

/// Maximum size for truncated content in characters
const MAX_TRUNCATED_CONTENT_SIZE: usize = 5000;

/// Handles messages that are individually larger than the context limit
/// by truncating their content rather than removing them entirely
fn handle_oversized_messages(
    messages: &[Message],
    token_counts: &[usize],
    context_limit: usize,
    strategy: &dyn TruncationStrategy,
) -> Result<(Conversation, Vec<usize>), anyhow::Error> {
    let mut truncated_messages = Vec::new();
    let mut truncated_token_counts = Vec::new();
    let mut any_truncated = false;

    // Create a basic token counter for re-estimating truncated content
    // Note: This is a rough approximation since we don't have access to the actual tokenizer here
    let estimate_tokens = |text: &str| -> usize {
        // Rough approximation: 1 token per 4 characters for English text
        (text.len() / 4).max(1)
    };

    for (i, (message, &original_tokens)) in messages.iter().zip(token_counts.iter()).enumerate() {
        if original_tokens > context_limit {
            warn!(
                "Message {} has {} tokens, exceeding context limit of {}",
                i, original_tokens, context_limit
            );

            // Try to truncate the message content
            let truncated_message = truncate_message_content(message, MAX_TRUNCATED_CONTENT_SIZE)?;
            let estimated_new_tokens =
                estimate_message_tokens(&truncated_message, &estimate_tokens);

            if estimated_new_tokens > context_limit {
                // Even truncated message is too large, skip it entirely
                warn!("Skipping message {} as even truncated version ({} tokens) exceeds context limit", i, estimated_new_tokens);
                any_truncated = true;
                continue;
            }

            truncated_messages.push(truncated_message);
            truncated_token_counts.push(estimated_new_tokens);
            any_truncated = true;
        } else {
            truncated_messages.push(message.clone());
            truncated_token_counts.push(original_tokens);
        }
    }

    if any_truncated {
        debug!("Truncated large message content, now attempting normal truncation");
        // After content truncation, try normal truncation if still needed
        return truncate_messages(
            &truncated_messages,
            &truncated_token_counts,
            context_limit,
            strategy,
        );
    }

    Ok((
        Conversation::new_unvalidated(truncated_messages),
        truncated_token_counts,
    ))
}

/// Truncates the content within a message while preserving its structure
fn truncate_message_content(message: &Message, max_content_size: usize) -> Result<Message> {
    let mut new_message = message.clone();

    for content in &mut new_message.content {
        match content {
            MessageContent::Text(text_content) => {
                if text_content.text.chars().count() > max_content_size {
                    let truncated = format!(
                        "{}\n\n[... content truncated from {} to {} characters ...]",
                        safe_truncate(&text_content.text, max_content_size),
                        text_content.text.chars().count(),
                        max_content_size
                    );
                    text_content.text = truncated;
                }
            }
            MessageContent::ToolResponse(tool_response) => {
                if let Ok(ref mut result) = tool_response.tool_result {
                    for content_item in result {
                        if let RawContent::Text(ref mut text_content) = content_item.deref_mut() {
                            if text_content.text.chars().count() > max_content_size {
                                let truncated = format!(
                                    "{}\n\n[... tool response truncated from {} to {} characters ...]",
                                    safe_truncate(&text_content.text, max_content_size),
                                    text_content.text.chars().count(),
                                    max_content_size
                                );
                                text_content.text = truncated;
                            }
                        }
                        // Handle Resource content which might contain large text
                        else if let RawContent::Resource(ref mut resource_content) =
                            content_item.deref_mut()
                        {
                            if let ResourceContents::TextResourceContents { text, .. } =
                                &mut resource_content.resource
                            {
                                if text.chars().count() > max_content_size {
                                    let truncated = format!(
                                        "{}\n\n[... resource content truncated from {} to {} characters ...]",
                                        safe_truncate(text, max_content_size),
                                        text.chars().count(),
                                        max_content_size
                                    );
                                    *text = truncated;
                                }
                            }
                        }
                    }
                }
            }
            // Other content types are typically smaller, but we could extend this if needed
            _ => {}
        }
    }

    Ok(new_message)
}

/// Estimates token count for a message using a simple heuristic
fn estimate_message_tokens(message: &Message, estimate_fn: &dyn Fn(&str) -> usize) -> usize {
    let mut total_tokens = 10; // Base overhead for message structure

    for content in &message.content {
        match content {
            MessageContent::Text(text_content) => {
                total_tokens += estimate_fn(&text_content.text);
            }
            MessageContent::ToolResponse(tool_response) => {
                if let Ok(ref result) = tool_response.tool_result {
                    for content_item in result {
                        match &content_item.raw {
                            RawContent::Text(text_content) => {
                                total_tokens += estimate_fn(&text_content.text);
                            }
                            RawContent::Resource(resource) => {
                                match &resource.resource {
                                    ResourceContents::TextResourceContents { text, .. } => {
                                        total_tokens += estimate_fn(text);
                                    }
                                    _ => total_tokens += 5, // Small overhead for other resource types
                                }
                            }
                            _ => {
                                total_tokens += 5; // Small overhead for other content types
                            }
                        }
                    }
                }
            }
            _ => total_tokens += 5, // Small overhead for other content types
        }
    }

    total_tokens
}

/// Truncates the messages to fit within the model's context window.
/// Mutates the input messages and token counts in place.
/// Returns an error if it's impossible to truncate the messages within the context limit.
/// - messages: The vector of messages in the conversation.
/// - token_counts: A parallel vector containing the token count for each message.
/// - context_limit: The maximum allowed context length in tokens.
/// - strategy: The truncation strategy to use. Only option is OldestFirstTruncation.
pub fn truncate_messages(
    messages: &[Message],
    token_counts: &[usize],
    context_limit: usize,
    strategy: &dyn TruncationStrategy,
) -> Result<(Conversation, Vec<usize>), anyhow::Error> {
    let mut messages = messages.to_owned();
    let mut token_counts = token_counts.to_owned();

    if messages.len() != token_counts.len() {
        return Err(anyhow!(
            "The vector for messages and token_counts must have same length"
        ));
    }

    // Step 1: Calculate total tokens
    let mut total_tokens: usize = token_counts.iter().sum();
    debug!("Total tokens before truncation: {}", total_tokens);

    // Check if any individual message is larger than the context limit
    // First, check for any message that's too large
    let max_message_tokens = token_counts.iter().max().copied().unwrap_or(0);
    if max_message_tokens > context_limit {
        // Try to handle large messages by truncating their content
        debug!(
            "Found oversized message with {} tokens, attempting content truncation",
            max_message_tokens
        );
        return handle_oversized_messages(&messages, &token_counts, context_limit, strategy);
    }

    let min_user_msg_tokens = messages
        .iter()
        .zip(token_counts.iter())
        .filter(|(msg, _)| msg.role == Role::User && msg.has_only_text_content())
        .map(|(_, &tokens)| tokens)
        .min();

    // If there are no valid user messages, or the smallest one is too big for the context
    if min_user_msg_tokens.is_none() || min_user_msg_tokens.unwrap() > context_limit {
        return Err(anyhow!(
            "Not possible to truncate messages within context limit: no suitable user messages found"
        ));
    }

    if total_tokens <= context_limit {
        return Ok((
            Conversation::new_unvalidated(messages.to_vec()),
            token_counts.to_vec(),
        )); // No truncation needed
    }

    // Step 2: Determine indices to remove based on strategy
    let indices_to_remove =
        strategy.determine_indices_to_remove(&messages, &token_counts, context_limit)?;

    // Circuit breaker: if we can't remove enough messages, fail gracefully
    let tokens_to_remove: usize = indices_to_remove
        .iter()
        .map(|&i| token_counts.get(i).copied().unwrap_or(0))
        .sum();

    if total_tokens - tokens_to_remove > context_limit && !indices_to_remove.is_empty() {
        debug!(
            "Standard truncation insufficient: {} tokens remain after removing {} tokens",
            total_tokens - tokens_to_remove,
            tokens_to_remove
        );
        // Try more aggressive truncation or content truncation
        return handle_oversized_messages(&messages, &token_counts, context_limit, strategy);
    }

    if indices_to_remove.is_empty() && total_tokens > context_limit {
        return Err(anyhow!(
            "Cannot truncate any messages: all messages may be essential or too large individually"
        ));
    }

    // Step 3: Remove the marked messages
    // Vectorize the set and sort in reverse order to avoid shifting indices when removing
    let mut indices_to_remove = indices_to_remove.iter().cloned().collect::<Vec<usize>>();
    indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));

    for &index in &indices_to_remove {
        if index < messages.len() {
            let _ = messages.remove(index);
            let removed_tokens = token_counts.remove(index);
            total_tokens -= removed_tokens;
        }
    }

    // Step 4: Ensure the last message is a user message with TextContent only
    while let Some(last_msg) = messages.last() {
        if last_msg.role != Role::User || !last_msg.has_only_text_content() {
            let _ = messages.pop().ok_or(anyhow!("Failed to pop message"))?;
            let removed_tokens = token_counts
                .pop()
                .ok_or(anyhow!("Failed to pop token count"))?;
            total_tokens -= removed_tokens;
        } else {
            break;
        }
    }

    // Step 5: Check first msg is a User message with TextContent only
    while let Some(first_msg) = messages.first() {
        if first_msg.role != Role::User || !first_msg.has_only_text_content() {
            let _ = messages.remove(0);
            let removed_tokens = token_counts.remove(0);
            total_tokens -= removed_tokens;
        } else {
            break;
        }
    }

    debug!("Total tokens after truncation: {}", total_tokens);

    // Ensure we have at least one message remaining and it's within context limit
    if messages.is_empty() {
        return Err(anyhow!(
            "Unable to preserve any messages within context limit"
        ));
    }

    if total_tokens > context_limit {
        return Err(anyhow!(
            "Unable to truncate messages within context window."
        ));
    }

    debug!("Truncation complete. Total tokens: {}", total_tokens);
    Ok((
        Conversation::new_unvalidated(messages.to_vec()),
        token_counts.to_vec(),
    ))
}

/// Trait representing a truncation strategy
pub trait TruncationStrategy {
    /// Determines the indices of messages to remove to fit within the context limit.
    ///
    /// - `messages`: The list of messages in the conversation.
    /// - `token_counts`: A parallel array containing the token count for each message.
    /// - `context_limit`: The maximum allowed context length in tokens.
    ///
    /// Returns a vector of indices to remove.
    fn determine_indices_to_remove(
        &self,
        messages: &[Message],
        token_counts: &[usize],
        context_limit: usize,
    ) -> Result<HashSet<usize>>;
}

/// Strategy to truncate messages by removing the oldest first
pub struct OldestFirstTruncation;

impl TruncationStrategy for OldestFirstTruncation {
    fn determine_indices_to_remove(
        &self,
        messages: &[Message],
        token_counts: &[usize],
        context_limit: usize,
    ) -> Result<HashSet<usize>> {
        let mut indices_to_remove = HashSet::new();
        let mut total_tokens: usize = token_counts.iter().sum();
        let mut tool_ids_to_remove = HashSet::new();

        for (i, message) in messages.iter().enumerate() {
            if total_tokens <= context_limit {
                break;
            }

            // Remove the message
            indices_to_remove.insert(i);
            total_tokens -= token_counts[i];
            debug!(
                "OldestFirst: Removing message at index {}. Tokens removed: {}",
                i, token_counts[i]
            );

            // If it's a ToolRequest or ToolResponse, mark its pair for removal
            if message.is_tool_call() || message.is_tool_response() {
                message.get_tool_ids().iter().for_each(|id| {
                    tool_ids_to_remove.insert((i, id.to_string()));
                });
            }
        }

        // Now, find and remove paired ToolResponses or ToolRequests
        for (i, message) in messages.iter().enumerate() {
            let message_tool_ids = message.get_tool_ids();
            // Find the other part of the pair - same tool_id but different message index
            for (message_idx, tool_id) in &tool_ids_to_remove {
                if message_idx != &i && message_tool_ids.contains(tool_id.as_str()) {
                    indices_to_remove.insert(i);
                    // No need to check other tool_ids for this message since it's already marked
                    break;
                }
            }
        }

        Ok(indices_to_remove)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use anyhow::Result;
    use mcp_core::tool::ToolCall;
    use rmcp::model::Content;
    use serde_json::json;

    // Helper function to create a user text message with a specified token count
    fn user_text(index: usize, tokens: usize) -> (Message, usize) {
        let content = format!("User message {}", index);
        (Message::user().with_text(content), tokens)
    }

    // Helper function to create an assistant text message with a specified token count
    fn assistant_text(index: usize, tokens: usize) -> (Message, usize) {
        let content = format!("Assistant message {}", index);
        (Message::assistant().with_text(content), tokens)
    }

    // Helper function to create a tool request message with a specified token count
    fn assistant_tool_request(id: &str, tool_call: ToolCall, tokens: usize) -> (Message, usize) {
        (
            Message::assistant().with_tool_request(id, Ok(tool_call)),
            tokens,
        )
    }

    // Helper function to create a tool response message with a specified token count
    fn user_tool_response(id: &str, result: Vec<Content>, tokens: usize) -> (Message, usize) {
        (Message::user().with_tool_response(id, Ok(result)), tokens)
    }

    // Helper function to create a large tool response with massive content
    fn large_tool_response(id: &str, large_text: String, tokens: usize) -> (Message, usize) {
        (
            Message::user().with_tool_response(id, Ok(vec![Content::text(large_text)])),
            tokens,
        )
    }

    // Helper function to create messages with alternating user and assistant
    // text messages of a fixed token count
    fn create_messages_with_counts(
        num_pairs: usize,
        tokens: usize,
        remove_last: bool,
    ) -> (Conversation, Vec<usize>) {
        let mut messages = Conversation::new_unvalidated((0..num_pairs).flat_map(|i| {
            vec![
                user_text(i * 2, tokens).0,
                assistant_text((i * 2) + 1, tokens).0,
            ]
        }));

        if remove_last {
            messages.pop();
        }

        let token_counts = vec![tokens; messages.len()];

        (messages, token_counts)
    }

    #[test]
    fn test_handle_oversized_single_message() -> Result<()> {
        // Create a scenario similar to the real issue: one very large tool response
        let large_content = "A".repeat(50000); // Very large content
        let messages = vec![
            user_text(1, 10).0,
            assistant_tool_request(
                "tool1",
                ToolCall::new("read_file", json!({"path": "large_file.txt"})),
                20,
            )
            .0,
            large_tool_response("tool1", large_content, 100000).0, // Massive tool response
            user_text(2, 10).0,
        ];
        let token_counts = vec![10, 20, 100000, 10]; // One message is huge
        let context_limit = 5000; // Much smaller than the large message

        let result = truncate_messages(
            &messages,
            &token_counts,
            context_limit,
            &OldestFirstTruncation,
        );

        // Should succeed by truncating the large content
        assert!(
            result.is_ok(),
            "Should handle oversized message by content truncation"
        );
        let (truncated_messages, truncated_counts) = result.unwrap();

        // Should have some messages remaining
        assert!(
            !truncated_messages.is_empty(),
            "Should have some messages left"
        );

        // Total should be within limit
        let total_tokens: usize = truncated_counts.iter().sum();
        assert!(
            total_tokens <= context_limit,
            "Total tokens {} should be <= context limit {}",
            total_tokens,
            context_limit
        );

        Ok(())
    }

    #[test]
    fn test_oldest_first_no_truncation() -> Result<()> {
        let (messages, token_counts) = create_messages_with_counts(1, 10, false);
        let context_limit = 25;

        let result = truncate_messages(
            &messages.messages(),
            &token_counts,
            context_limit,
            &OldestFirstTruncation,
        )?;

        assert_eq!(result.0.messages(), messages.messages());
        assert_eq!(result.1, token_counts);
        Ok(())
    }

    #[test]
    fn test_complex_conversation_with_tools() -> Result<()> {
        // Simulating a real conversation with multiple tool interactions
        let tool_call1 = ToolCall::new("file_read", json!({"path": "/tmp/test.txt"}));
        let tool_call2 = ToolCall::new("database_query", json!({"query": "SELECT * FROM users"}));

        let messages = vec![
            user_text(1, 15).0, // Initial user query
            assistant_tool_request("tool1", tool_call1.clone(), 20).0,
            user_tool_response(
                "tool1",
                vec![Content::text("File contents".to_string())],
                10,
            )
            .0,
            assistant_text(2, 25).0, // Assistant processes file contents
            user_text(3, 10).0,      // User follow-up
            assistant_tool_request("tool2", tool_call2.clone(), 30).0,
            user_tool_response(
                "tool2",
                vec![Content::text("Query results".to_string())],
                20,
            )
            .0,
            assistant_text(4, 35).0, // Assistant analyzes query results
            user_text(5, 5).0,       // Final user confirmation
        ];

        let token_counts = vec![15, 20, 10, 25, 10, 30, 20, 35, 5];
        let context_limit = 100; // Force truncation while preserving some tool interactions

        let result = truncate_messages(
            &messages,
            &token_counts,
            context_limit,
            &OldestFirstTruncation,
        )?;
        let (truncated_messages, truncated_counts) = result;

        // Verify that tool pairs are kept together and the conversation remains coherent
        assert!(truncated_messages.len() >= 3); // At least one complete interaction should remain
        assert!(truncated_messages.last().unwrap().role == Role::User); // Last message should be from user

        // Verify tool pairs are either both present or both removed
        let tool_ids: HashSet<_> = truncated_messages
            .iter()
            .flat_map(|m| m.get_tool_ids())
            .collect();

        // Each tool ID should appear 0 or 2 times (request + response)
        for id in tool_ids {
            let count = truncated_messages
                .iter()
                .flat_map(|m| m.get_tool_ids().into_iter())
                .filter(|&tool_id| tool_id == id)
                .count();
            assert!(count == 0 || count == 2, "Tool pair was split: {}", id);
        }

        // Total should be within limit
        let total_tokens: usize = truncated_counts.iter().sum();
        assert!(total_tokens <= context_limit);

        Ok(())
    }

    #[test]
    fn test_edge_case_context_window() -> Result<()> {
        // Test case where we're exactly at the context limit
        let (messages, token_counts) = create_messages_with_counts(2, 25, false);
        let context_limit = 100; // Exactly matches total tokens

        let result = truncate_messages(
            &messages.messages(),
            &token_counts,
            context_limit,
            &OldestFirstTruncation,
        )?;
        let (mut messages, mut token_counts) = result;

        assert_eq!(messages.len(), 4); // No truncation needed
        assert_eq!(token_counts.iter().sum::<usize>(), 100);

        // Now add one more token to force truncation
        messages.push(user_text(5, 1).0);
        token_counts.push(1);

        let result = truncate_messages(
            &messages.messages(),
            &token_counts,
            context_limit,
            &OldestFirstTruncation,
        )?;
        let (messages, token_counts) = result;

        assert!(token_counts.iter().sum::<usize>() <= context_limit);
        assert!(messages.last().unwrap().role == Role::User);

        Ok(())
    }

    #[test]
    fn test_multi_tool_chain() -> Result<()> {
        // Simulate a chain of dependent tool calls
        let tool_calls = vec![
            ToolCall::new("git_status", json!({})),
            ToolCall::new("git_diff", json!({"file": "main.rs"})),
            ToolCall::new("git_commit", json!({"message": "Update"})),
        ];

        let mut messages = Vec::new();
        let mut token_counts = Vec::new();

        // Build a chain of related tool calls
        // 30 tokens each round
        for (i, tool_call) in tool_calls.into_iter().enumerate() {
            let id = format!("git_{}", i);
            messages.push(user_text(i, 10).0);
            token_counts.push(10);

            messages.push(assistant_tool_request(&id, tool_call, 15).0);
            token_counts.push(20);
        }

        let context_limit = 50; // Force partial truncation

        let result = truncate_messages(
            &messages,
            &token_counts,
            context_limit,
            &OldestFirstTruncation,
        )?;
        let (truncated_messages, _) = result;

        // Verify that remaining tool chains are complete
        let remaining_tool_ids: HashSet<_> = truncated_messages
            .iter()
            .flat_map(|m| m.get_tool_ids())
            .collect();

        for _id in remaining_tool_ids {
            // Count request/response pairs
            let requests = truncated_messages
                .iter()
                .flat_map(|m| m.get_tool_request_ids().into_iter())
                .count();

            let responses = truncated_messages
                .iter()
                .flat_map(|m| m.get_tool_response_ids().into_iter())
                .count();

            assert_eq!(requests, 1, "Each remaining tool should have one request");
            assert_eq!(responses, 1, "Each remaining tool should have one response");
        }

        Ok(())
    }

    #[test]
    fn test_truncation_with_image_content() -> Result<()> {
        // Create a conversation with image content mixed in
        let messages = vec![
            Message::user().with_image("base64_data", "image/png"), // 50 tokens
            Message::assistant().with_text("I see the image"),      // 10 tokens
            Message::user().with_text("Can you describe it?"),      // 10 tokens
            Message::assistant().with_text("It shows..."),          // 20 tokens
            Message::user().with_text("Thanks!"),                   // 5 tokens
        ];
        let token_counts = vec![50, 10, 10, 20, 5];
        let context_limit = 45; // Force truncation

        let result = truncate_messages(
            &messages,
            &token_counts,
            context_limit,
            &OldestFirstTruncation,
        )?;
        let (messages, token_counts) = result;

        // Verify the conversation still makes sense
        assert!(messages.len() >= 1);
        assert!(messages.last().unwrap().role == Role::User);
        assert!(token_counts.iter().sum::<usize>() <= context_limit);

        Ok(())
    }

    #[test]
    fn test_error_cases() -> Result<()> {
        // Test impossibly small context window
        let (messages, token_counts) = create_messages_with_counts(1, 10, false);
        let result = truncate_messages(
            &messages.messages(),
            &token_counts,
            5, // Impossibly small context
            &OldestFirstTruncation,
        );
        assert!(result.is_err());

        // Test unmatched token counts
        let messages = vec![user_text(1, 10).0];
        let token_counts = vec![10, 10]; // Mismatched length
        let result = truncate_messages(&messages, &token_counts, 100, &OldestFirstTruncation);
        assert!(result.is_err());

        Ok(())
    }
}
