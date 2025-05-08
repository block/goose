use std::{collections::HashSet, iter::FromIterator, ops::Deref};

/// Messages which represent the content sent back and forth to LLM provider
///
/// We use these messages in the agent code, and interfaces which interact with
/// the agent. That let's us reuse message histories across different interfaces.
///
/// The content of the messages uses MCP types to avoid additional conversions
/// when interacting with MCP servers.
use chrono::Utc;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::types::core::{Content, ImageContent, Role, TextContent, ToolCall, ToolResult};

mod tool_result_serde;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub id: String,
    #[serde(with = "tool_result_serde")]
    pub tool_call: ToolResult<ToolCall>,
}

impl ToolRequest {
    pub fn to_readable_string(&self) -> String {
        match &self.tool_call {
            Ok(tool_call) => {
                format!(
                    "Tool: {}, Args: {}",
                    tool_call.name,
                    serde_json::to_string_pretty(&tool_call.arguments)
                        .unwrap_or_else(|_| "<<invalid json>>".to_string())
                )
            }
            Err(e) => format!("Invalid tool call: {}", e),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    pub id: String,
    #[serde(with = "tool_result_serde")]
    pub tool_result: ToolResult<Vec<Content>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingContent {
    pub thinking: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedactedThinkingContent {
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Content passed inside a message, which can be both simple content and tool content
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
    ToolRequest(ToolRequest),
    ToolResponse(ToolResponse),
    Thinking(ThinkingContent),
    RedactedThinking(RedactedThinkingContent),
}

impl MessageContent {
    pub fn text<S: Into<String>>(text: S) -> Self {
        MessageContent::Text(TextContent { text: text.into() })
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        MessageContent::Image(ImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
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

    pub fn as_tool_request_id(&self) -> Option<&str> {
        if let Self::ToolRequest(r) = self {
            Some(&r.id)
        } else {
            None
        }
    }

    pub fn as_tool_response_id(&self) -> Option<&str> {
        if let Self::ToolResponse(r) = self {
            Some(&r.id)
        } else {
            None
        }
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

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }
    pub fn is_tool_request(&self) -> bool {
        matches!(self, Self::ToolRequest(_))
    }
    pub fn is_tool_response(&self) -> bool {
        matches!(self, Self::ToolResponse(_))
    }
}

impl From<Content> for MessageContent {
    fn from(content: Content) -> Self {
        match content {
            Content::Text(text) => MessageContent::Text(text),
            Content::Image(image) => MessageContent::Image(image),
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// 2. Contents – a new-type wrapper around SmallVec
// ────────────────────────────────────────────────────────────────────────────

/// Holds the heterogeneous fragments that make up one chat message.
///
/// *   Up to two items are stored inline on the stack.
/// *   Falls back to a heap allocation only when necessary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Contents(SmallVec<[MessageContent; 2]>);

impl Contents {
    /*----------------------------------------------------------
     * 1-line ergonomic helpers
     *---------------------------------------------------------*/

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, MessageContent> {
        self.0.iter_mut()
    }

    pub fn push(&mut self, item: impl Into<MessageContent>) {
        self.0.push(item.into());
    }

    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.0.iter().filter_map(|c| c.as_text())
    }

    pub fn concat_text_str(&self) -> String {
        self.texts().collect::<Vec<_>>().join("\n")
    }

    /// Returns `true` if *any* item satisfies the predicate.
    pub fn any_is<P>(&self, pred: P) -> bool
    where
        P: FnMut(&MessageContent) -> bool,
    {
        self.iter().any(pred)
    }

    /// Returns `true` if *every* item satisfies the predicate.
    pub fn all_are<P>(&self, pred: P) -> bool
    where
        P: FnMut(&MessageContent) -> bool,
    {
        self.iter().all(pred)
    }
}

impl From<Vec<MessageContent>> for Contents {
    fn from(v: Vec<MessageContent>) -> Self {
        Contents(SmallVec::from_vec(v))
    }
}

impl FromIterator<MessageContent> for Contents {
    fn from_iter<I: IntoIterator<Item = MessageContent>>(iter: I) -> Self {
        Contents(SmallVec::from_iter(iter))
    }
}

/*--------------------------------------------------------------
 * Allow &message.content to behave like a slice of fragments.
 *-------------------------------------------------------------*/
impl Deref for Contents {
    type Target = [MessageContent];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
            created: Utc::now().timestamp(),
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
    use crate::types::core::ToolError;

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
