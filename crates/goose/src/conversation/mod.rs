use crate::conversation::message::{Message, MessageContent, MessageMetadata};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

mod fix;
pub mod message;
mod tool_result_serde;
pub use fix::*;

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

    pub fn messages(&self) -> &Vec<Message> {
        &self.0
    }

    pub fn push(&mut self, message: Message) {
        if message.content.is_empty() && message.metadata.inference.is_some() {
            if let Some(existing) = self
                .0
                .iter_mut()
                .rev()
                .find(|m| m.role == message.role && m.is_user_visible())
            {
                existing.metadata.inference = message.metadata.inference;
            }
            return;
        }

        if let Some(last) = self
            .0
            .last_mut()
            .filter(|m| m.id.is_some() && m.id == message.id)
        {
            if message.metadata.inference.is_some() {
                last.metadata.inference = message.metadata.inference.clone();
            }
            match (last.content.last_mut(), message.content.last()) {
                (Some(MessageContent::Text(ref mut last)), Some(MessageContent::Text(new)))
                    if message.content.len() == 1 =>
                {
                    last.text.push_str(&new.text);
                }
                (
                    Some(MessageContent::Thinking(ref mut last)),
                    Some(MessageContent::Thinking(new)),
                ) if message.content.len() == 1
                    && (last.signature.is_empty() || new.signature == last.signature) =>
                {
                    // Merge cases:
                    //   - `last` is still unsigned (block in progress) — append
                    //     and adopt `new.signature` if it's the closing delta.
                    //   - signatures match — same block continuing.
                    // An unsigned delta arriving after a signed block belongs
                    // to the next block (signature-at-end streams emit the
                    // first text of block N+1 before its signature), so the
                    // outer match arm falls through to push it separately.
                    last.thinking.push_str(&new.thinking);
                    if !new.signature.is_empty() {
                        last.signature = new.signature.clone();
                    }
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

/// Fix a conversation that we're about to send to an LLM. So the first and last
/// messages should always be from the user.
pub fn fix_conversation(conversation: Conversation) -> (Conversation, Vec<String>) {
    let all_messages = conversation.messages();

    // Create a shadow map: track each message as either Visible or NonVisible with its index
    enum MessageSlot {
        Visible(usize),      // Index into agent_visible_messages
        NonVisible(Message), // Non-visible messages pass through unchanged
    }

    let mut agent_visible_messages = Vec::new();
    let shadow_map: Vec<MessageSlot> = all_messages
        .iter()
        .map(|msg| {
            if msg.metadata.agent_visible {
                let idx = agent_visible_messages.len();
                agent_visible_messages.push(msg.clone());
                MessageSlot::Visible(idx)
            } else {
                MessageSlot::NonVisible(msg.clone())
            }
        })
        .collect();

    // Fix only the agent-visible messages
    let (fixed_visible, issues) = fix_messages(agent_visible_messages);

    // Reconstruct using shadow map: replace Visible slots with fixed messages
    let final_messages: Vec<Message> = shadow_map
        .into_iter()
        .filter_map(|slot| match slot {
            MessageSlot::Visible(idx) => fixed_visible.get(idx).cloned(),
            MessageSlot::NonVisible(msg) => Some(msg),
        })
        .collect();

    (Conversation::new_unvalidated(final_messages), issues)
}

#[cfg(test)]
mod tests {
    use crate::conversation::message::Message;
    use crate::conversation::{debug_conversation_fix, fix_conversation, Conversation};
    use rmcp::model::{CallToolRequestParams, Role};
    use rmcp::object;

    macro_rules! assert_has_issues_unordered {
        ($fixed:expr, $issues:expr, $($expected:expr),+ $(,)?) => {
            {
                let mut expected: Vec<&str> = vec![$($expected),+];
                let mut actual: Vec<&str> = $issues.iter().map(|s| s.as_str()).collect();
                expected.sort();
                actual.sort();

                if actual != expected {
                    panic!(
                        "assertion failed: issues don't match\nexpected: {:?}\n  actual: {:?}. Fixed conversation is:\n{:#?}",
                        expected, $issues, $fixed,
                    );
                }
            }
        };
    }

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
            debug_conversation_fix(&messages, fixed.messages(), &issues)
        );
        (fixed.messages().clone(), issues)
    }

    #[test]
    fn test_valid_conversation() {
        use rmcp::model::Content;

        let all_messages = [
            Message::user().with_text("Can you help me search for something?"),
            Message::assistant()
                .with_text("I'll help you search.")
                .with_tool_request(
                    "search_1",
                    Ok(CallToolRequestParams::new("web_search")
                        .with_arguments(object!({"query": "rust programming"}))),
                ),
            Message::user().with_tool_response(
                "search_1",
                Ok(rmcp::model::CallToolResult::success(vec![Content::text(
                    "Search results here",
                )])),
            ),
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
                    fixed.messages(),
                    messages.messages(),
                    "Step {}: Messages should be unchanged",
                    i
                );
            }
        }
    }

    #[test]
    fn test_role_alternation_and_content_placement_issues() {
        use rmcp::model::Content;

        let messages = vec![
            Message::user().with_text("Hello"),
            Message::user().with_text("Another user message"),
            Message::assistant()
                .with_text("Response")
                .with_tool_response(
                    "orphan_1",
                    Ok(rmcp::model::CallToolResult::success(vec![Content::text(
                        "result",
                    )])),
                ), // Wrong role
            Message::assistant().with_thinking("Let me think", "sig"),
            Message::user()
                .with_tool_request(
                    "bad_req",
                    Ok(CallToolRequestParams::new("search").with_arguments(object!({}))),
                )
                .with_text("User with bad tool request"),
        ];

        let (fixed, issues) = run_verify(messages);

        assert_eq!(fixed.len(), 3);

        assert_has_issues_unordered!(
            fixed,
            issues,
            "Merged consecutive assistant messages",
            "Merged consecutive user messages",
            "Removed tool response 'orphan_1' from assistant message",
            "Removed tool request 'bad_req' from user message",
        );

        assert_eq!(fixed[0].role, Role::User);
        assert_eq!(fixed[1].role, Role::Assistant);
        assert_eq!(fixed[2].role, Role::User);

        assert_eq!(fixed[0].content.len(), 2);
    }

    #[test]
    fn test_orphaned_tools_and_empty_messages() {
        use rmcp::model::Content;

        // This conversation completely collapses. the first user message is invalid
        // then we remove the empty user message and the wrong tool response
        // then we collapse the assistant messages
        // which we then remove because you can't end a conversation with an assistant message
        let messages = vec![
            Message::assistant()
                .with_text("I'll search for you")
                .with_tool_request(
                    "search_1",
                    Ok(CallToolRequestParams::new("search").with_arguments(object!({}))),
                ),
            Message::user(),
            Message::user().with_tool_response(
                "wrong_id",
                Ok(rmcp::model::CallToolResult::success(vec![Content::text(
                    "result",
                )])),
            ),
            Message::assistant().with_tool_request(
                "search_2",
                Ok(CallToolRequestParams::new("search").with_arguments(object!({}))),
            ),
        ];

        let (fixed, issues) = run_verify(messages);

        assert_eq!(fixed.len(), 1);

        assert_has_issues_unordered!(
            fixed,
            issues,
            "Removed empty message",
            "Removed orphaned tool response 'wrong_id'",
            "Removed orphaned tool request 'search_1'",
            "Removed orphaned tool request 'search_2'",
            "Removed empty message",
            "Removed empty message",
            "Removed leading assistant message",
            "Added placeholder user message to empty conversation",
        );

        assert_eq!(fixed[0].role, Role::User);
        assert_eq!(fixed[0].as_concat_text(), "Hello");
    }

    #[test]
    fn test_real_world_consecutive_assistant_messages() {
        use rmcp::model::Content;

        let conversation = Conversation::new_unvalidated(vec![
            Message::user().with_text("run ls in the current directory and then run a word count on the smallest file"),

            Message::assistant()
                .with_text("I'll help you run `ls` in the current directory and then perform a word count on the smallest file. Let me start by listing the directory contents.")
                .with_tool_request("toolu_bdrk_018adWbP4X26CfoJU5hkhu3i", Ok(CallToolRequestParams::new("developer__shell").with_arguments(object!({"command": "ls -la"})))),

            Message::assistant()
                .with_text("Now I'll identify the smallest file by size. Looking at the output, I can see that both `slack.yaml` and `subrecipes.yaml` have a size of 0 bytes, making them the smallest files. I'll run a word count on one of them:")
                .with_tool_request("toolu_bdrk_01KgDYHs4fAodi22NqxRzmwx", Ok(CallToolRequestParams::new("developer__shell").with_arguments(object!({"command": "wc slack.yaml"})))),

            Message::user()
                .with_tool_response("toolu_bdrk_01KgDYHs4fAodi22NqxRzmwx", Ok(rmcp::model::CallToolResult::success(vec![Content::text("0 0 0 slack.yaml")]))),

            Message::assistant()
                .with_text("I ran `ls -la` in the current directory and found several files. Looking at the file sizes, I can see that both `slack.yaml` and `subrecipes.yaml` are 0 bytes (the smallest files). I ran a word count on `slack.yaml` which shows: **0 lines**, **0 words**, **0 characters**"),
            Message::user().with_text("thanks!"),
        ]);

        let (fixed, issues) = fix_conversation(conversation);

        assert_eq!(fixed.len(), 5);
        assert_has_issues_unordered!(
            fixed,
            issues,
            "Removed orphaned tool request 'toolu_bdrk_018adWbP4X26CfoJU5hkhu3i'",
            "Merged consecutive assistant messages"
        )
    }

    #[test]
    fn test_tool_response_effective_role() {
        use rmcp::model::Content;

        let messages = vec![
            Message::user().with_text("Search for something"),
            Message::assistant()
                .with_text("I'll search for you")
                .with_tool_request(
                    "search_1",
                    Ok(CallToolRequestParams::new("search").with_arguments(object!({}))),
                ),
            Message::user().with_tool_response(
                "search_1",
                Ok(rmcp::model::CallToolResult::success(vec![Content::text(
                    "search results",
                )])),
            ),
            Message::user().with_text("Thanks!"),
        ];

        let (_fixed, issues) = run_verify(messages);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_merge_text_content_items() {
        use crate::conversation::message::MessageContent;
        use rmcp::model::{AnnotateAble, RawTextContent};

        let mut message = Message::assistant().with_text("Hello");

        message.content.push(MessageContent::Text(
            RawTextContent {
                text: " world".to_string(),
                meta: None,
            }
            .no_annotation(),
        ));
        message.content.push(MessageContent::Text(
            RawTextContent {
                text: "!".to_string(),
                meta: None,
            }
            .no_annotation(),
        ));

        let messages = vec![
            Message::user().with_text("hello"),
            message,
            Message::user().with_text("thanks"),
        ];

        let (fixed, issues) = run_verify(messages);

        assert_eq!(fixed.len(), 3);
        assert_has_issues_unordered!(fixed, issues, "Merged text content");

        let fixed_msg = &fixed[1];
        assert_eq!(fixed_msg.content.len(), 1);

        if let MessageContent::Text(text_content) = &fixed_msg.content[0] {
            assert_eq!(text_content.text, "Hello world!");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_merge_text_content_items_with_mixed_content() {
        use crate::conversation::message::MessageContent;
        use rmcp::model::{AnnotateAble, RawTextContent};

        let mut image_message = Message::assistant().with_text("Look at");

        image_message.content.push(MessageContent::Text(
            RawTextContent {
                text: " this image:".to_string(),
                meta: None,
            }
            .no_annotation(),
        ));

        image_message = image_message.with_image("", "");

        let messages = vec![
            Message::user().with_text("hello"),
            image_message,
            Message::user().with_text("thanks"),
        ];

        let (fixed, issues) = run_verify(messages);

        assert_eq!(fixed.len(), 3);
        assert_has_issues_unordered!(fixed, issues, "Merged text content");
        let fixed_msg = &fixed[1];

        assert_eq!(fixed_msg.content.len(), 2);
        if let MessageContent::Text(text_content) = &fixed_msg.content[0] {
            assert_eq!(text_content.text, "Look at this image:");
        } else {
            panic!("Expected first item to be text content");
        }

        if let MessageContent::Image(_) = &fixed_msg.content[1] {
            // Good
        } else {
            panic!("Expected second item to be an image");
        }
    }

    #[test]
    fn test_agent_visible_non_visible_message_ordering_with_fixes() {
        // Test that non-visible messages maintain their position relative to visible messages
        // even when visible messages are fixed (merged, removed, etc.)

        // Create messages with mixed visibility where visible ones need fixing
        let mut msg1_user = Message::user().with_text("First user message");
        msg1_user.metadata.agent_visible = true;

        let mut msg2_non_visible = Message::user().with_text("Non-visible note 1");
        msg2_non_visible.metadata.agent_visible = false;

        // These two consecutive user messages should be merged (triggering a fix)
        let mut msg3_user = Message::user().with_text("Second user message");
        msg3_user.metadata.agent_visible = true;

        let mut msg4_user = Message::user().with_text("Third user message");
        msg4_user.metadata.agent_visible = true;

        let mut msg5_non_visible = Message::user().with_text("Non-visible note 2");
        msg5_non_visible.metadata.agent_visible = false;

        let mut msg6_assistant = Message::assistant().with_text("Assistant response");
        msg6_assistant.metadata.agent_visible = true;

        let mut msg7_non_visible = Message::user().with_text("Non-visible note 3");
        msg7_non_visible.metadata.agent_visible = false;

        let mut msg8_user = Message::user().with_text("Final user message");
        msg8_user.metadata.agent_visible = true;

        let messages = vec![
            msg1_user.clone(),
            msg2_non_visible.clone(),
            msg3_user.clone(),
            msg4_user.clone(),
            msg5_non_visible.clone(),
            msg6_assistant.clone(),
            msg7_non_visible.clone(),
            msg8_user.clone(),
        ];

        let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages.clone()));

        // Should have merged consecutive user messages
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.contains("Merged consecutive")));

        let fixed_messages = fixed.messages();

        // Verify non-visible messages are still present
        let non_visible_texts: Vec<String> = fixed_messages
            .iter()
            .filter(|m| !m.metadata.agent_visible)
            .map(|m| m.as_concat_text())
            .collect();

        assert_eq!(non_visible_texts.len(), 3);
        assert_eq!(non_visible_texts[0], "Non-visible note 1");
        assert_eq!(non_visible_texts[1], "Non-visible note 2");
        assert_eq!(non_visible_texts[2], "Non-visible note 3");

        // Verify visible messages were processed
        let visible_texts: Vec<String> = fixed_messages
            .iter()
            .filter(|m| m.metadata.agent_visible)
            .map(|m| m.as_concat_text())
            .collect();

        // Should have 3 visible messages: first user, merged user messages, assistant, final user
        // But after merging consecutive users and fixing lead/trail, we get fewer
        assert!(!visible_texts.is_empty());

        // The key assertion: non-visible messages should be preserved and not reordered
        // relative to each other
        let mut found_note1 = false;
        let mut found_note2 = false;

        for msg in fixed_messages {
            let text = msg.as_concat_text();
            if text == "Non-visible note 1" {
                assert!(!found_note2 && !found_note1);
                found_note1 = true;
            } else if text == "Non-visible note 2" {
                assert!(found_note1 && !found_note2);
                found_note2 = true;
            } else if text == "Non-visible note 3" {
                assert!(found_note1 && found_note2);
            }
        }
    }

    #[test]
    fn test_shadow_map_with_multiple_consecutive_merges() {
        // Test the shadow map handles multiple consecutive visible messages that all merge
        let mut msg1 = Message::user().with_text("User 1");
        msg1.metadata.agent_visible = true;

        let mut msg2_non_vis = Message::user().with_text("Non-visible A");
        msg2_non_vis.metadata.agent_visible = false;

        let mut msg3 = Message::user().with_text("User 2");
        msg3.metadata.agent_visible = true;

        let mut msg4 = Message::user().with_text("User 3");
        msg4.metadata.agent_visible = true;

        let mut msg5 = Message::user().with_text("User 4");
        msg5.metadata.agent_visible = true;

        let mut msg6_non_vis = Message::user().with_text("Non-visible B");
        msg6_non_vis.metadata.agent_visible = false;

        let messages = vec![
            msg1,
            msg2_non_vis.clone(),
            msg3,
            msg4,
            msg5,
            msg6_non_vis.clone(),
        ];

        let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

        // Should have merged the consecutive user messages
        assert!(issues.iter().any(|i| i.contains("Merged consecutive")));

        let fixed_messages = fixed.messages();

        // Non-visible messages should still be present and in order
        let non_visible: Vec<String> = fixed_messages
            .iter()
            .filter(|m| !m.metadata.agent_visible)
            .map(|m| m.as_concat_text())
            .collect();

        assert_eq!(non_visible.len(), 2);
        assert_eq!(non_visible[0], "Non-visible A");
        assert_eq!(non_visible[1], "Non-visible B");

        // The merged message should contain all the user texts
        let visible: Vec<String> = fixed_messages
            .iter()
            .filter(|m| m.metadata.agent_visible)
            .map(|m| m.as_concat_text())
            .collect();

        assert_eq!(visible.len(), 1);
        assert!(visible[0].contains("User 1"));
        assert!(visible[0].contains("User 2"));
        assert!(visible[0].contains("User 3"));
        assert!(visible[0].contains("User 4"));
    }

    #[test]
    fn test_shadow_map_with_leading_trailing_removal() {
        // Test that shadow map handles removal of leading/trailing assistant messages
        let mut msg1_assistant = Message::assistant().with_text("Leading assistant");
        msg1_assistant.metadata.agent_visible = true;

        let mut msg2_non_vis = Message::user().with_text("Non-visible note");
        msg2_non_vis.metadata.agent_visible = false;

        let mut msg3_user = Message::user().with_text("User message");
        msg3_user.metadata.agent_visible = true;

        let mut msg4_assistant = Message::assistant().with_text("Assistant response");
        msg4_assistant.metadata.agent_visible = true;

        let mut msg5_assistant = Message::assistant().with_text("Trailing assistant");
        msg5_assistant.metadata.agent_visible = true;

        let messages = vec![
            msg1_assistant,
            msg2_non_vis.clone(),
            msg3_user,
            msg4_assistant,
            msg5_assistant,
        ];

        let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

        // Should have merged consecutive assistants, removed leading, and removed trailing
        assert!(issues
            .iter()
            .any(|i| i.contains("Merged consecutive assistant")));
        assert!(issues
            .iter()
            .any(|i| i.contains("Removed leading assistant")));
        assert!(issues
            .iter()
            .any(|i| i.contains("Removed trailing assistant")));

        let fixed_messages = fixed.messages();

        // Non-visible message should still be present
        let non_visible: Vec<String> = fixed_messages
            .iter()
            .filter(|m| !m.metadata.agent_visible)
            .map(|m| m.as_concat_text())
            .collect();

        assert_eq!(non_visible.len(), 1);
        assert_eq!(non_visible[0], "Non-visible note");

        // The two consecutive assistant messages get merged, then the merged message
        // is removed as trailing, leaving only the user message
        let visible: Vec<String> = fixed_messages
            .iter()
            .filter(|m| m.metadata.agent_visible)
            .map(|m| m.as_concat_text())
            .collect();

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0], "User message");
    }

    #[test]
    fn test_shadow_map_all_visible_messages_removed() {
        // Edge case: all visible messages are removed, only non-visible remain
        let mut msg1_assistant = Message::assistant().with_text("Only assistant");
        msg1_assistant.metadata.agent_visible = true;

        let mut msg2_non_vis = Message::user().with_text("Non-visible note 1");
        msg2_non_vis.metadata.agent_visible = false;

        let mut msg3_non_vis = Message::user().with_text("Non-visible note 2");
        msg3_non_vis.metadata.agent_visible = false;

        let messages = vec![msg1_assistant, msg2_non_vis, msg3_non_vis];

        let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

        // Should have removed the assistant and added placeholder
        assert!(issues
            .iter()
            .any(|i| i.contains("Removed leading assistant")));
        assert!(issues.iter().any(|i| i.contains("Added placeholder")));

        let fixed_messages = fixed.messages();

        // Non-visible messages should still be present
        let non_visible: Vec<String> = fixed_messages
            .iter()
            .filter(|m| !m.metadata.agent_visible)
            .map(|m| m.as_concat_text())
            .collect();

        assert_eq!(non_visible.len(), 2);
        assert_eq!(non_visible[0], "Non-visible note 1");
        assert_eq!(non_visible[1], "Non-visible note 2");

        // Should have placeholder user message
        let visible: Vec<String> = fixed_messages
            .iter()
            .filter(|m| m.metadata.agent_visible)
            .map(|m| m.as_concat_text())
            .collect();

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0], "Hello");
    }

    #[test]
    fn test_empty_tool_result_gets_placeholder() {
        // Test that tool results with empty content get a placeholder added
        let messages = vec![
            Message::user().with_text("Search for something"),
            Message::assistant()
                .with_text("I'll search for you")
                .with_tool_request(
                    "search_1",
                    Ok(CallToolRequestParams::new("search").with_arguments(object!({}))),
                ),
            Message::user().with_tool_response(
                "search_1",
                Ok(rmcp::model::CallToolResult::success(vec![])), // Empty content - this should get a placeholder
            ),
            Message::user().with_text("Thanks!"),
        ];

        let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

        // Should have added a placeholder
        assert!(issues
            .iter()
            .any(|i| i.contains("Added placeholder to empty tool result")));

        // Find the tool response and verify it has content now
        let tool_response_msg = fixed
            .messages()
            .iter()
            .find(|m| {
                m.content.iter().any(|c| {
                    matches!(
                        c,
                        crate::conversation::message::MessageContent::ToolResponse(_)
                    )
                })
            })
            .expect("Should have a tool response message");

        if let crate::conversation::message::MessageContent::ToolResponse(resp) =
            &tool_response_msg.content[0]
        {
            if let Ok(result) = &resp.tool_result {
                assert!(!result.content.is_empty(), "Content should not be empty");
                // Verify the placeholder text
                let text = result.content[0]
                    .as_text()
                    .expect("Should be text content")
                    .text
                    .clone();
                assert_eq!(text, "(empty result)");
            } else {
                panic!("Tool result should be Ok");
            }
        } else {
            panic!("First content should be ToolResponse");
        }
    }

    #[test]
    fn test_shadow_map_preserves_interleaving_pattern() {
        // Test that complex interleaving patterns are preserved
        let mut msg1_user = Message::user().with_text("User 1");
        msg1_user.metadata.agent_visible = true;

        let mut msg2_non_vis = Message::user().with_text("Non-vis A");
        msg2_non_vis.metadata.agent_visible = false;

        let mut msg3_assistant = Message::assistant().with_text("Assistant 1");
        msg3_assistant.metadata.agent_visible = true;

        let mut msg4_non_vis = Message::user().with_text("Non-vis B");
        msg4_non_vis.metadata.agent_visible = false;

        let mut msg5_user = Message::user().with_text("User 2");
        msg5_user.metadata.agent_visible = true;

        let mut msg6_non_vis = Message::user().with_text("Non-vis C");
        msg6_non_vis.metadata.agent_visible = false;

        let messages = vec![
            msg1_user,
            msg2_non_vis,
            msg3_assistant,
            msg4_non_vis,
            msg5_user,
            msg6_non_vis,
        ];

        let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

        // Should have no issues for this valid conversation
        assert!(issues.is_empty());

        let fixed_messages = fixed.messages();

        // Verify the interleaving pattern is preserved
        assert_eq!(fixed_messages.len(), 6);

        assert_eq!(fixed_messages[0].as_concat_text(), "User 1");
        assert!(fixed_messages[0].metadata.agent_visible);

        assert_eq!(fixed_messages[1].as_concat_text(), "Non-vis A");
        assert!(!fixed_messages[1].metadata.agent_visible);

        assert_eq!(fixed_messages[2].as_concat_text(), "Assistant 1");
        assert!(fixed_messages[2].metadata.agent_visible);

        assert_eq!(fixed_messages[3].as_concat_text(), "Non-vis B");
        assert!(!fixed_messages[3].metadata.agent_visible);

        assert_eq!(fixed_messages[4].as_concat_text(), "User 2");
        assert!(fixed_messages[4].metadata.agent_visible);

        assert_eq!(fixed_messages[5].as_concat_text(), "Non-vis C");
        assert!(!fixed_messages[5].metadata.agent_visible);
    }

    #[test]
    fn test_push_coalesces_thinking_deltas() {
        use crate::conversation::message::MessageContent;

        let mut conv = Conversation::empty();
        for fragment in ["I ", "should ", "think ", "about ", "this."] {
            conv.push(
                Message::assistant()
                    .with_thinking(fragment, "")
                    .with_id("turn-1"),
            );
        }

        assert_eq!(conv.messages().len(), 1);
        let content = &conv.messages()[0].content;
        assert_eq!(content.len(), 1);
        match &content[0] {
            MessageContent::Thinking(t) => {
                assert_eq!(t.thinking, "I should think about this.");
                assert_eq!(t.signature, "");
            }
            other => panic!("expected single Thinking block, got {:?}", other),
        }
    }

    #[test]
    fn test_push_thinking_adopts_signature_on_closing_delta() {
        use crate::conversation::message::MessageContent;

        let mut conv = Conversation::empty();
        // Streamed shape for one signed block: text deltas accumulate while
        // unsigned; the closing delta carries the signature.
        conv.push(
            Message::assistant()
                .with_thinking("a", "")
                .with_id("turn-1"),
        );
        conv.push(
            Message::assistant()
                .with_thinking("b", "sig1")
                .with_id("turn-1"),
        );

        let content = &conv.messages()[0].content;
        assert_eq!(content.len(), 1);
        match &content[0] {
            MessageContent::Thinking(t) => {
                assert_eq!(t.thinking, "ab");
                assert_eq!(t.signature, "sig1");
            }
            other => panic!("expected Thinking, got {:?}", other),
        }
    }

    #[test]
    fn test_push_unsigned_thinking_after_signed_starts_new_block() {
        use crate::conversation::message::MessageContent;

        let mut conv = Conversation::empty();
        conv.push(
            Message::assistant()
                .with_thinking("first body", "sig1")
                .with_id("turn-1"),
        );
        // A second thinking block begins; in signature-at-end streams the
        // first text arrives before the block's signature, so the new
        // unsigned delta must NOT be appended to the already-signed block —
        // otherwise the closing signature later would replay text under
        // the wrong signature.
        conv.push(
            Message::assistant()
                .with_thinking("second body start", "")
                .with_id("turn-1"),
        );

        let content = &conv.messages()[0].content;
        assert_eq!(
            content.len(),
            2,
            "unsigned delta must not merge into a signed block: {:?}",
            content
        );
        match (&content[0], &content[1]) {
            (MessageContent::Thinking(a), MessageContent::Thinking(b)) => {
                assert_eq!(a.thinking, "first body");
                assert_eq!(a.signature, "sig1");
                assert_eq!(b.thinking, "second body start");
                assert_eq!(b.signature, "");
            }
            other => panic!("unexpected content shape: {:?}", other),
        }
    }

    #[test]
    fn test_push_keeps_distinct_signed_thinking_blocks_separate() {
        use crate::conversation::message::MessageContent;

        let mut conv = Conversation::empty();
        conv.push(
            Message::assistant()
                .with_thinking("block A", "sig-A")
                .with_id("turn-1"),
        );
        conv.push(
            Message::assistant()
                .with_thinking("block B", "sig-B")
                .with_id("turn-1"),
        );

        let content = &conv.messages()[0].content;
        assert_eq!(
            content.len(),
            2,
            "two distinct signed blocks must not coalesce: {:?}",
            content
        );
        match (&content[0], &content[1]) {
            (MessageContent::Thinking(a), MessageContent::Thinking(b)) => {
                assert_eq!(a.thinking, "block A");
                assert_eq!(a.signature, "sig-A");
                assert_eq!(b.thinking, "block B");
                assert_eq!(b.signature, "sig-B");
            }
            other => panic!("unexpected content shape: {:?}", other),
        }
    }

    #[test]
    fn test_push_does_not_coalesce_multi_block_thinking_message() {
        use crate::conversation::message::MessageContent;

        let mut conv = Conversation::empty();
        conv.push(
            Message::assistant()
                .with_thinking("first", "")
                .with_id("turn-1"),
        );

        // Multi-block message must NOT coalesce into the existing thinking
        // block — the merge arm requires `message.content.len() == 1`.
        let mut multi = Message::assistant().with_thinking("second", "");
        multi = multi.with_text("and now text").with_id("turn-1");
        conv.push(multi);

        let content = &conv.messages()[0].content;
        assert_eq!(content.len(), 3);
        match (&content[0], &content[1], &content[2]) {
            (MessageContent::Thinking(a), MessageContent::Thinking(b), MessageContent::Text(c)) => {
                assert_eq!(a.thinking, "first");
                assert_eq!(b.thinking, "second");
                assert_eq!(c.text, "and now text");
            }
            other => panic!("unexpected content shape: {:?}", other),
        }
    }
}
