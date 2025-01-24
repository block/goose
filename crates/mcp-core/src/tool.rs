/// Tools represent a routine that a server can execute
/// Tool calls represent requests from the client to execute one
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool that can be used by a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The name of the tool
    pub name: String,
    /// A description of what the tool does
    pub description: String,
    /// A JSON Schema object defining the expected parameters for the tool
    pub input_schema: Value,
}

impl Tool {
    /// Create a new tool with the given name and description
    pub fn new<N, D>(name: N, description: D, input_schema: Value) -> Self
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
    pub arguments: Value,
}

impl ToolCall {
    /// Create a new ToolUse with the given name and parameters
    pub fn new<S: Into<String>>(name: S, arguments: Value) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}
