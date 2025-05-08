use serde::{Deserialize, Serialize};
use serde_json;

use crate::message::tool_result_serde;
use crate::types::core::{Content, ImageContent, TextContent, ToolCall, ToolResult};

// — Newtype wrappers (local structs) so we satisfy Rust’s orphan rules —
// We need these because we can’t implement UniFFI’s FfiConverter directly on a type alias.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRequestToolCall(#[serde(with = "tool_result_serde")] pub ToolResult<ToolCall>);

impl ToolRequestToolCall {
    pub fn as_result(&self) -> &ToolResult<ToolCall> {
        &self.0
    }
}
impl std::ops::Deref for ToolRequestToolCall {
    type Target = ToolResult<ToolCall>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<Result<ToolCall, crate::types::core::ToolError>> for ToolRequestToolCall {
    fn from(res: Result<ToolCall, crate::types::core::ToolError>) -> Self {
        ToolRequestToolCall(res)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResponseToolResult(
    #[serde(with = "tool_result_serde")] pub ToolResult<Vec<Content>>,
);

impl ToolResponseToolResult {
    pub fn as_result(&self) -> &ToolResult<Vec<Content>> {
        &self.0
    }
}
impl std::ops::Deref for ToolResponseToolResult {
    type Target = ToolResult<Vec<Content>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<Result<Vec<Content>, crate::types::core::ToolError>> for ToolResponseToolResult {
    fn from(res: Result<Vec<Content>, crate::types::core::ToolError>) -> Self {
        ToolResponseToolResult(res)
    }
}

// — Register the newtypes with UniFFI, converting via JSON strings —
// UniFFI’s FFI layer supports only primitive buffers (here String), so we JSON-serialize
// through our `tool_result_serde` to preserve the same success/error schema on both sides.

uniffi::custom_type!(ToolRequestToolCall, String, {
    lower: |obj| {
        serde_json::to_string(&obj.0).unwrap()
    },
    try_lift: |val| {
        Ok(serde_json::from_str(&val).unwrap() )
    },
});

uniffi::custom_type!(ToolResponseToolResult, String, {
    lower: |obj| {
        serde_json::to_string(&obj.0).unwrap()
    },
    try_lift: |val| {
        Ok(serde_json::from_str(&val).unwrap() )
    },
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub id: String,
    pub tool_call: ToolRequestToolCall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    pub id: String,
    pub tool_result: ToolResponseToolResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ThinkingContent {
    pub thinking: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct RedactedThinkingContent {
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
/// Content passed inside a message, which can be both simple content and tool content
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
    ToolReq(ToolRequest),
    ToolResp(ToolResponse),
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

    pub fn tool_request<S: Into<String>>(id: S, tool_call: ToolRequestToolCall) -> Self {
        MessageContent::ToolReq(ToolRequest {
            id: id.into(),
            tool_call,
        })
    }

    pub fn tool_response<S: Into<String>>(id: S, tool_result: ToolResponseToolResult) -> Self {
        MessageContent::ToolResp(ToolResponse {
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
        if let MessageContent::ToolReq(ref tool_request) = self {
            Some(tool_request)
        } else {
            None
        }
    }

    pub fn as_tool_response(&self) -> Option<&ToolResponse> {
        if let MessageContent::ToolResp(ref tool_response) = self {
            Some(tool_response)
        } else {
            None
        }
    }

    pub fn as_tool_response_text(&self) -> Option<String> {
        if let Some(tool_response) = self.as_tool_response() {
            if let Ok(contents) = &tool_response.tool_result.0 {
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
        if let Self::ToolReq(r) = self {
            Some(&r.id)
        } else {
            None
        }
    }

    pub fn as_tool_response_id(&self) -> Option<&str> {
        if let Self::ToolResp(r) = self {
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
        matches!(self, Self::ToolReq(_))
    }
    pub fn is_tool_response(&self) -> bool {
        matches!(self, Self::ToolResp(_))
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
