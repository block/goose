use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use rmcp::model::Role;

use crate::conversation::message::{Message, MessageContent};
use crate::token_counter::TokenCounter;

use super::block::{ContextBlock, ContextPriority, ContextSlot};
use super::policy::{LONG_TOOL_OUTPUT_TOKEN_THRESHOLD, RECENT_MESSAGE_WINDOW};

fn message_estimated_tokens(msg: &Message, token_counter: &TokenCounter) -> usize {
    token_counter.count_chat_tokens("", std::slice::from_ref(msg), &[])
}

fn has_tool_content(msg: &Message) -> bool {
    msg.content.iter().any(|c| {
        matches!(
            c,
            MessageContent::ToolRequest(_) | MessageContent::ToolResponse(_)
        )
    })
}

/// A signature identifying a tool response's content, used to detect
/// duplicate tool output (the same result repeated verbatim later). Hashes
/// the content instead of collecting it into a `String` so a large tool
/// result isn't cloned just to be compared.
fn tool_response_signature(msg: &Message) -> Option<u64> {
    msg.content.iter().find_map(|c| {
        let resp = c.as_tool_response()?;
        let result = resp.tool_result.as_ref().ok()?;

        let mut hasher = DefaultHasher::new();
        let mut has_text = false;
        for item in &result.content {
            if let Some(text) = item.as_text() {
                text.text.hash(&mut hasher);
                has_text = true;
            }
        }

        has_text.then(|| hasher.finish())
    })
}

fn tool_response_failed(msg: &Message) -> bool {
    msg.content.iter().any(|c| {
        c.as_tool_response()
            .map(|resp| resp.tool_result.is_err())
            .unwrap_or(false)
    })
}

/// Classify agent-visible messages into [`ContextBlock`]s.
///
/// Messages from the most recent user message onward are `Required` (current
/// user message, most recent assistant response, unresolved tool state). The
/// window just before that is `High` priority (recent conversation / recent
/// tool results). Anything older is `Medium` priority (a summarization
/// candidate) unless it's a duplicate tool response or a failed tool attempt,
/// in which case it's `Low` priority (droppable).
///
/// The returned blocks are in chronological order.
pub fn classify_blocks(messages: &[Message], token_counter: &TokenCounter) -> Vec<ContextBlock> {
    let indexed: Vec<&Message> = messages.iter().filter(|m| m.is_agent_visible()).collect();
    if indexed.is_empty() {
        return Vec::new();
    }

    let last_pos = indexed.len() - 1;
    let last_user_pos = indexed.iter().rposition(|m| m.role == Role::User);
    let required_start = last_user_pos.unwrap_or(last_pos);

    // Only the most recent occurrence of an identical tool response is kept;
    // earlier ones are duplicates.
    let mut latest_signature_pos: HashMap<u64, usize> = HashMap::new();
    for (pos, msg) in indexed.iter().enumerate() {
        if let Some(sig) = tool_response_signature(msg) {
            latest_signature_pos.insert(sig, pos);
        }
    }

    let mut blocks = Vec::with_capacity(indexed.len());
    let mut recent_window_remaining = RECENT_MESSAGE_WINDOW;

    for pos in (0..=last_pos).rev() {
        let msg = indexed[pos];
        let estimated_tokens = message_estimated_tokens(msg, token_counter);
        let is_tool_content = has_tool_content(msg);

        let (priority, slot, reason) = if pos >= required_start {
            let slot = if is_tool_content {
                ContextSlot::RecentToolResults
            } else {
                ContextSlot::RecentConversation
            };
            (ContextPriority::Required, slot, None)
        } else if let Some(sig) = tool_response_signature(msg) {
            if latest_signature_pos.get(&sig).copied() != Some(pos) {
                (
                    ContextPriority::Low,
                    ContextSlot::RecentToolResults,
                    Some("duplicate_tool_output".to_string()),
                )
            } else {
                classify_non_duplicate(
                    msg,
                    is_tool_content,
                    estimated_tokens,
                    &mut recent_window_remaining,
                )
            }
        } else {
            classify_non_duplicate(
                msg,
                is_tool_content,
                estimated_tokens,
                &mut recent_window_remaining,
            )
        };

        blocks.push(ContextBlock {
            slot,
            priority,
            messages: vec![msg.clone()],
            estimated_tokens,
            label: format!("{}#{}", slot.as_str(), pos),
            reason,
        });
    }

    blocks.reverse();
    drop_paired_tool_requests(&mut blocks);
    blocks
}

/// Downgrades the assistant tool-request message paired with each dropped
/// (`Low`) tool response so the exchange is removed as a unit. Dropping only
/// the response side would leave an orphaned tool call in the packet —
/// something providers reject — and would under-report the tokens the drop
/// actually saves. A request block is only downgraded when it isn't
/// `Required`, every tool call it carries points at a dropped response, and
/// it contains nothing but the calls themselves (plus thinking content,
/// which is meaningless without them).
fn drop_paired_tool_requests(blocks: &mut [ContextBlock]) {
    let mut dropped_response_reasons: HashMap<String, String> = HashMap::new();
    for block in blocks.iter() {
        if block.priority != ContextPriority::Low {
            continue;
        }
        for content in block.messages.iter().flat_map(|m| m.content.iter()) {
            if let Some(resp) = content.as_tool_response() {
                let reason = block
                    .reason
                    .clone()
                    .unwrap_or_else(|| "low_value".to_string());
                dropped_response_reasons.insert(resp.id.clone(), reason);
            }
        }
    }
    if dropped_response_reasons.is_empty() {
        return;
    }

    for block in blocks.iter_mut() {
        if matches!(
            block.priority,
            ContextPriority::Required | ContextPriority::Low
        ) {
            continue;
        }
        let Some(msg) = block.messages.first() else {
            continue;
        };
        if msg.role != Role::Assistant {
            continue;
        }

        let request_ids: Vec<&str> = msg
            .content
            .iter()
            .filter_map(|c| c.as_tool_request().map(|r| r.id.as_str()))
            .collect();
        if request_ids.is_empty() {
            continue;
        }

        let all_responses_dropped = request_ids
            .iter()
            .all(|id| dropped_response_reasons.contains_key(*id));
        let nothing_else_of_value = msg.content.iter().all(|c| {
            matches!(
                c,
                MessageContent::ToolRequest(_)
                    | MessageContent::Thinking(_)
                    | MessageContent::RedactedThinking(_)
            )
        });

        if all_responses_dropped && nothing_else_of_value {
            block.priority = ContextPriority::Low;
            block.reason = dropped_response_reasons.get(request_ids[0]).cloned();
        }
    }
}

fn classify_non_duplicate(
    msg: &Message,
    is_tool_content: bool,
    estimated_tokens: usize,
    recent_window_remaining: &mut usize,
) -> (ContextPriority, ContextSlot, Option<String>) {
    if tool_response_failed(msg) {
        return (
            ContextPriority::Low,
            ContextSlot::RecentToolResults,
            Some("stale_failed_attempt".to_string()),
        );
    }

    if *recent_window_remaining > 0 {
        *recent_window_remaining -= 1;
        if is_tool_content && estimated_tokens > LONG_TOOL_OUTPUT_TOKEN_THRESHOLD {
            return (
                ContextPriority::Medium,
                ContextSlot::SummarizedToolResults,
                Some("long_tool_output".to_string()),
            );
        }
        let slot = if is_tool_content {
            ContextSlot::RecentToolResults
        } else {
            ContextSlot::RecentConversation
        };
        return (ContextPriority::High, slot, None);
    }

    if is_tool_content {
        (
            ContextPriority::Medium,
            ContextSlot::SummarizedToolResults,
            Some("older_conversation".to_string()),
        )
    } else {
        (
            ContextPriority::Medium,
            ContextSlot::OlderConversationSummary,
            Some("older_conversation".to_string()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::create_token_counter;
    use rmcp::model::{AnnotateAble, CallToolRequestParams};

    async fn counter() -> TokenCounter {
        create_token_counter().await.unwrap()
    }

    #[tokio::test]
    async fn latest_user_message_and_tail_are_required() {
        let tc = counter().await;
        let messages = vec![
            Message::user().with_text("hello"),
            Message::assistant().with_text("hi there"),
            Message::user().with_text("what's next?"),
        ];
        let blocks = classify_blocks(&messages, &tc);

        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[2].priority, ContextPriority::Required);
        assert!(blocks[2].messages[0]
            .content
            .iter()
            .any(|c| c.as_text() == Some("what's next?")));
    }

    #[tokio::test]
    async fn unresolved_tool_call_after_last_user_message_is_required() {
        let tc = counter().await;
        let messages = vec![
            Message::user().with_text("read the file"),
            Message::assistant()
                .with_tool_request("call1", Ok(CallToolRequestParams::new("read_file"))),
        ];
        let blocks = classify_blocks(&messages, &tc);

        assert!(blocks
            .iter()
            .all(|b| b.priority == ContextPriority::Required));
    }

    #[tokio::test]
    async fn duplicate_tool_output_is_low_priority() {
        let tc = counter().await;
        let messages = vec![
            Message::user().with_text("look this up twice"),
            Message::assistant()
                .with_tool_request("call1", Ok(CallToolRequestParams::new("search"))),
            Message::user().with_tool_response(
                "call1",
                Ok(rmcp::model::CallToolResult::success(vec![
                    rmcp::model::RawContent::text("same result").no_annotation(),
                ])),
            ),
            Message::assistant().with_text("let me check again"),
            Message::assistant()
                .with_tool_request("call2", Ok(CallToolRequestParams::new("search"))),
            Message::user().with_tool_response(
                "call2",
                Ok(rmcp::model::CallToolResult::success(vec![
                    rmcp::model::RawContent::text("same result").no_annotation(),
                ])),
            ),
            Message::user().with_text("thanks"),
        ];
        let blocks = classify_blocks(&messages, &tc);

        let duplicate = blocks
            .iter()
            .find(|b| b.reason.as_deref() == Some("duplicate_tool_output"));
        assert!(
            duplicate.is_some(),
            "expected the earlier identical tool response to be flagged as a duplicate"
        );

        // The paired request goes down with its duplicate response so the
        // exchange is removed as a unit — never an orphaned tool call.
        let low_duplicates: Vec<_> = blocks
            .iter()
            .filter(|b| {
                b.priority == ContextPriority::Low
                    && b.reason.as_deref() == Some("duplicate_tool_output")
            })
            .collect();
        assert_eq!(low_duplicates.len(), 2, "response and its request");
        assert!(low_duplicates
            .iter()
            .any(|b| b.messages[0].content.iter().any(|c| c
                .as_tool_request()
                .map(|r| r.id == "call1")
                .unwrap_or(false))));

        // The kept (latest) exchange is untouched on both sides.
        assert!(blocks.iter().any(|b| b.priority != ContextPriority::Low
            && b.messages[0].content.iter().any(|c| c
                .as_tool_request()
                .map(|r| r.id == "call2")
                .unwrap_or(false))));
    }

    #[tokio::test]
    async fn failed_tool_attempt_drops_its_request_too() {
        let tc = counter().await;
        let messages = vec![
            Message::user().with_text("try the flaky tool"),
            Message::assistant()
                .with_tool_request("call1", Ok(CallToolRequestParams::new("flaky"))),
            Message::user().with_tool_response(
                "call1",
                Err(rmcp::model::ErrorData::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    "boom".to_string(),
                    None,
                )),
            ),
            Message::assistant().with_text("that failed, trying something else"),
            Message::user().with_text("ok, thanks"),
        ];
        let blocks = classify_blocks(&messages, &tc);

        let low_failed: Vec<_> = blocks
            .iter()
            .filter(|b| {
                b.priority == ContextPriority::Low
                    && b.reason.as_deref() == Some("stale_failed_attempt")
            })
            .collect();
        assert_eq!(low_failed.len(), 2, "failed response and its request");
    }

    #[tokio::test]
    async fn request_with_text_content_survives_paired_drop() {
        let tc = counter().await;
        let messages = vec![
            Message::user().with_text("try the flaky tool"),
            // The assistant message carries explanation text alongside the
            // call — dropping the whole message would lose that text, so only
            // the response goes; fix_conversation strips the orphaned call.
            Message::assistant()
                .with_text("I'll try the flaky tool now")
                .with_tool_request("call1", Ok(CallToolRequestParams::new("flaky"))),
            Message::user().with_tool_response(
                "call1",
                Err(rmcp::model::ErrorData::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    "boom".to_string(),
                    None,
                )),
            ),
            Message::assistant().with_text("that failed"),
            Message::user().with_text("ok"),
        ];
        let blocks = classify_blocks(&messages, &tc);

        let request_block = blocks
            .iter()
            .find(|b| {
                b.messages[0]
                    .content
                    .iter()
                    .any(|c| c.as_tool_request().is_some())
            })
            .unwrap();
        assert_ne!(
            request_block.priority,
            ContextPriority::Low,
            "a request message carrying real text must not be dropped wholesale"
        );
    }

    #[tokio::test]
    async fn failed_tool_attempt_is_low_priority() {
        let tc = counter().await;
        let messages = vec![
            Message::user().with_text("try the flaky tool"),
            Message::assistant()
                .with_tool_request("call1", Ok(CallToolRequestParams::new("flaky"))),
            Message::user().with_tool_response(
                "call1",
                Err(rmcp::model::ErrorData::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    "boom".to_string(),
                    None,
                )),
            ),
            Message::assistant().with_text("that failed, trying something else"),
            Message::user().with_text("ok, thanks"),
        ];
        let blocks = classify_blocks(&messages, &tc);

        let failed = blocks
            .iter()
            .find(|b| b.reason.as_deref() == Some("stale_failed_attempt"));
        assert!(failed.is_some());
    }

    #[tokio::test]
    async fn old_messages_beyond_window_are_medium_priority() {
        let tc = counter().await;
        let mut messages = Vec::new();
        for i in 0..(RECENT_MESSAGE_WINDOW + 5) {
            messages.push(Message::user().with_text(format!("turn {i}")));
            messages.push(Message::assistant().with_text(format!("reply {i}")));
        }
        messages.push(Message::user().with_text("final question"));
        let blocks = classify_blocks(&messages, &tc);

        assert!(blocks.iter().any(|b| b.priority == ContextPriority::Medium
            && b.reason.as_deref() == Some("older_conversation")));
        assert!(blocks.iter().any(|b| b.priority == ContextPriority::High));
        assert_eq!(blocks.last().unwrap().priority, ContextPriority::Required);
    }
}
