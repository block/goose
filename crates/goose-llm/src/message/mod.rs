//! Messages which represent the content sent back and forth to LLM provider
//!
//! We use these messages in the agent code, and interfaces which interact with
//! the agent. That let's us reuse message histories across different interfaces.
//!
//! The content of the messages uses MCP types to avoid additional conversions
//! when interacting with MCP servers.

mod contents;
mod message_content;
mod tool_result_serde;

pub use contents::Contents;
pub use message_content::{
    MessageContent, RedactedThinkingContent, ThinkingContent, ToolRequest, ToolRequestToolCall,
    ToolResponse, ToolResponseToolResult,
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::types::core::Role;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
/// A message to or from an LLM
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: Role,
    pub created: i64,
    pub content: Contents,
}

impl Message {
    pub fn new(role: Role) -> Self {
        Self {
            role,
            created: Utc::now().timestamp_millis(),
            content: Contents::default(),
        }
    }

    /// Create a new user message with the current timestamp
    pub fn user() -> Self {
        Self::new(Role::User)
    }

    /// Create a new assistant message with the current timestamp
    pub fn assistant() -> Self {
        Self::new(Role::Assistant)
    }

    /// Add any item that implements Into<MessageContent> to the message
    pub fn with_content(mut self, item: impl Into<MessageContent>) -> Self {
        self.content.push(item);
        self
    }

    /// Add text content to the message
    pub fn with_text<S: Into<String>>(self, text: S) -> Self {
        self.with_content(MessageContent::text(text))
    }

    /// Add image content to the message
    pub fn with_image<S: Into<String>, T: Into<String>>(self, data: S, mime_type: T) -> Self {
        self.with_content(MessageContent::image(data, mime_type))
    }

    /// Add a tool request to the message
    pub fn with_tool_request<S: Into<String>, T: Into<ToolRequestToolCall>>(
        self,
        id: S,
        tool_call: T,
    ) -> Self {
        self.with_content(MessageContent::tool_request(id, tool_call.into()))
    }

    /// Add a tool response to the message
    pub fn with_tool_response<S: Into<String>>(
        self,
        id: S,
        result: ToolResponseToolResult,
    ) -> Self {
        self.with_content(MessageContent::tool_response(id, result))
    }

    /// Add thinking content to the message
    pub fn with_thinking<S1: Into<String>, S2: Into<String>>(
        self,
        thinking: S1,
        signature: S2,
    ) -> Self {
        self.with_content(MessageContent::thinking(thinking, signature))
    }

    /// Add redacted thinking content to the message
    pub fn with_redacted_thinking<S: Into<String>>(self, data: S) -> Self {
        self.with_content(MessageContent::redacted_thinking(data))
    }

    /// Check if the message is a tool call
    pub fn contains_tool_call(&self) -> bool {
        self.content.any_is(MessageContent::is_tool_request)
    }

    /// Check if the message is a tool response
    pub fn contains_tool_response(&self) -> bool {
        self.content.any_is(MessageContent::is_tool_response)
    }

    /// Check if the message contains only text content
    pub fn has_only_text_content(&self) -> bool {
        self.content.all_are(MessageContent::is_text)
    }

    /// Retrieves all tool `id` from ToolRequest messages
    pub fn tool_request_ids(&self) -> HashSet<&str> {
        self.content
            .iter()
            .filter_map(MessageContent::as_tool_request_id)
            .collect()
    }

    /// Retrieves all tool `id` from ToolResponse messages
    pub fn tool_response_ids(&self) -> HashSet<&str> {
        self.content
            .iter()
            .filter_map(MessageContent::as_tool_response_id)
            .collect()
    }

    /// Retrieves all tool `id` from the message
    pub fn tool_ids(&self) -> HashSet<&str> {
        self.tool_request_ids()
            .into_iter()
            .chain(self.tool_response_ids())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::types::core::{ToolCall, ToolError};

    #[test]
    fn test_message_serialization() {
        let message = Message::assistant()
            .with_text("Hello, I'll help you with that.")
            .with_tool_request(
                "tool123",
                Ok(ToolCall::new("test_tool", json!({"param": "value"}))),
            );

        let json_str = serde_json::to_string_pretty(&message).unwrap();
        println!("Serialized message: {}", json_str);

        // Parse back to Value to check structure
        let value: Value = serde_json::from_str(&json_str).unwrap();
        println!(
            "Read back serialized message: {}",
            serde_json::to_string_pretty(&value).unwrap()
        );

        // Check top-level fields
        assert_eq!(value["role"], "assistant");
        assert!(value["created"].is_i64());
        assert!(value["content"].is_array());

        // Check content items
        let content = &value["content"];

        // First item should be text
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Hello, I'll help you with that.");

        // Second item should be toolRequest
        assert_eq!(content[1]["type"], "toolReq");
        assert_eq!(content[1]["id"], "tool123");

        // Check tool_call serialization
        assert_eq!(content[1]["toolCall"]["status"], "success");
        assert_eq!(content[1]["toolCall"]["value"]["name"], "test_tool");
        assert_eq!(
            content[1]["toolCall"]["value"]["arguments"]["param"],
            "value"
        );
    }

    #[test]
    fn test_error_serialization() {
        let message = Message::assistant().with_tool_request(
            "tool123",
            Err(ToolError::ExecutionError(
                "Something went wrong".to_string(),
            )),
        );

        let json_str = serde_json::to_string_pretty(&message).unwrap();
        println!("Serialized error: {}", json_str);

        // Parse back to Value to check structure
        let value: Value = serde_json::from_str(&json_str).unwrap();

        // Check tool_call serialization with error
        let tool_call = &value["content"][0]["toolCall"];
        assert_eq!(tool_call["status"], "error");
        assert_eq!(tool_call["error"], "Execution failed: Something went wrong");
    }

    #[test]
    fn test_deserialization() {
        // Create a JSON string with our new format
        let json_str = r#"{
            "role": "assistant",
            "created": 1740171566,
            "content": [
                {
                    "type": "text",
                    "text": "I'll help you with that."
                },
                {
                    "type": "toolReq",
                    "id": "tool123",
                    "toolCall": {
                        "status": "success",
                        "value": {
                            "name": "test_tool",
                            "arguments": {"param": "value"},
                            "needsApproval": false
                        }
                    }
                }
            ]
        }"#;

        let message: Message = serde_json::from_str(json_str).unwrap();

        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.created, 1740171566);
        assert_eq!(message.content.len(), 2);

        // Check first content item
        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "I'll help you with that.");
        } else {
            panic!("Expected Text content");
        }

        // Check second content item
        if let MessageContent::ToolReq(req) = &message.content[1] {
            assert_eq!(req.id, "tool123");
            if let Ok(tool_call) = req.tool_call.as_result() {
                assert_eq!(tool_call.name, "test_tool");
                assert_eq!(tool_call.arguments, json!({"param": "value"}));
            } else {
                panic!("Expected successful tool call");
            }
        } else {
            panic!("Expected ToolRequest content");
        }
    }

    #[test]
    fn test_message_with_text() {
        let message = Message::user().with_text("Hello");
        assert_eq!(message.content.concat_text_str(), "Hello");
    }

    #[test]
    fn test_message_with_tool_request() {
        let tool_call = Ok(ToolCall::new("test_tool", json!({})));

        let message = Message::assistant().with_tool_request("req1", tool_call);
        assert!(message.contains_tool_call());
        assert!(!message.contains_tool_response());

        let ids = message.tool_ids();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains("req1"));
    }
}
