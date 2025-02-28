use std::collections::HashSet;

/// Messages which represent the content sent back and forth to LLM provider
///
/// We use these messages in the agent code, and interfaces which interact with
/// the agent. That let's us reuse message histories across different interfaces.
///
/// The content of the messages uses MCP types to avoid additional conversions
/// when interacting with MCP servers.
use chrono::Utc;
use mcp_core::content::{Content, ImageContent, TextContent};
use mcp_core::handler::ToolResult;
use mcp_core::prompt::{PromptMessage, PromptMessageContent, PromptMessageRole};
use mcp_core::resource::ResourceContents;
use mcp_core::role::Role;
use mcp_core::tool::ToolCall;
use serde_json::Value;

mod tool_result_serde;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub id: String,
    #[serde(with = "tool_result_serde")]
    pub tool_call: ToolResult<ToolCall>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    pub id: String,
    #[serde(with = "tool_result_serde")]
    pub tool_result: ToolResult<Vec<Content>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfirmationRequest {
    pub id: String,
    pub tool_name: String,
    pub arguments: Value,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ThinkingContent {
    pub thinking: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RedactedThinkingContent {
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// Content passed inside a message, which can be both simple content and tool content
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
    ToolRequest(ToolRequest),
    ToolResponse(ToolResponse),
    ToolConfirmationRequest(ToolConfirmationRequest),
    Thinking(ThinkingContent),
    RedactedThinking(RedactedThinkingContent),
}

impl MessageContent {
    pub fn text<S: Into<String>>(text: S) -> Self {
        MessageContent::Text(TextContent {
            text: text.into(),
            annotations: None,
        })
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        MessageContent::Image(ImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
        })
    }

    pub fn tool_request<S: Into<String>>(id: S, tool_call: ToolResult<ToolCall>) -> Self {
        MessageContent::ToolRequest(ToolRequest {
            id: id.into(),
            tool_call,
        })
    }

    pub fn tool_response<S: Into<String>>(id: S, tool_result: ToolResult<Vec<Content>>) -> Self {
        MessageContent::ToolResponse(ToolResponse {
            id: id.into(),
            tool_result,
        })
    }

    pub fn tool_confirmation_request<S: Into<String>>(
        id: S,
        tool_name: String,
        arguments: Value,
        prompt: Option<String>,
    ) -> Self {
        MessageContent::ToolConfirmationRequest(ToolConfirmationRequest {
            id: id.into(),
            tool_name,
            arguments,
            prompt,
        })
    }

    pub fn thinking<S1: Into<String>, S2: Into<String>>(thinking: S1, signature: S2) -> Self {
        MessageContent::Thinking(ThinkingContent {
            thinking: thinking.into(),
            signature: signature.into(),
        })
    }

    pub fn redacted_thinking<S: Into<String>>(data: S) -> Self {
        MessageContent::RedactedThinking(RedactedThinkingContent { data: data.into() })
    }
    pub fn as_tool_request(&self) -> Option<&ToolRequest> {
        if let MessageContent::ToolRequest(ref tool_request) = self {
            Some(tool_request)
        } else {
            None
        }
    }

    pub fn as_tool_response(&self) -> Option<&ToolResponse> {
        if let MessageContent::ToolResponse(ref tool_response) = self {
            Some(tool_response)
        } else {
            None
        }
    }

    pub fn as_tool_confirmation_request(&self) -> Option<&ToolConfirmationRequest> {
        if let MessageContent::ToolConfirmationRequest(ref tool_confirmation_request) = self {
            Some(tool_confirmation_request)
        } else {
            None
        }
    }

    pub fn as_tool_response_text(&self) -> Option<String> {
        if let Some(tool_response) = self.as_tool_response() {
            if let Ok(contents) = &tool_response.tool_result {
                let texts: Vec<String> = contents
                    .iter()
                    .filter_map(|content| content.as_text().map(String::from))
                    .collect();
                if !texts.is_empty() {
                    return Some(texts.join("\n"));
                }
            }
        }
        None
    }

    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(text) => Some(&text.text),
            _ => None,
        }
    }

    /// Get the thinking content if this is a ThinkingContent variant
    pub fn as_thinking(&self) -> Option<&ThinkingContent> {
        match self {
            MessageContent::Thinking(thinking) => Some(thinking),
            _ => None,
        }
    }

    /// Get the redacted thinking content if this is a RedactedThinkingContent variant
    pub fn as_redacted_thinking(&self) -> Option<&RedactedThinkingContent> {
        match self {
            MessageContent::RedactedThinking(redacted) => Some(redacted),
            _ => None,
        }
    }
}

impl From<Content> for MessageContent {
    fn from(content: Content) -> Self {
        match content {
            Content::Text(text) => MessageContent::Text(text),
            Content::Image(image) => MessageContent::Image(image),
            Content::Resource(resource) => MessageContent::Text(TextContent {
                text: resource.get_text(),
                annotations: None,
            }),
        }
    }
}

impl From<PromptMessage> for Message {
    fn from(prompt_message: PromptMessage) -> Self {
        // Create a new message with the appropriate role
        let message = match prompt_message.role {
            PromptMessageRole::User => Message::user(),
            PromptMessageRole::Assistant => Message::assistant(),
        };

        // Convert and add the content
        let content = match prompt_message.content {
            PromptMessageContent::Text { text } => MessageContent::text(text),
            PromptMessageContent::Image { image } => {
                MessageContent::image(image.data, image.mime_type)
            }
            PromptMessageContent::Resource { resource } => {
                // For resources, convert to text content with the resource text
                match resource.resource {
                    ResourceContents::TextResourceContents { text, .. } => {
                        MessageContent::text(text)
                    }
                    ResourceContents::BlobResourceContents { blob, .. } => {
                        MessageContent::text(format!("[Binary content: {}]", blob))
                    }
                }
            }
        };

        message.with_content(content)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// A message to or from an LLM
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: Role,
    pub created: i64,
    pub content: Vec<MessageContent>,
}

impl Message {
    /// Create a new user message with the current timestamp
    pub fn user() -> Self {
        Message {
            role: Role::User,
            created: Utc::now().timestamp(),
            content: Vec::new(),
        }
    }

    /// Create a new assistant message with the current timestamp
    pub fn assistant() -> Self {
        Message {
            role: Role::Assistant,
            created: Utc::now().timestamp(),
            content: Vec::new(),
        }
    }

    /// Add any MessageContent to the message
    pub fn with_content(mut self, content: MessageContent) -> Self {
        self.content.push(content);
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
    pub fn with_tool_request<S: Into<String>>(
        self,
        id: S,
        tool_call: ToolResult<ToolCall>,
    ) -> Self {
        self.with_content(MessageContent::tool_request(id, tool_call))
    }

    /// Add a tool response to the message
    pub fn with_tool_response<S: Into<String>>(
        self,
        id: S,
        result: ToolResult<Vec<Content>>,
    ) -> Self {
        self.with_content(MessageContent::tool_response(id, result))
    }

    /// Add a tool confirmation request to the message
    pub fn with_tool_confirmation_request<S: Into<String>>(
        self,
        id: S,
        tool_name: String,
        arguments: Value,
        prompt: Option<String>,
    ) -> Self {
        self.with_content(MessageContent::tool_confirmation_request(
            id, tool_name, arguments, prompt,
        ))
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

    /// Get the concatenated text content of the message, separated by newlines
    pub fn as_concat_text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if the message is a tool call
    pub fn is_tool_call(&self) -> bool {
        self.content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(_)))
    }

    /// Check if the message is a tool response
    pub fn is_tool_response(&self) -> bool {
        self.content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolResponse(_)))
    }

    /// Retrieves all tool `id` from the message
    pub fn get_tool_ids(&self) -> HashSet<&str> {
        self.content
            .iter()
            .filter_map(|content| match content {
                MessageContent::ToolRequest(req) => Some(req.id.as_str()),
                MessageContent::ToolResponse(res) => Some(res.id.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Retrieves all tool `id` from ToolRequest messages
    pub fn get_tool_request_ids(&self) -> HashSet<&str> {
        self.content
            .iter()
            .filter_map(|content| {
                if let MessageContent::ToolRequest(req) = content {
                    Some(req.id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Retrieves all tool `id` from ToolResponse messages
    pub fn get_tool_response_ids(&self) -> HashSet<&str> {
        self.content
            .iter()
            .filter_map(|content| {
                if let MessageContent::ToolResponse(res) = content {
                    Some(res.id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if the message has only TextContent
    pub fn has_only_text_content(&self) -> bool {
        self.content
            .iter()
            .all(|c| matches!(c, MessageContent::Text(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::content::EmbeddedResource;
    use mcp_core::handler::ToolError;
    use mcp_core::prompt::PromptMessageContent;
    use mcp_core::resource::ResourceContents;
    use serde_json::{json, Value};

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
        assert_eq!(content[1]["type"], "toolRequest");
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
                    "type": "toolRequest",
                    "id": "tool123",
                    "toolCall": {
                        "status": "success",
                        "value": {
                            "name": "test_tool",
                            "arguments": {"param": "value"}
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
        if let MessageContent::ToolRequest(req) = &message.content[1] {
            assert_eq!(req.id, "tool123");
            if let Ok(tool_call) = &req.tool_call {
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
    fn test_from_prompt_message_text() {
        let prompt_content = PromptMessageContent::Text {
            text: "Hello, world!".to_string(),
        };

        let prompt_message = PromptMessage {
            role: PromptMessageRole::User,
            content: prompt_content,
        };

        let message = Message::from(prompt_message);

        if let MessageContent::Text(text_content) = &message.content[0] {
            assert_eq!(text_content.text, "Hello, world!");
        } else {
            panic!("Expected MessageContent::Text");
        }
    }

    #[test]
    fn test_from_prompt_message_image() {
        let prompt_content = PromptMessageContent::Image {
            image: ImageContent {
                data: "base64data".to_string(),
                mime_type: "image/jpeg".to_string(),
                annotations: None,
            },
        };

        let prompt_message = PromptMessage {
            role: PromptMessageRole::User,
            content: prompt_content,
        };

        let message = Message::from(prompt_message);

        if let MessageContent::Image(image_content) = &message.content[0] {
            assert_eq!(image_content.data, "base64data");
            assert_eq!(image_content.mime_type, "image/jpeg");
        } else {
            panic!("Expected MessageContent::Image");
        }
    }

    #[test]
    fn test_from_prompt_message_text_resource() {
        let resource = ResourceContents::TextResourceContents {
            uri: "file:///test.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            text: "Resource content".to_string(),
        };

        let prompt_content = PromptMessageContent::Resource {
            resource: EmbeddedResource {
                resource,
                annotations: None,
            },
        };

        let prompt_message = PromptMessage {
            role: PromptMessageRole::User,
            content: prompt_content,
        };

        let message = Message::from(prompt_message);

        if let MessageContent::Text(text_content) = &message.content[0] {
            assert_eq!(text_content.text, "Resource content");
        } else {
            panic!("Expected MessageContent::Text");
        }
    }

    #[test]
    fn test_from_prompt_message_blob_resource() {
        let resource = ResourceContents::BlobResourceContents {
            uri: "file:///test.bin".to_string(),
            mime_type: Some("application/octet-stream".to_string()),
            blob: "binary_data".to_string(),
        };

        let prompt_content = PromptMessageContent::Resource {
            resource: EmbeddedResource {
                resource,
                annotations: None,
            },
        };

        let prompt_message = PromptMessage {
            role: PromptMessageRole::User,
            content: prompt_content,
        };

        let message = Message::from(prompt_message);

        if let MessageContent::Text(text_content) = &message.content[0] {
            assert_eq!(text_content.text, "[Binary content: binary_data]");
        } else {
            panic!("Expected MessageContent::Text");
        }
    }

    #[test]
    fn test_from_prompt_message() {
        // Test user message conversion
        let prompt_message = PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::Text {
                text: "Hello, world!".to_string(),
            },
        };

        let message = Message::from(prompt_message);
        assert_eq!(message.role, Role::User);
        assert_eq!(message.content.len(), 1);
        assert_eq!(message.as_concat_text(), "Hello, world!");

        // Test assistant message conversion
        let prompt_message = PromptMessage {
            role: PromptMessageRole::Assistant,
            content: PromptMessageContent::Text {
                text: "I can help with that.".to_string(),
            },
        };

        let message = Message::from(prompt_message);
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content.len(), 1);
        assert_eq!(message.as_concat_text(), "I can help with that.");
    }

    #[test]
    fn test_message_with_text() {
        let message = Message::user().with_text("Hello");
        assert_eq!(message.as_concat_text(), "Hello");
    }

    #[test]
    fn test_message_with_tool_request() {
        let tool_call = Ok(ToolCall {
            name: "test_tool".to_string(),
            arguments: serde_json::json!({}),
        });

        let message = Message::assistant().with_tool_request("req1", tool_call);
        assert!(message.is_tool_call());
        assert!(!message.is_tool_response());

        let ids = message.get_tool_ids();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains("req1"));
    }
}
