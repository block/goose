use serde::{Deserialize, Serialize};

use crate::message::Message;
use crate::providers::Usage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub message: Message,
    pub model: String,
    pub usage: Usage,
    pub runtime_metrics: RuntimeMetrics,
}

impl CompletionResponse {
    pub fn new(
        message: Message,
        model: String,
        usage: Usage,
        runtime_metrics: RuntimeMetrics,
    ) -> Self {
        Self {
            message,
            model,
            usage,
            runtime_metrics,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    pub total_time_ms: u128,
    pub total_time_ms_provider: u128,
    pub tokens_per_second: Option<f64>,
}

impl RuntimeMetrics {
    pub fn new(
        total_time_ms: u128,
        total_time_ms_provider: u128,
        tokens_per_second: Option<f64>,
    ) -> Self {
        Self {
            total_time_ms,
            total_time_ms_provider,
            tokens_per_second,
        }
    }
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

#[derive(Debug, Clone, Serialize)]
pub struct Extension {
    name: String,
    instructions: Option<String>,
    tools: Vec<Tool>,
}

impl Extension {
    pub fn new(name: String, instructions: Option<String>, tools: Vec<Tool>) -> Self {
        Self {
            name,
            instructions,
            tools,
        }
    }

    pub fn get_prefixed_tools(&self) -> Vec<Tool> {
        self.tools
            .iter()
            .map(|tool| {
                let mut prefixed_tool = tool.clone();
                prefixed_tool.name = format!("{}__{}", self.name, tool.name);
                prefixed_tool
            })
            .collect()
    }
}
