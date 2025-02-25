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
use mcp_core::prompt::PromptMessageContent;
use mcp_core::resource::ResourceContents;
use mcp_core::role::Role;
use mcp_core::tool::ToolCall;

/// Convert PromptMessageContent to MessageContent
///
/// This function allows converting from the prompt message content type
/// to the message content type used in the agent.
pub fn prompt_content_to_message_content(content: PromptMessageContent) -> MessageContent {
    match content {
        PromptMessageContent::Text { text } => MessageContent::text(text),
        PromptMessageContent::Image { image } => MessageContent::image(image.data, image.mime_type),
        PromptMessageContent::Resource { resource } => {
            // For resources, convert to text content with the resource text
            match resource.resource {
                ResourceContents::TextResourceContents { text, .. } => MessageContent::text(text),
                ResourceContents::BlobResourceContents { blob, .. } => {
                    MessageContent::text(format!("[Binary content: {}]", blob))
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolRequest {
    pub id: String,
    pub tool_call: ToolResult<ToolCall>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolResponse {
    pub id: String,
    pub tool_result: ToolResult<Vec<Content>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// Content passed inside a message, which can be both simple content and tool content
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
    ToolRequest(ToolRequest),
    ToolResponse(ToolResponse),
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// A message to or from an LLM
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
    use mcp_core::prompt::PromptMessageContent;
    use mcp_core::resource::ResourceContents;

    #[test]
    fn test_prompt_content_to_message_content_text() {
        let prompt_content = PromptMessageContent::Text {
            text: "Hello, world!".to_string(),
        };

        let message_content = prompt_content_to_message_content(prompt_content);

        if let MessageContent::Text(text_content) = message_content {
            assert_eq!(text_content.text, "Hello, world!");
        } else {
            panic!("Expected MessageContent::Text");
        }
    }

    #[test]
    fn test_prompt_content_to_message_content_image() {
        let prompt_content = PromptMessageContent::Image {
            image: ImageContent {
                data: "base64data".to_string(),
                mime_type: "image/jpeg".to_string(),
                annotations: None,
            },
        };

        let message_content = prompt_content_to_message_content(prompt_content);

        if let MessageContent::Image(image_content) = message_content {
            assert_eq!(image_content.data, "base64data");
            assert_eq!(image_content.mime_type, "image/jpeg");
        } else {
            panic!("Expected MessageContent::Image");
        }
    }

    #[test]
    fn test_prompt_content_to_message_content_text_resource() {
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

        let message_content = prompt_content_to_message_content(prompt_content);

        if let MessageContent::Text(text_content) = message_content {
            assert_eq!(text_content.text, "Resource content");
        } else {
            panic!("Expected MessageContent::Text");
        }
    }

    #[test]
    fn test_prompt_content_to_message_content_blob_resource() {
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

        let message_content = prompt_content_to_message_content(prompt_content);

        if let MessageContent::Text(text_content) = message_content {
            assert_eq!(text_content.text, "[Binary content: binary_data]");
        } else {
            panic!("Expected MessageContent::Text");
        }
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
