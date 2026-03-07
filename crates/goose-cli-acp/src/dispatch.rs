use std::time::Instant;

use sacp::schema::{ContentBlock, SessionNotification, SessionUpdate, ToolCallStatus};
use sacp::util::MatchMessage;

use crate::display;
use crate::stream::StoredToolOutput;
use crate::stream::TurnState;

pub(crate) async fn handle_message(
    msg: sacp::MessageCx,
    ctx: &mut TurnState<'_>,
) -> Result<(), sacp::Error> {
    MatchMessage::new(msg)
        .if_notification(async |notif: SessionNotification| {
            match notif.update {
                SessionUpdate::AgentMessageChunk(chunk) => {
                    if let ContentBlock::Text(t) = &chunk.content {
                        ctx.mode.render_agent_text(&t.text);
                    }
                }
                SessionUpdate::AgentThoughtChunk(chunk) => {
                    if let ContentBlock::Text(t) = &chunk.content {
                        ctx.mode.render_thinking(&t.text);
                    }
                }
                SessionUpdate::ToolCall(tc) => {
                    let summary = tc.raw_input.as_ref().and_then(display::summarize_args);
                    ctx.mode.render_tool_start(&tc.title, tc.raw_input.as_ref());
                    ctx.active_tools
                        .insert(tc.tool_call_id.clone(), (Instant::now(), summary));
                }
                SessionUpdate::ToolCallUpdate(tcu) => {
                    handle_tool_call_update(tcu, ctx);
                }
                _ => {}
            }
            Ok(())
        })
        .await
        .otherwise_ignore()?;
    Ok(())
}

pub(crate) fn handle_tool_call_update(tcu: sacp::schema::ToolCallUpdate, ctx: &mut TurnState<'_>) {
    let status = match tcu.fields.status {
        Some(ToolCallStatus::Completed) | Some(ToolCallStatus::Failed) => tcu.fields.status,
        _ => return,
    };

    let title = tcu.fields.title.as_deref().unwrap_or("tool");
    let (elapsed, args) = ctx
        .active_tools
        .remove(&tcu.tool_call_id)
        .map(|(t, a)| (t.elapsed(), a))
        .unwrap_or_default();

    if ctx.active_tools.is_empty() {
        ctx.mode.on_tools_empty();
    }

    let output = tcu.fields.raw_output.as_ref().and_then(|content| {
        content.as_str().map(|s| s.to_string()).or_else(|| {
            serde_json::to_string_pretty(content)
                .ok()
                .filter(|s| !s.is_empty())
        })
    });

    let output = output.unwrap_or_default();
    let number = store_tool_output(ctx, title, &output);

    if matches!(status, Some(ToolCallStatus::Completed)) {
        ctx.mode
            .render_tool_complete(title, elapsed, args.as_deref(), number);
        if !output.is_empty() {
            ctx.mode.render_tool_output(&output);
        }
    } else {
        ctx.mode.render_tool_failed(title, elapsed, number);
    }
}

/// Max stored tool outputs per session. Oldest are evicted when exceeded.
const MAX_STORED_OUTPUTS: usize = 200;
/// Max bytes per individual tool output. Larger outputs are tail-truncated.
const MAX_OUTPUT_BYTES: usize = 512 * 1024; // 512 KB

fn store_tool_output(ctx: &mut TurnState<'_>, title: &str, output: &str) -> Option<usize> {
    if ctx.mode.is_interactive() {
        Some(push_tool_output(
            ctx.tool_outputs,
            ctx.next_tool_id,
            title,
            output,
        ))
    } else {
        None
    }
}

/// Core storage logic, extracted for testability.
fn push_tool_output(
    outputs: &mut std::collections::VecDeque<StoredToolOutput>,
    next_id: &mut usize,
    title: &str,
    output: &str,
) -> usize {
    #[allow(clippy::string_slice)] // start is validated by is_char_boundary loop
    let stored = if output.len() > MAX_OUTPUT_BYTES {
        let mut start = output.len() - MAX_OUTPUT_BYTES;
        while !output.is_char_boundary(start) && start < output.len() {
            start += 1;
        }
        format!(
            "... (truncated, showing last {} bytes)\n{}",
            output.len() - start,
            &output[start..]
        )
    } else {
        output.to_string()
    };
    let id = *next_id;
    *next_id += 1;
    outputs.push_back(StoredToolOutput {
        id,
        title: title.to_string(),
        output: stored,
    });
    if outputs.len() > MAX_STORED_OUTPUTS {
        outputs.pop_front();
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn monotonic_ids() {
        let mut outputs = VecDeque::new();
        let mut next_id = 1;
        let id1 = push_tool_output(&mut outputs, &mut next_id, "shell", "output1");
        let id2 = push_tool_output(&mut outputs, &mut next_id, "read", "output2");
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(outputs.len(), 2);
    }

    #[test]
    fn eviction_preserves_monotonic_ids() {
        let mut outputs = VecDeque::new();
        let mut next_id = 1;
        // Fill past the cap
        for i in 0..MAX_STORED_OUTPUTS + 5 {
            let id = push_tool_output(&mut outputs, &mut next_id, "tool", &format!("out-{i}"));
            assert_eq!(id, i + 1, "ID should be monotonically increasing");
        }
        assert_eq!(outputs.len(), MAX_STORED_OUTPUTS);
        // First entry should be the 6th (IDs 1-5 evicted)
        assert_eq!(outputs.front().unwrap().id, 6);
        // Last entry should have the highest ID
        assert_eq!(outputs.back().unwrap().id, MAX_STORED_OUTPUTS + 5);
    }

    #[test]
    fn large_output_truncated() {
        let mut outputs = VecDeque::new();
        let mut next_id = 1;
        let big = "x".repeat(MAX_OUTPUT_BYTES + 100);
        push_tool_output(&mut outputs, &mut next_id, "tool", &big);
        let stored = &outputs[0].output;
        assert!(stored.starts_with("... (truncated"));
        assert!(stored.len() <= MAX_OUTPUT_BYTES + 100); // header + kept bytes
    }
}
