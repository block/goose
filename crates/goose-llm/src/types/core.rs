// This file defines core types that require serialization to
// construct payloads for LLM model providers and work with MCPs.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Content {
    Text(TextContent),
    Image(ImageContent),
}

impl Content {
    pub fn text<S: Into<String>>(text: S) -> Self {
        Content::Text(TextContent { text: text.into() })
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        Content::Image(ImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
        })
    }

    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Content::Text(text) => Some(&text.text),
            _ => None,
        }
    }

    /// Get the image content if this is an ImageContent variant
    pub fn as_image(&self) -> Option<(&str, &str)> {
        match self {
            Content::Image(image) => Some((&image.data, &image.mime_type)),
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
pub struct ImageContent {
    pub data: String,
    pub mime_type: String,
}

/// A tool that can be used by a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The name of the tool
    pub name: String,
    /// A description of what the tool does
    pub description: String,
    /// A JSON Schema object defining the expected parameters for the tool
    pub input_schema: serde_json::Value,
}

impl Tool {
    /// Create a new tool with the given name and description
    pub fn new<N, D>(name: N, description: D, input_schema: serde_json::Value) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        Tool {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// A tool call request that an extension can execute
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    /// The name of the tool to execute
    pub name: String,
    /// The parameters for the execution
    pub arguments: serde_json::Value,
    /// Whether the tool call needs approval before execution. Default is false.
    pub needs_approval: bool,
}

impl ToolCall {
    /// Create a new ToolUse with the given name and parameters
    pub fn new<S: Into<String>>(name: S, arguments: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            arguments,
            needs_approval: false,
        }
    }

    /// Set needs_approval field
    pub fn set_needs_approval(&mut self, flag: bool) {
        self.needs_approval = flag;
    }
}

#[non_exhaustive]
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
