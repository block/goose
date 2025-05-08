use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::de::Deserializer;
use serde_json::ser::Serializer;
use smallvec::SmallVec;
use std::{iter::FromIterator, ops::Deref};
use thiserror::Error;

use crate::{tool_result_serde, JsonValueFfi};

#[derive(Error, Debug, Clone, Deserialize, Serialize, PartialEq, uniffi::Error)]
pub enum ToolError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Schema error: {0}")]
    SchemaError(String),
    #[error("Tool not found: {0}")]
    NotFound(String),
}

pub type ToolResult<T> = std::result::Result<T, ToolError>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub name: String,
    pub params: JsonValueFfi,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessageContent {
    Text(TextContent),
    ToolReq(ToolRequest), // both enum variant and struct cannot have the same name ToolRequest
    ToolResp(ToolResponse), // both enum variant and struct cannot have the same name Tool
}

impl MessageContent {
    pub fn text<S: Into<String>>(text: S) -> Self {
        MessageContent::Text(TextContent { text: text.into() })
    }

    pub fn tool_request<S: Into<String>>(id: S, tool_call: ToolRequestToolCall) -> Self {
        MessageContent::ToolReq(ToolRequest {
            id: id.into(),
            tool_call,
        })
    }

    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(text) => Some(&text.text),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    pub text: String,
}

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Contents(SmallVec<[MessageContent; 2]>);

impl Contents {
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, MessageContent> {
        self.0.iter_mut()
    }

    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.0.iter().filter_map(|c| c.as_text())
    }

    pub fn concat_text_str(&self) -> String {
        self.texts().collect::<Vec<_>>().join("\n")
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

impl Deref for Contents {
    type Target = [MessageContent];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// — Register the contents type with UniFFI, converting to/from Vec<MessageContent> —
// We need to do this because UniFFI’s FFI layer supports only primitive buffers (here Vec<u8>),
uniffi::custom_type!(Contents, Vec<MessageContent>, {
    lower: |contents: &Contents| {
        contents.0.to_vec()
    },
    try_lift: |contents: Vec<MessageContent>| {
        Ok(Contents::from(contents))
    },
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: Role,
    pub created: i64,
    pub content: Contents,
}

// — Newtype wrappers (local structs) so we satisfy Rust’s orphan rules —
// We need these because we can’t implement UniFFI’s FfiConverter directly on a type alias.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRequestToolCall(pub ToolResult<ToolCall>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResponseToolResult(pub ToolResult<Vec<TextContent>>);

// — Register the newtypes with UniFFI, converting via JSON strings —
// UniFFI’s FFI layer supports only primitive buffers (here String), so we JSON-serialize
// through our `tool_result_serde` to preserve the same success/error schema on both sides.

uniffi::custom_type!(ToolRequestToolCall, String, {
    lower: |wrapper: &ToolRequestToolCall| {
        let mut buf = Vec::new();
        {
            let mut ser = Serializer::new(&mut buf);
            // note the borrow on wrapper.0
            tool_result_serde::serialize(&wrapper.0, &mut ser)
                .expect("ToolRequestToolCall serialization failed");
        }
        String::from_utf8(buf).expect("ToolRequestToolCall produced invalid UTF-8")
    },
    try_lift: |s: String| {
        let mut de = Deserializer::from_str(&s);
        let result = tool_result_serde::deserialize(&mut de)
            .map_err(anyhow::Error::new)?;
        Ok(ToolRequestToolCall(result))
    },
});

uniffi::custom_type!(ToolResponseToolResult, String, {
    lower: |wrapper: &ToolResponseToolResult| {
        let mut buf = Vec::new();
        {
            let mut ser = Serializer::new(&mut buf);
            tool_result_serde::serialize(&wrapper.0, &mut ser)
                .expect("ToolResponseToolResult serialization failed");
        }
        String::from_utf8(buf).expect("ToolResponseToolResult produced invalid UTF-8")
    },
    try_lift: |s: String| {
        let mut de = Deserializer::from_str(&s);
        let result = tool_result_serde::deserialize(&mut de)
            .map_err(anyhow::Error::new)?;
        Ok(ToolResponseToolResult(result))
    },
});

// --- Completion Types ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolConfig {
    pub name: String,
    pub input_schema: serde_json::Value,
}

impl ToolConfig {
    pub fn new(name: &str, input_schema: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            input_schema,
        }
    }
}

uniffi::custom_type!(ToolConfig, String, {
    lower: |tc: &ToolConfig| {
        serde_json::to_string(&tc).unwrap()
    },
    try_lift: |s: String| {
        Ok(serde_json::from_str(&s).unwrap())
    },
});

#[uniffi::export]
pub fn create_tool_config(name: &str, input_schema: JsonValueFfi) -> ToolConfig {
    ToolConfig::new(name, input_schema.into())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ExtensionConfig {
    name: String,
    tools: Vec<ToolConfig>,
}

impl ExtensionConfig {
    pub fn new(name: String, tools: Vec<ToolConfig>) -> Self {
        Self { name, tools }
    }
}
