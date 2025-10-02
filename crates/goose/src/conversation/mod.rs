use crate::conversation::message::{Message, MessageContent, MessageMetadata};
use rmcp::model::Role;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;
use utoipa::ToSchema;

pub mod message;
mod tool_result_serde;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct Conversation(Vec<Message>);

#[derive(Error, Debug)]
#[error("invalid conversation: {reason}")]
pub struct InvalidConversation {
    reason: String,
    conversation: Conversation,
}

impl Conversation {
    pub fn new<I>(messages: I) -> Result<Self, InvalidConversation>
    where
        I: IntoIterator<Item = Message>,
    {
        Self::new_unvalidated(messages).validate()
    }

    pub fn new_unvalidated<I>(messages: I) -> Self
    where
        I: IntoIterator<Item = Message>,
    {
        Self(messages.into_iter().collect())
    }

    pub fn empty() -> Self {
        Self::new_unvalidated([])
    }

    /// Get all messages regardless of visibility.
    ///
    /// ⚠️ WARNING: This method should rarely be used directly. Consider:
    /// - `agent_visible_messages()` - For sending to LLM providers
    /// - `user_visible_messages()` - For displaying to users
    /// - Storage/persistence operations only
    ///
    /// If you're calling this for anything other than storage, you probably
    /// want one of the visibility-filtered methods instead.
    pub fn all_messages(&self) -> &Vec<Message> {
        &self.0
    }

    /// Deprecated: Use `all_messages()` and be explicit about visibility needs.
    /// This will be removed in a future version.
    #[deprecated(
        since = "1.9.0",
        note = "Use `all_messages()`, `agent_visible_messages()`, or `user_visible_messages()` to be explicit about visibility"
    )]
    pub fn messages(&self) -> &Vec<Message> {
        &self.0
    }

    pub fn push(&mut self, message: Message) {
        if let Some(last) = self
            .0
            .last_mut()
            .filter(|m| m.id.is_some() && m.id == message.id)
        {
            match (last.content.last_mut(), message.content.last()) {
                (Some(MessageContent::Text(ref mut last)), Some(MessageContent::Text(new)))
                    if message.content.len() == 1 =>
                {
                    last.text.push_str(&new.text);
                }
                (_, _) => {
                    last.content.extend(message.content);
                }
            }
        } else {
            self.0.push(message);
        }
    }

    pub fn last(&self) -> Option<&Message> {
        self.0.last()
    }

    pub fn first(&self) -> Option<&Message> {
        self.0.first()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Message>,
    {
        for message in iter {
            self.push(message);
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Message> {
        self.0.iter()
    }

    pub fn pop(&mut self) -> Option<Message> {
        self.0.pop()
    }

    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn filtered_messages<F>(&self, filter: F) -> Vec<Message>
    where
        F: Fn(&MessageMetadata) -> bool,
    {
        self.0
            .iter()
            .filter(|msg| filter(&msg.metadata))
            .cloned()
            .collect()
    }

    pub fn agent_visible_messages(&self) -> Vec<Message> {
        self.filtered_messages(|meta| meta.agent_visible)
    }

    pub fn user_visible_messages(&self) -> Vec<Message> {
        self.filtered_messages(|meta| meta.user_visible)
    }

    fn validate(self) -> Result<Self, InvalidConversation> {
        let (_messages, issues) = fix_messages(self.0.clone());
        if !issues.is_empty() {
            let reason = issues.join("\n");
            Err(InvalidConversation {
                reason,
                conversation: self,
            })
        } else {
            Ok(self)
        }
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::empty()
    }
}

impl IntoIterator for Conversation {
    type Item = Message;
    type IntoIter = std::vec::IntoIter<Message>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<'a> IntoIterator for &'a Conversation {
    type Item = &'a Message;
    type IntoIter = std::slice::Iter<'a, Message>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// Fix a conversation that we're about to send to an LLM. So the last and first
/// messages should always be from the user.
pub fn fix_conversation(conversation: Conversation) -> (Conversation, Vec<String>) {
    // fix_conversation operates on all messages passed to it (should be pre-filtered to agent_visible if needed)
    let messages = conversation.all_messages().clone();
    let (messages, issues) = fix_messages(messages);
    (Conversation::new_unvalidated(messages), issues)
}

fn fix_messages(messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
    let (messages_1, empty_removed) = remove_empty_messages(messages);
    let (messages_2, tool_calling_fixed) = fix_tool_calling(messages_1);
    let (messages_3, messages_merged) = merge_consecutive_messages(messages_2);
    let (messages_4, lead_trail_fixed) = fix_lead_trail(messages_3);
    let (messages_5, populated_if_empty) = populate_if_empty(messages_4);

    let mut issues = Vec::new();
    issues.extend(empty_removed);
    issues.extend(tool_calling_fixed);
    issues.extend(messages_merged);
    issues.extend(lead_trail_fixed);
    issues.extend(populated_if_empty);

    (messages_5, issues)
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
                            issues.push("Removed thinking content from user message".to_string());
                        }
                        MessageContent::ToolResponse(resp) => {
                            if pending_tool_requests.contains(&resp.id) {
                                pending_tool_requests.remove(&resp.id);
                            } else {
                                content_to_remove.push(idx);
                                issues
                                    .push(format!("Removed orphaned tool response '{}'", resp.id));
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
    let (messages, empty_removed) = remove_empty_messages(messages);
    issues.extend(empty_removed);
    (messages, issues)
}

fn merge_consecutive_messages(messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
    let mut issues = Vec::new();
    let mut merged_messages: Vec<Message> = Vec::new();

    for message in messages {
        if let Some(last) = merged_messages.last_mut() {
            let effective = effective_role(&message);
            if effective_role(last) == effective {
                last.content.extend(message.content);
                issues.push(format!("Merged consecutive {} messages", effective));
                continue;
            }
        }
        merged_messages.push(message);
    }

    (merged_messages, issues)
}

fn has_tool_response(message: &Message) -> bool {
    message
        .content
        .iter()
        .any(|content| matches!(content, MessageContent::ToolResponse(_)))
}

fn effective_role(message: &Message) -> String {
    if message.role == Role::User && has_tool_response(message) {
        "tool".to_string()
    } else {
        match message.role {
            Role::User => "user".to_string(),
            Role::Assistant => "assistant".to_string(),
        }
    }
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

const PLACEHOLDER_USER_MESSAGE: &str = "Hello";

fn populate_if_empty(mut messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
    let mut issues = Vec::new();

    if messages.is_empty() {
        issues.push("Added placeholder user message to empty conversation".to_string());
        messages.push(Message::user().with_text(PLACEHOLDER_USER_MESSAGE));
    }
    (messages, issues)
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
    use crate::conversation::message::{Message, MessageContent, MessageMetadata};
    use crate::conversation::{debug_conversation_fix, fix_conversation, Conversation};
    use rmcp::model::{CallToolRequestParam, Role};
    use rmcp::object;

    fn run_verify(messages: Vec<Message>) -> (Vec<Message>, Vec<String>) {
        let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages.clone()));

        // Uncomment the following line to print the debug report
        // let report = debug_conversation_fix(&messages, &fixed, &issues);
        // print!("\n{}", report);

        let (_fixed, issues_with_fixed) = fix_conversation(fixed.clone());
        assert_eq!(
            issues_with_fixed.len(),
            0,
            "Fixed conversation should have no issues, but found: {:?}\n\n{}",
            issues_with_fixed,
            debug_conversation_fix(&messages, fixed.all_messages(), &issues)
        );
        (fixed.all_messages().clone(), issues)
    }

    #[test]
    fn test_valid_conversation() {
        let all_messages = vec![
            Message::user().with_text("Can you help me search for something?"),
            Message::assistant()
                .with_text("I'll help you search.")
                .with_tool_request(
                    "search_1",
                    Ok(CallToolRequestParam {
                        name: "web_search".into(),
                        arguments: Some(object!({"query": "rust programming"})),
                    }),
                ),
            Message::user().with_tool_response("search_1", Ok(vec![])),
            Message::assistant().with_text("Based on the search results, here's what I found..."),
        ];

        for i in 1..=all_messages.len() {
            let messages = Conversation::new_unvalidated(all_messages[..i].to_vec());
            if messages.last().unwrap().role == Role::User {
                let (fixed, issues) = fix_conversation(messages.clone());
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
                assert_eq!(
                    fixed.all_messages(),
                    messages.all_messages(),
                    "Step {}: Messages should be unchanged",
                    i
                );
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
                .with_tool_request(
                    "bad_req",
                    Ok(CallToolRequestParam {
                        name: "search".into(),
                        arguments: Some(object!({})),
                    }),
                )
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
                .with_tool_request(
                    "search_1",
                    Ok(CallToolRequestParam {
                        name: "search".into(),
                        arguments: Some(object!({})),
                    }),
                ),
            Message::user(),
            Message::user().with_tool_response("wrong_id", Ok(vec![])),
            Message::assistant().with_tool_request(
                "search_2",
                Ok(CallToolRequestParam {
                    name: "search".into(),
                    arguments: Some(object!({})),
                }),
            ),
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
        let conversation = Conversation::new_unvalidated(vec![
            Message::user().with_text("run ls in the current directory and then run a word count on the smallest file"),

            Message::assistant()
                .with_text("I'll help you run `ls` in the current directory and then perform a word count on the smallest file. Let me start by listing the directory contents.")
                .with_tool_request("toolu_bdrk_018adWbP4X26CfoJU5hkhu3i", Ok(CallToolRequestParam { name: "developer__shell".into(), arguments: Some(object!({"command": "ls -la"})) })),

            Message::assistant()
                .with_text("Now I'll identify the smallest file by size. Looking at the output, I can see that both `slack.yaml` and `subrecipes.yaml` have a size of 0 bytes, making them the smallest files. I'll run a word count on one of them:")
                .with_tool_request("toolu_bdrk_01KgDYHs4fAodi22NqxRzmwx", Ok(CallToolRequestParam { name: "developer__shell".into(), arguments: Some(object!({"command": "wc slack.yaml"})) })),

            Message::user()
                .with_tool_response("toolu_bdrk_01KgDYHs4fAodi22NqxRzmwx", Ok(vec![])),

            Message::assistant()
                .with_text("I ran `ls -la` in the current directory and found several files. Looking at the file sizes, I can see that both `slack.yaml` and `subrecipes.yaml` are 0 bytes (the smallest files). I ran a word count on `slack.yaml` which shows: **0 lines**, **0 words**, **0 characters**"),
            Message::user().with_text("thanks!"),
        ]);

        let (fixed, issues) = fix_conversation(conversation);

        assert_eq!(fixed.len(), 5);
        assert_eq!(issues.len(), 2);
        assert!(issues[0].contains("Removed orphaned tool request"));
        assert!(issues[1].contains("Merged consecutive assistant messages"));
    }

    #[test]
    fn test_tool_response_effective_role() {
        let messages = vec![
            Message::user().with_text("Search for something"),
            Message::assistant()
                .with_text("I'll search for you")
                .with_tool_request(
                    "search_1",
                    Ok(CallToolRequestParam {
                        name: "search".into(),
                        arguments: Some(object!({})),
                    }),
                ),
            Message::user().with_tool_response("search_1", Ok(vec![])),
            Message::user().with_text("Thanks!"),
        ];

        let (_fixed, issues) = run_verify(messages);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_fix_conversation_with_mixed_visibility() {
        // Simulates the scenario after summarization where old messages are marked agent_visible=false
        // but a new tool_use is added that's agent_visible=true
        let messages = vec![
            // Old messages from before summarization (agent_visible=false, user_visible=true)
            Message::user()
                .with_text("Previous conversation")
                .with_metadata(MessageMetadata::user_only()),
            Message::assistant()
                .with_text("I'll help")
                .with_tool_request(
                    "old_tool",
                    Ok(CallToolRequestParam {
                        name: "old_search".into(),
                        arguments: Some(object!({})),
                    }),
                )
                .with_metadata(MessageMetadata::user_only()),
            Message::user()
                .with_tool_response("old_tool", Ok(vec![]))
                .with_metadata(MessageMetadata::user_only()),
            // Summary message (agent_visible=true, user_visible=false)
            Message::user()
                .with_text("Summary of conversation")
                .with_metadata(MessageMetadata::agent_only()),
            // New messages after summarization (agent_visible=true, user_visible=true)
            Message::user().with_text("New question"),
            Message::assistant()
                .with_text("Let me search")
                .with_tool_request(
                    "new_tool",
                    Ok(CallToolRequestParam {
                        name: "new_search".into(),
                        arguments: Some(object!({})),
                    }),
                ),
        ];

        // When we filter to agent_visible messages and fix, we should only see:
        // - Summary message
        // - New question
        // - Assistant with new_tool request (orphaned, so should be removed)
        let conversation = Conversation::new_unvalidated(messages);
        let agent_visible_messages = conversation.agent_visible_messages();
        let agent_conversation = Conversation::new_unvalidated(agent_visible_messages);

        let (fixed, issues) = fix_conversation(agent_conversation);

        // Should have removed the orphaned new_tool request
        assert!(
            issues.iter().any(|i| i.contains("Removed orphaned tool request 'new_tool'")),
            "Expected orphaned tool request to be removed. Issues: {:?}",
            issues
        );

        // Should have at least 1 message (after merging and cleanup)
        assert!(
            !fixed.is_empty(),
            "Fixed conversation should not be empty. Got {} messages",
            fixed.len()
        );

        // Verify no tool requests remain in the fixed conversation
        for msg in fixed.all_messages() {
            for content in &msg.content {
                assert!(
                    !matches!(content, MessageContent::ToolRequest(_)),
                    "Should not have any tool requests in fixed conversation"
                );
            }
        }

        // The key assertion: verify that when filtering by agent_visible and then fixing,
        // we don't send orphaned tool_use blocks to the provider
        println!("Fixed conversation has {} messages", fixed.len());
        println!("Issues found: {:?}", issues);
    }
}
