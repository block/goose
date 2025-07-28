use crate::message::{Message, MessageContent};
use rmcp::model::Role;
use std::collections::HashSet;

pub struct ConversationFixer;

const PLACEHOLDER_USER_MESSAGE: &str = "Hello";

impl ConversationFixer {
    /// Fix a conversation that we're about to send to an LLM. So the last and first
    /// messages should always be from the user.
    pub fn fix_conversation(messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
        let (messages, empty_removed) = Self::remove_empty_messages(messages);
        let (messages, tool_calling_fixed) = Self::fix_tool_calling(messages);
        let (messages, messages_merged) = Self::merge_consecutive_messages(messages);
        let (messages, lead_trail_fixed) = Self::fix_lead_trail(messages);
        let (messages, populated_if_empty) = Self::populate_if_empty(messages);

        let mut issues = Vec::new();
        issues.extend(empty_removed);
        issues.extend(tool_calling_fixed);
        issues.extend(messages_merged);
        issues.extend(lead_trail_fixed);
        issues.extend(populated_if_empty);

        (messages, issues)
    }

    fn remove_empty_messages(messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
        let mut issues = Vec::new();
        let filtered_messages = messages
            .into_iter()
            .filter(|msg| {
                if msg.content.is_empty() {
                    issues.push("Removed empty message".to_string());
                    false
                } else {
                    true
                }
            })
            .collect();
        (filtered_messages, issues)
    }

    fn fix_tool_calling(mut messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
        let mut issues = Vec::new();
        let mut pending_tool_requests: HashSet<String> = HashSet::new();

        for message in &mut messages {
            let mut content_to_remove = Vec::new();

            match message.role {
                Role::User => {
                    for (idx, content) in message.content.iter().enumerate() {
                        match content {
                            MessageContent::ToolRequest(req) => {
                                content_to_remove.push(idx);
                                issues.push(format!(
                                    "Removed tool request '{}' from user message",
                                    req.id
                                ));
                            }
                            MessageContent::ToolConfirmationRequest(req) => {
                                content_to_remove.push(idx);
                                issues.push(format!(
                                    "Removed tool confirmation request '{}' from user message",
                                    req.id
                                ));
                            }
                            MessageContent::Thinking(_) | MessageContent::RedactedThinking(_) => {
                                content_to_remove.push(idx);
                                issues
                                    .push("Removed thinking content from user message".to_string());
                            }
                            MessageContent::ToolResponse(resp) => {
                                if pending_tool_requests.contains(&resp.id) {
                                    pending_tool_requests.remove(&resp.id);
                                } else {
                                    content_to_remove.push(idx);
                                    issues.push(format!(
                                        "Removed orphaned tool response '{}'",
                                        resp.id
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Role::Assistant => {
                    for (idx, content) in message.content.iter().enumerate() {
                        match content {
                            MessageContent::ToolResponse(resp) => {
                                content_to_remove.push(idx);
                                issues.push(format!(
                                    "Removed tool response '{}' from assistant message",
                                    resp.id
                                ));
                            }
                            MessageContent::FrontendToolRequest(req) => {
                                content_to_remove.push(idx);
                                issues.push(format!(
                                    "Removed frontend tool request '{}' from assistant message",
                                    req.id
                                ));
                            }
                            MessageContent::ToolRequest(req) => {
                                pending_tool_requests.insert(req.id.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }

            for &idx in content_to_remove.iter().rev() {
                message.content.remove(idx);
            }
        }

        for message in &mut messages {
            if message.role == Role::Assistant {
                let mut content_to_remove = Vec::new();
                for (idx, content) in message.content.iter().enumerate() {
                    if let MessageContent::ToolRequest(req) = content {
                        if pending_tool_requests.contains(&req.id) {
                            content_to_remove.push(idx);
                            issues.push(format!("Removed orphaned tool request '{}'", req.id));
                        }
                    }
                }
                for &idx in content_to_remove.iter().rev() {
                    message.content.remove(idx);
                }
            }
        }
        let (messages, empty_removed) = Self::remove_empty_messages(messages);
        issues.extend(empty_removed);
        (messages, issues)
    }

    fn merge_consecutive_messages(messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
        let mut issues = Vec::new();
        let mut merged_messages: Vec<Message> = Vec::new();

        for message in messages {
            if let Some(last) = merged_messages.last_mut() {
                if last.role == message.role {
                    last.content.extend(message.content);
                    let role_name = match message.role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                    };
                    issues.push(format!("Merged consecutive {} messages", role_name));
                    continue;
                }
            }
            merged_messages.push(message);
        }

        (merged_messages, issues)
    }

    fn fix_lead_trail(mut messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
        let mut issues = Vec::new();

        if let Some(first) = messages.first() {
            if first.role == Role::Assistant {
                messages.remove(0);
                issues.push("Removed leading assistant message".to_string());
            }
        }

        if let Some(last) = messages.last() {
            if last.role == Role::Assistant {
                messages.pop();
                issues.push("Removed trailing assistant message".to_string());
            }
        }

        (messages, issues)
    }

    fn populate_if_empty(mut messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
        let mut issues = Vec::new();

        if messages.is_empty() {
            issues.push("Added placeholder user message to empty conversation".to_string());
            messages.push(Message::user().with_text(PLACEHOLDER_USER_MESSAGE));
        }
        (messages, issues)
    }
}

pub fn debug_conversation_fix(
    messages: &[Message],
    fixed: &[Message],
    issues: &[String],
) -> String {
    let mut output = String::new();

    output.push_str("=== CONVERSATION FIX DEBUG ===\n\n");

    output.push_str("BEFORE:\n");
    for (i, msg) in messages.iter().enumerate() {
        output.push_str(&format!("  [{}] {}\n", i, msg.debug()));
    }

    output.push_str("\nISSUES FOUND:\n");
    if issues.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for issue in issues {
            output.push_str(&format!("  - {}\n", issue));
        }
    }

    output.push_str("\nAFTER:\n");
    for (i, msg) in fixed.iter().enumerate() {
        output.push_str(&format!("  [{}] {}\n", i, msg.debug()));
    }

    output.push_str("\n==============================\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::tool::ToolCall;
    use serde_json::json;

    fn run_verify(messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
        let (fixed, issues) = ConversationFixer::fix_conversation(messages.clone());

        // Uncomment the following line to print the debug report
        // let report = debug_conversation_fix(&messages, &fixed, &issues);
        // print!("\n{}", report);

        let (_fixed, issues_with_fixed) = ConversationFixer::fix_conversation(fixed.clone());
        assert_eq!(
            issues_with_fixed.len(),
            0,
            "Fixed conversation should have no issues, but found: {:?}\n\n{}",
            issues_with_fixed,
            debug_conversation_fix(&messages, &fixed, &issues)
        );
        (fixed, issues)
    }

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

        let (fixed, issues) = run_verify(messages);

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

        let (fixed, issues) = run_verify(messages);

        assert_eq!(fixed.len(), 1);

        assert!(issues.iter().any(|i| i.contains("Removed empty message")));
        assert!(issues
            .iter()
            .any(|i| i.contains("Removed orphaned tool response 'wrong_id'")));

        assert_eq!(fixed[0].role, Role::User);
        assert_eq!(fixed[0].as_concat_text(), "Hello");
    }

    #[test]
    fn test_real_world_consecutive_assistant_messages() {
        let messages = vec![
            Message::user().with_text("run ls in the current directory and then run a word count on the smallest file"),
            Message::assistant()
                .with_text("I'll help you run `ls` in the current directory and then perform a word count on the smallest file. Let me start by listing the directory contents.")
                .with_tool_request("toolu_bdrk_018adWbP4X26CfoJU5hkhu3i", Ok(ToolCall::new("developer__shell", json!({"command": "ls -la"})))),
            Message::assistant()
                .with_text("Now I'll identify the smallest file by size. Looking at the output, I can see that both `slack.yaml` and `subrecipes.yaml` have a size of 0 bytes, making them the smallest files. I'll run a word count on one of them:")
                .with_tool_request("toolu_bdrk_01KgDYHs4fAodi22NqxRzmwx", Ok(ToolCall::new("developer__shell", json!({"command": "wc slack.yaml"})))),
            Message::user()
                .with_tool_response("toolu_bdrk_01KgDYHs4fAodi22NqxRzmwx", Ok(vec![])),
            Message::assistant()
                .with_text("I ran `ls -la` in the current directory and found several files. Looking at the file sizes, I can see that both `slack.yaml` and `subrecipes.yaml` are 0 bytes (the smallest files). I ran a word count on `slack.yaml` which shows: **0 lines**, **0 words**, **0 characters**"),
            Message::user().with_text("thanks!"),
        ];

        let (fixed, issues) = ConversationFixer::fix_conversation(messages);

        assert_eq!(fixed.len(), 5);
        assert_eq!(issues.len(), 2);
        assert!(issues[0].contains("Removed orphaned tool request"));
        assert!(issues[1].contains("Merged consecutive assistant messages"));
    }
}
