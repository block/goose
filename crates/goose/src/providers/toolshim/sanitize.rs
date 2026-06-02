use super::*;

#[allow(clippy::string_slice)] // Marker constants are ASCII; byte indexing is safe.
pub fn strip_tokenized_tool_markup(content: &str) -> String {
    let mut stripped = content.to_string();

    while let Some(section_start) = stripped.find(TOOL_CALLS_SECTION_BEGIN) {
        let after_start = section_start + TOOL_CALLS_SECTION_BEGIN.len();
        if let Some(section_end_rel) = stripped[after_start..].find(TOOL_CALLS_SECTION_END) {
            let section_end = after_start + section_end_rel + TOOL_CALLS_SECTION_END.len();
            stripped.replace_range(section_start..section_end, "");
        } else {
            stripped.replace_range(section_start..stripped.len(), "");
            break;
        }
    }

    for marker in [
        TOOL_CALL_BEGIN,
        TOOL_CALL_ARGUMENT_BEGIN,
        TOOL_CALL_ARGUMENT_END,
        TOOL_CALL_END,
        TOOL_CALLS_SECTION_BEGIN,
        TOOL_CALLS_SECTION_END,
    ] {
        stripped = stripped.replace(marker, " ");
    }

    stripped.trim().to_string()
}

pub fn append_tool_calls_to_message(
    mut message: Message,
    tool_calls: Vec<CallToolRequestParams>,
) -> Message {
    for tool_call in tool_calls {
        if tool_call.name != "noop" {
            let id = Uuid::new_v4().to_string();
            message = message.with_tool_request(id, Ok(tool_call));
        }
    }
    message
}

pub fn sanitize_message_after_tokenized_parse(mut message: Message) -> Message {
    for content in &mut message.content {
        if let MessageContent::Text(text) = content {
            text.text = strip_tokenized_tool_markup(&text.text);
        }
    }

    message.content.retain(|content| match content {
        MessageContent::Text(text) => !text.text.trim().is_empty(),
        _ => true,
    });

    message
}

pub fn sanitize_message_after_json_tool_parse(mut message: Message) -> Message {
    for content in &mut message.content {
        if let MessageContent::Text(text) = content {
            let lower = text.text.to_ascii_lowercase();
            let looks_like_tool_directive = lower.contains("using tool:")
                || (text.text.contains("\"name\"") && text.text.contains("\"arguments\""));

            if looks_like_tool_directive {
                text.text.clear();
            }
        }
    }

    message.content.retain(|content| match content {
        MessageContent::Text(text) => !text.text.trim().is_empty(),
        _ => true,
    });

    message
}

/// Returns `true` if the text contains any raw tool-use markers that should
/// never appear in final assistant output.
pub fn has_tool_markers(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    for marker in [
        TOOL_CALLS_SECTION_BEGIN,
        TOOL_CALLS_SECTION_END,
        TOOL_CALL_BEGIN,
        TOOL_CALL_ARGUMENT_BEGIN,
        TOOL_CALL_ARGUMENT_END,
        TOOL_CALL_END,
    ] {
        if text.contains(marker) {
            return true;
        }
    }
    lower.contains("using tool:") || (text.contains("\"name\"") && text.contains("\"arguments\""))
}

/// Catch-all sanitization applied to every message leaving the toolshim
/// pipeline, regardless of whether tool-call parsing succeeded.
pub fn sanitize_residual_markers(mut message: Message) -> Message {
    let mut changed = false;
    for content in &mut message.content {
        if let MessageContent::Text(text) = content {
            if has_tool_markers(&text.text) {
                // Strip tokenized markers first (handles section blocks)
                text.text = strip_tokenized_tool_markup(&text.text);
                // Then clear any remaining JSON-style tool directives
                let lower = text.text.to_ascii_lowercase();
                if lower.contains("using tool:")
                    || (text.text.contains("\"name\"") && text.text.contains("\"arguments\""))
                {
                    text.text.clear();
                }
                changed = true;
            }
        }
    }
    if changed {
        message.content.retain(|content| match content {
            MessageContent::Text(text) => !text.text.trim().is_empty(),
            _ => true,
        });
    }
    message
}
