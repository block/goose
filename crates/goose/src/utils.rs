use crate::conversation::message::{Message, MessageContent};
use rmcp::model::{CallToolResult, Content};
use tokio_util::sync::CancellationToken;
use unicode_normalization::UnicodeNormalization;

/// Maximum number of characters to show for tool output in user-facing displays.
/// Content exceeding this is truncated with a message indicating the full length.
const MAX_TOOL_OUTPUT_DISPLAY_CHARS: usize = 10_000;

/// Check if a character is in the Unicode Tags Block range (U+E0000-U+E007F)
/// These characters are invisible and can be used for steganographic attacks
fn is_in_unicode_tag_range(c: char) -> bool {
    matches!(c, '\u{E0000}'..='\u{E007F}')
}

pub fn contains_unicode_tags(text: &str) -> bool {
    text.chars().any(is_in_unicode_tag_range)
}

/// Sanitize Unicode Tags Block characters from text
pub fn sanitize_unicode_tags(text: &str) -> String {
    let normalized: String = text.nfc().collect();

    normalized
        .chars()
        .filter(|&c| !is_in_unicode_tag_range(c))
        .collect()
}

/// Safely truncate a string at character boundaries, not byte boundaries
///
/// This function ensures that multi-byte UTF-8 characters (like Japanese, emoji, etc.)
/// are not split in the middle, which would cause a panic.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of characters to keep
///
/// # Returns
/// A truncated string with "..." appended if truncation occurred
pub fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Truncate tool output text for user display, preserving the original for agent processing.
/// Returns None if no truncation was needed.
pub fn truncate_tool_text_for_display(text: &str) -> Option<String> {
    let char_count = text.chars().count();
    if char_count <= MAX_TOOL_OUTPUT_DISPLAY_CHARS {
        return None;
    }
    let truncated: String = text.chars().take(MAX_TOOL_OUTPUT_DISPLAY_CHARS).collect();
    Some(format!(
        "{}\n\n... [output truncated: showing {MAX_TOOL_OUTPUT_DISPLAY_CHARS} of {char_count} characters]",
        truncated
    ))
}

/// Create a clone of a Message with tool response text truncated for user display.
/// Only modifies ToolResponse content; all other content types pass through unchanged.
pub fn truncate_message_for_display(message: &Message) -> Message {
    let mut any_changed = false;
    let truncated_content: Vec<MessageContent> = message
        .content
        .iter()
        .map(|c| match c {
            MessageContent::ToolResponse(resp) => match &resp.tool_result {
                Ok(result) => {
                    let mut content_changed = false;
                    let new_content: Vec<Content> = result
                        .content
                        .iter()
                        .map(|content| {
                            if let Some(text) = content.as_text() {
                                if let Some(truncated) =
                                    truncate_tool_text_for_display(&text.text)
                                {
                                    content_changed = true;
                                    return Content::text(truncated);
                                }
                            }
                            content.clone()
                        })
                        .collect();
                    if !content_changed {
                        return c.clone();
                    }
                    any_changed = true;
                    MessageContent::ToolResponse(crate::conversation::message::ToolResponse {
                        id: resp.id.clone(),
                        tool_result: Ok(CallToolResult {
                            content: new_content,
                            ..result.clone()
                        }),
                        metadata: resp.metadata.clone(),
                    })
                }
                Err(_) => c.clone(),
            },
            _ => c.clone(),
        })
        .collect();

    if !any_changed {
        return message.clone();
    }

    Message {
        content: truncated_content,
        ..message.clone()
    }
}

pub fn is_token_cancelled(cancellation_token: &Option<CancellationToken>) -> bool {
    cancellation_token
        .as_ref()
        .is_some_and(|t| t.is_cancelled())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_unicode_tags() {
        // Test detection of Unicode Tags Block characters
        assert!(contains_unicode_tags("Hello\u{E0041}world"));
        assert!(contains_unicode_tags("\u{E0000}"));
        assert!(contains_unicode_tags("\u{E007F}"));
        assert!(!contains_unicode_tags("Hello world"));
        assert!(!contains_unicode_tags("Hello 世界 🌍"));
        assert!(!contains_unicode_tags(""));
    }

    #[test]
    fn test_sanitize_unicode_tags() {
        // Test that Unicode Tags Block characters are removed
        let malicious = "Hello\u{E0041}\u{E0042}\u{E0043}world"; // Invisible "ABC"
        let cleaned = sanitize_unicode_tags(malicious);
        assert_eq!(cleaned, "Helloworld");
    }

    #[test]
    fn test_sanitize_unicode_tags_preserves_legitimate_unicode() {
        // Test that legitimate Unicode characters are preserved
        let clean_text = "Hello world 世界 🌍";
        let cleaned = sanitize_unicode_tags(clean_text);
        assert_eq!(cleaned, clean_text);
    }

    #[test]
    fn test_sanitize_unicode_tags_empty_string() {
        let empty = "";
        let cleaned = sanitize_unicode_tags(empty);
        assert_eq!(cleaned, "");
    }

    #[test]
    fn test_sanitize_unicode_tags_only_malicious() {
        // Test string containing only Unicode Tags characters
        let only_malicious = "\u{E0041}\u{E0042}\u{E0043}";
        let cleaned = sanitize_unicode_tags(only_malicious);
        assert_eq!(cleaned, "");
    }

    #[test]
    fn test_sanitize_unicode_tags_mixed_content() {
        // Test mixed legitimate and malicious Unicode
        let mixed = "Hello\u{E0041} 世界\u{E0042} 🌍\u{E0043}!";
        let cleaned = sanitize_unicode_tags(mixed);
        assert_eq!(cleaned, "Hello 世界 🌍!");
    }

    #[test]
    fn test_safe_truncate_ascii() {
        assert_eq!(safe_truncate("hello world", 20), "hello world");
        assert_eq!(safe_truncate("hello world", 8), "hello...");
        assert_eq!(safe_truncate("hello", 5), "hello");
        assert_eq!(safe_truncate("hello", 3), "...");
    }

    #[test]
    fn test_safe_truncate_japanese() {
        // Japanese characters: "こんにちは世界" (Hello World)
        let japanese = "こんにちは世界";
        assert_eq!(safe_truncate(japanese, 10), japanese);
        assert_eq!(safe_truncate(japanese, 5), "こん...");
        assert_eq!(safe_truncate(japanese, 7), japanese);
    }

    #[test]
    fn test_safe_truncate_mixed() {
        // Mixed ASCII and Japanese
        let mixed = "Hello こんにちは";
        assert_eq!(safe_truncate(mixed, 20), mixed);
        assert_eq!(safe_truncate(mixed, 8), "Hello...");
    }

    #[test]
    fn test_truncate_tool_text_short() {
        let short = "Hello world";
        assert!(truncate_tool_text_for_display(short).is_none());
    }

    #[test]
    fn test_truncate_tool_text_long() {
        let long: String = "x".repeat(20_000);
        let result = truncate_tool_text_for_display(&long).unwrap();
        assert!(result.contains("output truncated"));
        assert!(result.contains("10000 of 20000 characters"));
        // Should start with the first 10000 chars
        assert!(result.starts_with(&"x".repeat(10_000)));
    }

    #[test]
    fn test_truncate_message_no_tool_response() {
        use rmcp::model::Role;
        let msg = Message::new(Role::Assistant, 0, vec![MessageContent::text("Hello")]);
        let truncated = truncate_message_for_display(&msg);
        assert_eq!(truncated.content.len(), 1);
        assert_eq!(truncated.as_concat_text(), "Hello");
    }

    #[test]
    fn test_truncate_message_with_long_tool_response() {
        use rmcp::model::{CallToolResult, Content, Role};
        let long_text: String = "line\n".repeat(5000);
        let tool_resp = MessageContent::tool_response(
            "test-id".to_string(),
            Ok(CallToolResult::success(vec![Content::text(long_text.clone())])),
        );
        let msg = Message::new(Role::Assistant, 0, vec![tool_resp]);
        let truncated = truncate_message_for_display(&msg);
        assert_eq!(truncated.content.len(), 1);
        if let MessageContent::ToolResponse(resp) = &truncated.content[0] {
            let result = resp.tool_result.as_ref().unwrap();
            let text = result.content[0].as_text().unwrap();
            assert!(text.text.contains("output truncated"));
            assert!(text.text.len() < long_text.len());
        } else {
            panic!("Expected ToolResponse");
        }
    }

    #[test]
    fn test_truncate_message_short_tool_response_unchanged() {
        use rmcp::model::{CallToolResult, Content, Role};
        let tool_resp = MessageContent::tool_response(
            "test-id".to_string(),
            Ok(CallToolResult::success(vec![Content::text("short output")])),
        );
        let msg = Message::new(Role::Assistant, 0, vec![tool_resp]);
        let truncated = truncate_message_for_display(&msg);
        if let MessageContent::ToolResponse(resp) = &truncated.content[0] {
            let result = resp.tool_result.as_ref().unwrap();
            let text = result.content[0].as_text().unwrap();
            assert_eq!(text.text, "short output");
        } else {
            panic!("Expected ToolResponse");
        }
    }
}
