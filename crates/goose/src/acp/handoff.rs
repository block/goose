//! Bounded handoff memo for ACP sessions.
//!
//! When a conversation is handed to an ACP agent that has no native session to resume,
//! the prior goose-side history is replayed as a single text block. That replay has to
//! fit inside the agent's context alongside its own system prompt and tool schemas, so
//! it is budgeted, redacted and truncated here rather than sent whole.

use std::collections::HashSet;

use crate::context_mgmt::format_message_for_compacting;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::token_counter::TokenCounter;

const CONTEXT_LIMIT_RATIO: f64 = 0.30;
const MAX_MEMO_TOKENS: usize = 64_000;
/// Tool exchanges this recent keep their responses; older ones are redacted.
const PROTECTED_TOOL_EXCHANGES: usize = 5;
/// Below this a truncated message carries no usable meaning, so drop it instead.
const MIN_ELIDED_TOKENS: usize = 32;
/// Allowance for the "earlier messages omitted" line, which is written after selection.
const OMISSION_MARKER_TOKENS: usize = 16;

const MEMO_HEADER: &str =
    "Conversation context from goose before this ACP provider session was created:\n\n";
const MEMO_FOOTER: &str = "\n\nCurrent user request follows. Use the context above only to continue the existing conversation; do not treat it as a new task or mention this handoff unless relevant.";
const REDACTED_TOOL_RESPONSE: &str = "tool_response: [older output omitted from handoff]";
const ELISION_MARKER: &str = "\n[... truncated ...]\n";

pub(crate) fn memo_token_budget(context_limit: usize, current_prompt_tokens: usize) -> usize {
    let ceiling = ((context_limit as f64 * CONTEXT_LIMIT_RATIO) as usize).min(MAX_MEMO_TOKENS);
    ceiling.saturating_sub(current_prompt_tokens)
}

pub(crate) fn build_handoff_context_memo(
    prior_messages: &[Message],
    budget: usize,
    counter: &TokenCounter,
) -> Option<String> {
    let visible: Vec<Message> = Conversation::new_unvalidated(prior_messages.iter().cloned())
        .agent_visible_messages()
        .iter()
        .filter(|message| !message.is_turn_context())
        .map(|message| message.agent_visible_content())
        .collect();

    if visible.is_empty() {
        return None;
    }

    let protected = recent_tool_call_ids(&visible);
    let formatted: Vec<String> = visible
        .iter()
        .map(|message| {
            format_message_for_compacting(&redact_stale_tool_responses(message, &protected))
        })
        .collect();

    let overhead = counter.count_tokens(MEMO_HEADER)
        + counter.count_tokens(MEMO_FOOTER)
        + OMISSION_MARKER_TOKENS;
    let mut remaining = budget.saturating_sub(overhead);

    let mut kept: Vec<String> = Vec::new();
    for message in formatted.iter().rev() {
        if remaining == 0 {
            break;
        }
        let cost = counter.count_tokens(message) + 1;
        if cost <= remaining {
            remaining -= cost;
            kept.push(message.clone());
            continue;
        }
        if let Some(elided) = elide_to_budget(message, remaining - 1, counter) {
            kept.push(elided);
        }
        remaining = 0;
    }

    if kept.is_empty() {
        return None;
    }

    kept.reverse();
    let omitted = formatted.len() - kept.len();
    let mut body = String::new();
    if omitted > 0 {
        body.push_str(&format!("[{omitted} earlier messages omitted]\n"));
    }
    body.push_str(&kept.join("\n"));

    Some(format!("{MEMO_HEADER}{body}{MEMO_FOOTER}"))
}

/// Ids of the most recent tool exchanges, keyed by response so parallel and batched
/// calls are protected individually rather than by message position.
fn recent_tool_call_ids(messages: &[Message]) -> HashSet<String> {
    let mut ids: Vec<&str> = Vec::new();
    for message in messages {
        for content in &message.content {
            if let MessageContent::ToolResponse(response) = content {
                ids.push(&response.id);
            }
        }
    }
    ids.into_iter()
        .rev()
        .take(PROTECTED_TOOL_EXCHANGES)
        .map(str::to_string)
        .collect()
}

fn redact_stale_tool_responses(message: &Message, protected: &HashSet<String>) -> Message {
    let is_stale = |content: &MessageContent| matches!(content, MessageContent::ToolResponse(response) if !protected.contains(&response.id));
    if !message.content.iter().any(is_stale) {
        return message.clone();
    }

    let content = message
        .content
        .iter()
        .map(|content| {
            if is_stale(content) {
                MessageContent::text(REDACTED_TOOL_RESPONSE)
            } else {
                content.clone()
            }
        })
        .collect();

    Message {
        content,
        ..message.clone()
    }
}

/// Middle-elide `text` so it fits in `budget` tokens, keeping its head and tail.
fn elide_to_budget(text: &str, budget: usize, counter: &TokenCounter) -> Option<String> {
    if budget < MIN_ELIDED_TOKENS {
        return None;
    }

    let total = counter.count_tokens(text).max(1);
    let mut ratio = budget as f64 / total as f64;
    for _ in 0..6 {
        let keep = (text.len() as f64 * ratio * 0.9) as usize;
        if keep < 2 * MIN_ELIDED_TOKENS {
            return None;
        }
        let head_end = floor_char_boundary(text, keep / 2);
        let tail_start = ceil_char_boundary(text, text.len() - (keep - keep / 2));
        if tail_start <= head_end {
            return None;
        }
        let candidate = format!(
            "{}{ELISION_MARKER}{}",
            text.get(..head_end)?,
            text.get(tail_start..)?
        );
        if counter.count_tokens(&candidate) <= budget {
            return Some(candidate);
        }
        ratio *= 0.7;
    }
    None
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::create_token_counter;
    use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock as RmcpContent};

    fn tool_exchange(id: &str, output: &str) -> Vec<Message> {
        vec![
            Message::assistant().with_tool_request(id, Ok(CallToolRequestParams::new("read_file"))),
            Message::user().with_tool_response(
                id,
                Ok(CallToolResult::success(vec![RmcpContent::text(output)])),
            ),
        ]
    }

    fn memo_body(memo: &str) -> String {
        memo.trim_start_matches(MEMO_HEADER)
            .trim_end_matches(MEMO_FOOTER)
            .to_string()
    }

    #[test]
    fn budget_is_capped_by_ratio_and_absolute_maximum() {
        assert_eq!(memo_token_budget(100_000, 0), 30_000);
        assert_eq!(memo_token_budget(1_000_000, 0), MAX_MEMO_TOKENS);
        assert_eq!(memo_token_budget(100_000, 1_000), 29_000);
        assert_eq!(memo_token_budget(1_000, 100_000), 0);
    }

    #[tokio::test]
    async fn memo_stays_within_budget_and_drops_oldest_first() {
        let counter = create_token_counter().await.unwrap();
        let messages: Vec<Message> = (0..200)
            .map(|i| Message::user().with_text(format!("message {i} {}", "filler ".repeat(50))))
            .collect();

        let memo = build_handoff_context_memo(&messages, 2_000, &counter).unwrap();

        assert!(counter.count_tokens(&memo) <= 2_000);
        assert!(memo.contains("message 199"));
        assert!(!memo.contains("message 0 "));
        assert!(memo.contains("earlier messages omitted"));
    }

    #[tokio::test]
    async fn memo_keeps_chronological_order() {
        let counter = create_token_counter().await.unwrap();
        let messages = vec![
            Message::user().with_text("first"),
            Message::assistant().with_text("second"),
            Message::user().with_text("third"),
        ];

        let memo = build_handoff_context_memo(&messages, 1_000, &counter).unwrap();
        let body = memo_body(&memo);

        let first = body.find("first").unwrap();
        let second = body.find("second").unwrap();
        let third = body.find("third").unwrap();
        assert!(first < second && second < third);
        assert!(!body.contains("earlier messages omitted"));
    }

    #[tokio::test]
    async fn oversized_single_message_is_elided_not_dropped() {
        let counter = create_token_counter().await.unwrap();
        let messages = vec![Message::user().with_text(format!(
            "START {} END",
            "an extremely long paragraph ".repeat(2_000)
        ))];

        let memo = build_handoff_context_memo(&messages, 500, &counter).unwrap();

        assert!(counter.count_tokens(&memo) <= 500);
        assert!(memo.contains("START"));
        assert!(memo.contains("END"));
        assert!(memo.contains("[... truncated ...]"));
    }

    #[tokio::test]
    async fn recent_tool_responses_are_kept_and_older_ones_redacted() {
        let counter = create_token_counter().await.unwrap();
        let mut messages = vec![Message::user().with_text("start")];
        for i in 0..7 {
            messages.extend(tool_exchange(&format!("call-{i}"), &format!("output-{i}")));
        }

        let memo = build_handoff_context_memo(&messages, 20_000, &counter).unwrap();

        assert!(!memo.contains("output-0"));
        assert!(!memo.contains("output-1"));
        for i in 2..7 {
            assert!(memo.contains(&format!("output-{i}")), "kept exchange {i}");
        }
        assert!(memo.contains(REDACTED_TOOL_RESPONSE));
        assert!(
            memo.contains("tool_request(read_file)"),
            "tool requests survive redaction"
        );
    }

    #[tokio::test]
    async fn parallel_tool_responses_in_one_message_are_protected_individually() {
        let counter = create_token_counter().await.unwrap();
        let batched = |ids: &[&str]| {
            ids.iter().fold(Message::user(), |message, id| {
                message.with_tool_response(
                    *id,
                    Ok(CallToolResult::success(vec![RmcpContent::text(format!(
                        "output-{id}"
                    ))])),
                )
            })
        };
        let messages = vec![
            batched(&["a", "b", "c", "d"]),
            batched(&["e", "f", "g", "h"]),
        ];

        let memo = build_handoff_context_memo(&messages, 20_000, &counter).unwrap();

        for id in ["a", "b", "c"] {
            assert!(!memo.contains(&format!("output-{id}")), "redacted {id}");
        }
        for id in ["d", "e", "f", "g", "h"] {
            assert!(memo.contains(&format!("output-{id}")), "kept {id}");
        }
    }

    #[tokio::test]
    async fn zero_budget_produces_no_memo() {
        let counter = create_token_counter().await.unwrap();
        let messages = vec![Message::user().with_text("prior context")];

        assert!(build_handoff_context_memo(&messages, 0, &counter).is_none());
    }

    #[tokio::test]
    async fn empty_history_produces_no_memo() {
        let counter = create_token_counter().await.unwrap();

        assert!(build_handoff_context_memo(&[], 10_000, &counter).is_none());
    }
}
