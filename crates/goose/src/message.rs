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
use mcp_core::role::Role;
use mcp_core::tool::ToolCall;
use serde_json::Value;

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
pub struct ToolConfirmationRequest {
    pub id: String,
    pub tool_name: String,
    pub arguments: Value,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
/// Content passed inside a message, which can be both simple content and tool content
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
    ToolRequest(ToolRequest),
    ToolResponse(ToolResponse),
    ToolConfirmationRequest(ToolConfirmationRequest),
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
