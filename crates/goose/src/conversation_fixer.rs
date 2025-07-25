use crate::message::{Message, MessageContent};
use rmcp::model::Role;
use std::collections::HashSet;

pub struct ConversationFixer;

impl ConversationFixer {
    /// Fix a conversation that we're about to send to an LLM. So the last and first
    /// messages should always be from the user.
    pub fn fix_conversation(messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
        let mut fixed_messages: std::vec::Vec<Message> = Vec::new();
        let mut issues = Vec::new();
        let mut pending_tool_requests: HashSet<String> = HashSet::new();

        for message in messages {
            let mut fixed_message = message.clone();
            let mut message_issues = Vec::new();

            if fixed_message.content.is_empty() {
                issues.push("Removed empty message".to_string());
                continue;
            }

            if let Some(last_msg) = fixed_messages.last_mut() {
                if last_msg.role == fixed_message.role {
                    last_msg.content.extend(fixed_message.content);
                    let role_name = match fixed_message.role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                    };
                    issues.push(format!("Merged consecutive {} messages", role_name));
                    continue;
                }
            }

            let mut content_to_remove = Vec::new();
            let mut check_content =
                |idx: usize, should_remove: bool, content_name: String, role_name: &str| {
                    if should_remove {
                        content_to_remove.push(idx);
                        message_issues.push(format!(
                            "Removed {} from {} message",
                            content_name, role_name
                        ));
                    }
                };

            match fixed_message.role {
                Role::User => {
                    for (idx, content) in fixed_message.content.iter().enumerate() {
                        let (should_remove, content_name) = match content {
                            MessageContent::ToolRequest(req) => {
                                (true, format!("tool request '{}'", req.id))
                            }
                            MessageContent::ToolConfirmationRequest(req) => {
                                (true, format!("tool confirmation request '{}'", req.id))
                            }
                            MessageContent::Thinking(_) => (true, "thinking content".to_string()),
                            MessageContent::RedactedThinking(_) => {
                                (true, "redacted thinking content".to_string())
                            }
                            MessageContent::ToolResponse(resp) => {
                                if !pending_tool_requests.contains(&resp.id) {
                                    (true, format!("orphaned tool response '{}'", resp.id))
                                } else {
                                    pending_tool_requests.remove(&resp.id);
                                    (false, String::new())
                                }
                            }
                            _ => (false, String::new()),
                        };

                        check_content(idx, should_remove, content_name, "user");
                    }
                }
                Role::Assistant => {
                    for (idx, content) in fixed_message.content.iter().enumerate() {
                        let (should_remove, content_name) = match content {
                            MessageContent::ToolResponse(resp) => {
                                (true, format!("tool response '{}'", resp.id))
                            }
                            MessageContent::FrontendToolRequest(req) => {
                                (true, format!("frontend tool request '{}'", req.id))
                            }
                            MessageContent::ToolRequest(req) => {
                                pending_tool_requests.insert(req.id.clone());
                                (false, String::new())
                            }
                            _ => (false, String::new()),
                        };

                        check_content(idx, should_remove, content_name, "assistant");
                    }
                }
            }
            for &idx in content_to_remove.iter().rev() {
                fixed_message.content.remove(idx);
            }
            if fixed_message.content.is_empty() {
                message_issues.push("Removed message after content filtering".to_string());
                issues.extend(message_issues);
                continue;
            }

            if fixed_messages.is_empty() && fixed_message.role != Role::User {
                issues.push(
                    "Conversation should start with user message, prepending placeholder"
                        .to_string(),
                );
                let placeholder = Message::user().with_text("Hello");
                fixed_messages.push(placeholder);
            }

            issues.extend(message_issues);
            fixed_messages.push(fixed_message);
        }

        for unmatched_id in pending_tool_requests {
            issues.push(format!(
                "Tool request '{}' has no corresponding response",
                unmatched_id
            ));
        }

        if let Some(last_msg) = fixed_messages.last() {
            if last_msg.role == Role::Assistant {
                fixed_messages.pop();
                issues.push("Removed trailing assistant message".to_string());
            }
        }
        (fixed_messages, issues)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::tool::ToolCall;
    use serde_json::json;

    #[test]
    fn test_valid_conversation() {
        let all_messages = vec![
            Message::user().with_text("Can you help me search for something?"),
            Message::assistant()
                .with_text("I'll help you search.")
                .with_tool_request(
                    "search_1",
                    Ok(ToolCall::new(
                        "web_search",
                        json!({"query": "rust programming"}),
                    )),
                ),
            Message::user().with_tool_response("search_1", Ok(vec![])),
            Message::assistant().with_text("Based on the search results, here's what I found..."),
        ];

        for i in 1..=all_messages.len() {
            let messages = all_messages[..i].to_vec();
            if messages.last().unwrap().role == Role::User {
                let (fixed, issues) = ConversationFixer::fix_conversation(messages.clone());
                assert_eq!(
                    fixed.len(),
                    messages.len(),
                    "Step {}: Length should match",
                    i
                );
                assert!(
                    issues.is_empty(),
                    "Step {}: Should have no issues, but found: {:?}",
                    i,
                    issues
                );
                assert_eq!(fixed, messages, "Step {}: Messages should be unchanged", i);
            }
        }
    }

    #[test]
    fn test_role_alternation_and_content_placement_issues() {
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::user().with_text("Another user message"),
            Message::assistant()
                .with_text("Response")
                .with_tool_response("orphan_1", Ok(vec![])), // Wrong role
            Message::assistant().with_thinking("Let me think", "sig"),
            Message::user()
                .with_tool_request("bad_req", Ok(ToolCall::new("search", json!({}))))
                .with_text("User with bad tool request"),
        ];

        let (fixed, issues) = ConversationFixer::fix_conversation(messages);

        assert_eq!(fixed.len(), 3);
        assert_eq!(issues.len(), 4);

        assert!(issues
            .iter()
            .any(|i| i.contains("Merged consecutive user messages")));
        assert!(issues
            .iter()
            .any(|i| i.contains("Removed tool response 'orphan_1' from assistant message")));
        assert!(issues
            .iter()
            .any(|i| i.contains("Removed tool request 'bad_req' from user message")));

        assert_eq!(fixed[0].role, Role::User);
        assert_eq!(fixed[1].role, Role::Assistant);
        assert_eq!(fixed[2].role, Role::User);

        assert_eq!(fixed[0].content.len(), 2);
    }

    #[test]
    fn test_orphaned_tools_and_empty_messages() {
        // This conversation completely collapses. the first user message is invalid
        // then we remove the empty user message and the wrong tool response
        // then we collapse the assistant messages
        // which we then remove because you can't end a conversation with an assistant message
        let messages = vec![
            Message::assistant()
                .with_text("I'll search for you")
                .with_tool_request("search_1", Ok(ToolCall::new("search", json!({})))),
            Message::user(),
            Message::user().with_tool_response("wrong_id", Ok(vec![])),
            Message::assistant()
                .with_tool_request("search_2", Ok(ToolCall::new("search", json!({})))),
        ];

        let (fixed, issues) = ConversationFixer::fix_conversation(messages);

        assert_eq!(fixed.len(), 1);
        assert_eq!(issues.len(), 7);

        assert!(issues
            .iter()
            .any(|i| i.contains("Conversation should start with user message")));
        assert!(issues.iter().any(|i| i.contains("Removed empty message")));
        assert!(issues
            .iter()
            .any(|i| i.contains("Removed orphaned tool response 'wrong_id'")));

        assert_eq!(fixed[0].role, Role::User);
        assert_eq!(fixed[0].as_concat_text(), "Hello");
    }
}
