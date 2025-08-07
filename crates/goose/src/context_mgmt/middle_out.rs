use crate::message::Message;
use anyhow::Result;
use rmcp::model::Role;
use std::collections::HashSet;
use tracing::{debug, warn};

use super::truncate::{truncate_messages, TruncationStrategy};

#[allow(dead_code)]
const ANTHROPIC_MAX_MESSAGES: usize = 200000;

pub struct MiddleOutCompression;

impl MiddleOutCompression {
    pub fn compress(
        messages: &[Message],
        token_counts: &[usize],
        context_limit: usize,
        max_output_tokens: usize,
    ) -> Result<(Vec<Message>, Vec<usize>)> {
        let total_tokens: usize = token_counts.iter().sum();
        let required_tokens = total_tokens + max_output_tokens;

        if required_tokens <= context_limit {
            return Ok((messages.to_vec(), token_counts.to_vec()));
        }

        let effective_limit = context_limit.saturating_sub(max_output_tokens);

        if effective_limit < context_limit / 2 {
            warn!(
                "Context window too small for middle-out compression. Need at least {} tokens, have {}",
                context_limit / 2,
                effective_limit
            );
        }

        let strategy = MiddleOutStrategy::new();
        truncate_messages(messages, token_counts, effective_limit, &strategy)
    }

    pub fn compress_for_message_count(
        messages: &[Message],
        token_counts: &[usize],
        max_messages: usize,
    ) -> Result<(Vec<Message>, Vec<usize>)> {
        if messages.len() <= max_messages {
            return Ok((messages.to_vec(), token_counts.to_vec()));
        }

        let strategy = MiddleOutStrategy::new();
        let indices_to_remove =
            strategy.determine_indices_for_message_count(messages, max_messages)?;

        let mut remaining_messages = Vec::new();
        let mut remaining_counts = Vec::new();

        for (i, (msg, &count)) in messages.iter().zip(token_counts.iter()).enumerate() {
            if !indices_to_remove.contains(&i) {
                remaining_messages.push(msg.clone());
                remaining_counts.push(count);
            }
        }

        Ok((remaining_messages, remaining_counts))
    }

    pub fn should_apply_by_default(model_context_length: usize) -> bool {
        model_context_length <= 8192
    }
}

struct MiddleOutStrategy {
    #[allow(dead_code)]
    preserve_system: bool,
    preserve_tools: bool,
}

impl MiddleOutStrategy {
    fn new() -> Self {
        Self {
            preserve_system: true,
            preserve_tools: true,
        }
    }

    fn determine_indices_for_message_count(
        &self,
        messages: &[Message],
        max_messages: usize,
    ) -> Result<HashSet<usize>> {
        let mut indices_to_remove = HashSet::new();

        if messages.len() <= max_messages {
            return Ok(indices_to_remove);
        }

        let messages_to_remove = messages.len() - max_messages;
        let keep_from_start = max_messages / 2;
        let keep_from_end = max_messages - keep_from_start;

        let mut protected_indices = HashSet::new();

        for i in 0..keep_from_start.min(messages.len()) {
            protected_indices.insert(i);
        }

        for i in (messages.len().saturating_sub(keep_from_end))..messages.len() {
            protected_indices.insert(i);
        }

        let mut tool_pairs = HashSet::new();
        for (i, msg) in messages.iter().enumerate() {
            if msg.is_tool_call() || msg.is_tool_response() {
                for tool_id in msg.get_tool_ids() {
                    tool_pairs.insert((i, tool_id.to_string()));
                }
            }
        }

        for (i, msg) in messages.iter().enumerate() {
            if protected_indices.contains(&i) {
                continue;
            }

            if indices_to_remove.len() >= messages_to_remove {
                break;
            }

            if self.preserve_tools && (msg.is_tool_call() || msg.is_tool_response()) {
                let tool_ids = msg.get_tool_ids();
                let mut can_remove = true;

                for tool_id in &tool_ids {
                    for (tool_idx, tid) in &tool_pairs {
                        if tid == tool_id && *tool_idx != i && protected_indices.contains(tool_idx)
                        {
                            can_remove = false;
                            break;
                        }
                    }
                    if !can_remove {
                        break;
                    }
                }

                if !can_remove {
                    continue;
                }

                for tool_id in tool_ids {
                    for (tool_idx, tid) in &tool_pairs {
                        if tid == tool_id && *tool_idx != i {
                            indices_to_remove.insert(*tool_idx);
                        }
                    }
                }
            }

            indices_to_remove.insert(i);
        }

        debug!(
            "Middle-out compression for message count: removing {} messages from middle",
            indices_to_remove.len()
        );

        Ok(indices_to_remove)
    }

    fn find_middle_indices(
        &self,
        messages: &[Message],
        token_counts: &[usize],
        tokens_to_remove: usize,
    ) -> HashSet<usize> {
        let mut indices = HashSet::new();
        let total_messages = messages.len();

        if total_messages <= 2 {
            return indices;
        }

        let start_preserve = total_messages / 4;
        let end_preserve = total_messages / 4;

        let middle_start = start_preserve;
        let middle_end = total_messages.saturating_sub(end_preserve);

        if middle_start >= middle_end {
            return indices;
        }

        let mut removed_tokens = 0;
        let mut middle_indices: Vec<(usize, usize)> = Vec::new();

        for (i, &tokens) in token_counts
            .iter()
            .enumerate()
            .take(middle_end)
            .skip(middle_start)
        {
            middle_indices.push((i, tokens));
        }

        middle_indices.sort_by(|a, b| b.1.cmp(&a.1));

        for (idx, tokens) in middle_indices {
            if removed_tokens >= tokens_to_remove {
                break;
            }

            let msg = &messages[idx];

            if msg.role == Role::User && msg.has_only_text_content() {
                continue;
            }

            if self.preserve_tools && (msg.is_tool_call() || msg.is_tool_response()) {
                let mut skip = false;
                for tool_id in msg.get_tool_ids() {
                    for (i, other_msg) in messages.iter().enumerate() {
                        if i != idx
                            && other_msg.get_tool_ids().contains(&tool_id)
                            && (i < middle_start || i >= middle_end)
                        {
                            skip = true;
                            break;
                        }
                    }
                    if skip {
                        break;
                    }
                }
                if skip {
                    continue;
                }
            }

            indices.insert(idx);
            removed_tokens += tokens;
        }

        indices
    }
}

impl TruncationStrategy for MiddleOutStrategy {
    fn determine_indices_to_remove(
        &self,
        messages: &[Message],
        token_counts: &[usize],
        context_limit: usize,
    ) -> Result<HashSet<usize>> {
        let total_tokens: usize = token_counts.iter().sum();

        if total_tokens <= context_limit {
            return Ok(HashSet::new());
        }

        let tokens_to_remove = total_tokens - context_limit;
        let indices = self.find_middle_indices(messages, token_counts, tokens_to_remove);

        debug!(
            "Middle-out compression: removing {} messages ({} tokens) from middle",
            indices.len(),
            indices.iter().map(|&i| token_counts[i]).sum::<usize>()
        );

        Ok(indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;

    fn create_test_messages(count: usize) -> (Vec<Message>, Vec<usize>) {
        let mut messages = Vec::new();
        let mut token_counts = Vec::new();

        for i in 0..count {
            if i % 2 == 0 {
                messages.push(Message::user().with_text(format!("User message {}", i)));
            } else {
                messages.push(Message::assistant().with_text(format!("Assistant message {}", i)));
            }
            token_counts.push(100);
        }

        (messages, token_counts)
    }

    #[test]
    fn test_no_compression_needed() {
        let (messages, token_counts) = create_test_messages(5);
        let result = MiddleOutCompression::compress(&messages, &token_counts, 1000, 100).unwrap();

        assert_eq!(result.0.len(), messages.len());
        assert_eq!(result.1, token_counts);
    }

    #[test]
    fn test_middle_out_compression() {
        let (messages, token_counts) = create_test_messages(10);
        let result = MiddleOutCompression::compress(&messages, &token_counts, 600, 100).unwrap();

        assert!(result.0.len() < messages.len());
        assert!(result.1.iter().sum::<usize>() <= 500);

        assert_eq!(result.0[0].content[0].to_string(), "User message 0");

        let last_idx = result.0.len() - 1;
        assert!(
            result.0[last_idx].role == Role::User || result.0[last_idx].role == Role::Assistant
        );
    }

    #[test]
    fn test_message_count_compression() {
        let (messages, token_counts) = create_test_messages(10);
        let result =
            MiddleOutCompression::compress_for_message_count(&messages, &token_counts, 6).unwrap();

        assert_eq!(result.0.len(), 6);

        assert_eq!(result.0[0].content[0].to_string(), "User message 0");
    }

    #[test]
    fn test_should_apply_by_default() {
        assert!(MiddleOutCompression::should_apply_by_default(8192));
        assert!(MiddleOutCompression::should_apply_by_default(4096));
        assert!(!MiddleOutCompression::should_apply_by_default(16384));
        assert!(!MiddleOutCompression::should_apply_by_default(128000));
    }
}
