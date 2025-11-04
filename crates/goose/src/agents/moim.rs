use crate::agents::extension_manager::ExtensionManager;
use crate::agents::SessionConfig;
use crate::conversation::message::{Message, MessageContent};
use uuid::Uuid;

/// Inject MOIM (Minus One Info Message) into conversation.
///
/// MOIM provides ephemeral context that's included in LLM calls
/// but never persisted to conversation history.
pub async fn inject_moim(
    messages: &[Message],
    extension_manager: &ExtensionManager,
    _session: &Option<SessionConfig>,
) -> Vec<Message> {
    let moim_content = match extension_manager.collect_moim().await {
        Some(content) if !content.trim().is_empty() => content,
        _ => {
            tracing::debug!("No MOIM content available");
            return messages.to_vec();
        }
    };

    tracing::debug!("Injecting MOIM: {} chars", moim_content.len());

    let moim_message = Message::user()
        .with_text(moim_content)
        .with_id(format!("moim_{}", Uuid::new_v4()))
        .agent_only();

    let mut messages_with_moim = messages.to_vec();

    if messages_with_moim.is_empty() {
        messages_with_moim.push(moim_message);
    } else {
        let insert_pos = find_moim_insertion_point(&messages_with_moim);
        messages_with_moim.insert(insert_pos, moim_message);
    }

    messages_with_moim
}

/// Find a safe insertion point for MOIM that won't break tool call/response pairs.
fn find_moim_insertion_point(messages: &[Message]) -> usize {
    if messages.is_empty() {
        return 0;
    }

    let last_pos = messages.len() - 1;

    // Don't break tool call/response pairs
    if last_pos > 0 {
        let prev_msg = &messages[last_pos - 1];
        let curr_msg = &messages[last_pos];

        let prev_has_tool_calls = prev_msg
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(_)));

        let curr_has_tool_responses = curr_msg
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolResponse(_)));

        if prev_has_tool_calls && curr_has_tool_responses {
            tracing::debug!("MOIM: Adjusting position to avoid breaking tool pair");
            return last_pos.saturating_sub(1);
        }
    }

    last_pos
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{ToolRequest, ToolResponse};
    use rmcp::model::{CallToolRequestParam, Content};
    use rmcp::object;
    use std::sync::Arc;

    #[test]
    fn test_find_insertion_point_edge_cases() {
        // Test empty messages
        let messages = vec![];
        assert_eq!(find_moim_insertion_point(&messages), 0);

        // Test single message - should return 0 (insert at beginning)
        let messages = vec![Message::user().with_text("Hello")];
        assert_eq!(find_moim_insertion_point(&messages), 0);

        // Test multiple messages - should return last position
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi"),
            Message::user().with_text("How are you?"),
        ];
        assert_eq!(find_moim_insertion_point(&messages), 2);
    }

    #[test]
    fn test_find_insertion_point_tool_pair_detection() {
        // Helper to create tool request and response
        let tool_request = ToolRequest {
            id: "test_tool_1".to_string(),
            tool_call: Ok(CallToolRequestParam {
                name: "test_tool".into(),
                arguments: Some(object!({"key": "value"})),
            }),
        };

        let tool_response = ToolResponse {
            id: "test_tool_1".to_string(),
            tool_result: Ok(vec![Content::text("Tool executed successfully")]),
        };

        // Test: Tool call/response pair at the end - should back up
        let messages = vec![
            Message::user().with_text("Please use a tool"),
            Message::assistant()
                .with_text("I'll use the tool now.")
                .with_content(MessageContent::ToolRequest(tool_request.clone())),
            Message::user().with_content(MessageContent::ToolResponse(tool_response.clone())),
        ];
        assert_eq!(find_moim_insertion_point(&messages), 1); // Should back up to position 1

        // Test: Tool pair in the middle with more messages after
        let messages_with_more = vec![
            Message::user().with_text("First request"),
            Message::assistant().with_content(MessageContent::ToolRequest(tool_request.clone())),
            Message::user().with_content(MessageContent::ToolResponse(tool_response)),
            Message::assistant().with_text("Tool completed, here's the result"),
        ];
        assert_eq!(find_moim_insertion_point(&messages_with_more), 3); // Should use last position
    }

    #[test]
    fn test_find_insertion_point_non_tool_pairs() {
        let tool_request = ToolRequest {
            id: "test_tool_1".to_string(),
            tool_call: Ok(CallToolRequestParam {
                name: "test_tool".into(),
                arguments: Some(object!({"key": "value"})),
            }),
        };

        let tool_response = ToolResponse {
            id: "test_tool_1".to_string(),
            tool_result: Ok(vec![Content::text("Tool executed")]),
        };

        // Test: Tool request without matching response
        let messages = vec![
            Message::user().with_text("Please use a tool"),
            Message::assistant().with_content(MessageContent::ToolRequest(tool_request)),
            Message::assistant().with_text("Actually, let me reconsider."),
        ];
        assert_eq!(find_moim_insertion_point(&messages), 2); // No pair, use last position

        // Test: Tool response without preceding request
        let messages = vec![
            Message::user().with_text("Here's a tool response from earlier"),
            Message::assistant().with_text("Okay, I see that."),
            Message::user().with_content(MessageContent::ToolResponse(tool_response)),
        ];
        assert_eq!(find_moim_insertion_point(&messages), 2); // No pair, use last position
    }

    #[tokio::test]
    async fn test_moim_injection_basic() {
        let provider = Arc::new(tokio::sync::Mutex::new(None));
        let extension_manager = ExtensionManager::new(provider);

        // Test empty conversation
        let messages = vec![];
        let result = inject_moim(&messages, &extension_manager, &None).await;
        assert_eq!(result.len(), 1);
        assert!(result[0].id.as_ref().unwrap().starts_with("moim_"));

        // Verify MOIM content and metadata
        let content = result[0].content.first().and_then(|c| c.as_text()).unwrap();
        assert!(content.contains("<info-msg>"));
        assert!(content.contains("Datetime:"));
        assert!(!result[0].is_user_visible());
        assert!(result[0].is_agent_visible());

        // Test with existing messages
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there"),
        ];
        let result = inject_moim(&messages, &extension_manager, &None).await;

        assert_eq!(result.len(), 3);
        // MOIM should be at position 1 (before the last message)
        assert!(result[1].id.as_ref().unwrap().starts_with("moim_"));
        assert!(!result[1].is_user_visible());
        assert!(result[1].is_agent_visible());
    }

    #[tokio::test]
    async fn test_moim_injection_preserves_tool_pair() {
        let tool_request = ToolRequest {
            id: "test_tool_1".to_string(),
            tool_call: Ok(CallToolRequestParam {
                name: "test_tool".into(),
                arguments: Some(object!({"key": "value"})),
            }),
        };

        let tool_response = ToolResponse {
            id: "test_tool_1".to_string(),
            tool_result: Ok(vec![Content::text("Tool executed successfully")]),
        };

        let messages = vec![
            Message::user().with_text("Please use a tool"),
            Message::assistant()
                .with_text("I'll use the tool now.")
                .with_content(MessageContent::ToolRequest(tool_request)),
            Message::user().with_content(MessageContent::ToolResponse(tool_response)),
        ];

        let provider = Arc::new(tokio::sync::Mutex::new(None));
        let extension_manager = ExtensionManager::new(provider);
        let result = inject_moim(&messages, &extension_manager, &None).await;

        // Should have 4 messages total (3 original + 1 MOIM)
        assert_eq!(result.len(), 4);

        // MOIM should be at position 1 (before the tool request/response pair)
        assert!(result[1].id.as_ref().unwrap().starts_with("moim_"));

        // Verify the tool pair is still together (positions 2 and 3)
        assert!(result[2]
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(_))));
        assert!(result[3]
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolResponse(_))));
    }
}
