//! Bidirectional converters between Goose Message ↔ A2A Message.

use a2a::types::{Message as A2aMessage, Part, PartContent, Role as A2aRole};

use crate::conversation::message::{Message, MessageContent};
use rmcp::model::Role;

/// Convert a Goose Message to an A2A Message.
pub fn goose_message_to_a2a(msg: &Message) -> A2aMessage {
    let role = match msg.role {
        Role::User => A2aRole::User,
        Role::Assistant => A2aRole::Agent,
    };

    let parts: Vec<Part> = msg
        .content
        .iter()
        .filter_map(goose_content_to_a2a_part)
        .collect();

    A2aMessage {
        role,
        parts,
        message_id: msg.id.clone().unwrap_or_default(),
        context_id: None,
        task_id: None,
        extensions: vec![],
        metadata: None,
        reference_task_ids: vec![],
    }
}

/// Convert an A2A Message to a Goose Message.
pub fn a2a_message_to_goose(msg: &A2aMessage) -> Message {
    let role = match msg.role {
        A2aRole::User | A2aRole::Unspecified => Role::User,
        A2aRole::Agent => Role::Assistant,
    };

    let content: Vec<MessageContent> = msg
        .parts
        .iter()
        .filter_map(a2a_part_to_goose_content)
        .collect();

    Message::new(role, chrono::Utc::now().timestamp(), content)
}

fn goose_content_to_a2a_part(content: &MessageContent) -> Option<Part> {
    match content {
        MessageContent::Text(t) => Some(Part {
            content: PartContent::Text {
                text: t.text.clone(),
            },
            metadata: None,
            filename: None,
            media_type: None,
        }),
        MessageContent::JsonRenderSpec(spec) => Some(Part {
            content: PartContent::Text {
                text: spec.spec.clone(),
            },
            metadata: None,
            filename: None,
            media_type: None,
        }),
        MessageContent::Image(img) => Some(Part {
            content: PartContent::File {
                raw: Some(img.data.clone()),
                url: None,
            },
            metadata: None,
            filename: None,
            media_type: Some(img.mime_type.clone()),
        }),
        // Tool requests/responses are Goose-internal; skip for A2A.
        MessageContent::ToolRequest(_)
        | MessageContent::ToolResponse(_)
        | MessageContent::ToolConfirmationRequest(_)
        | MessageContent::ActionRequired(_)
        | MessageContent::FrontendToolRequest(_)
        | MessageContent::Thinking(_)
        | MessageContent::RedactedThinking(_)
        | MessageContent::SystemNotification(_) => None,
    }
}

fn a2a_part_to_goose_content(part: &Part) -> Option<MessageContent> {
    match &part.content {
        PartContent::Text { text } => Some(MessageContent::text(text.clone())),
        PartContent::File { raw, url } => {
            if let Some(data) = raw {
                let mime = part
                    .media_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                Some(MessageContent::image(data.clone(), mime))
            } else {
                url.as_ref()
                    .map(|url| MessageContent::text(format!("[File: {}]", url)))
            }
        }
        PartContent::Data { data } => Some(MessageContent::text(
            serde_json::to_string_pretty(data).unwrap_or_default(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goose_to_a2a_text() {
        let msg = Message::new(
            Role::User,
            1234567890,
            vec![MessageContent::text("Hello A2A!")],
        );
        let a2a = goose_message_to_a2a(&msg);
        assert_eq!(a2a.role, A2aRole::User);
        assert_eq!(a2a.parts.len(), 1);
        match &a2a.parts[0].content {
            PartContent::Text { text } => assert_eq!(text, "Hello A2A!"),
            _ => panic!("Expected text part"),
        }
    }

    #[test]
    fn test_a2a_to_goose_text() {
        let a2a = A2aMessage {
            role: A2aRole::Agent,
            parts: vec![Part::text("Response from agent")],
            message_id: "msg-1".to_string(),
            context_id: None,
            task_id: None,
            extensions: vec![],
            metadata: None,
            reference_task_ids: vec![],
        };
        let goose = a2a_message_to_goose(&a2a);
        assert_eq!(goose.role, Role::Assistant);
        assert_eq!(goose.content.len(), 1);
        match &goose.content[0] {
            MessageContent::Text(t) => assert!(t.raw.text.contains("Response from agent")),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_roundtrip_text() {
        let original = Message::new(
            Role::User,
            1234567890,
            vec![MessageContent::text("Roundtrip test")],
        );
        let a2a = goose_message_to_a2a(&original);
        let restored = a2a_message_to_goose(&a2a);
        assert_eq!(restored.role, original.role);
        assert_eq!(restored.content.len(), original.content.len());
    }
}
