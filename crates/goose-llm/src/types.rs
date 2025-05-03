use goose::message::Message;
use goose::providers::base::ProviderUsage;
use mcp_core::tool::Tool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    message: Message,
    usage: ProviderUsage,
    runtime_metrics: RuntimeMetrics,
}

impl CompletionResponse {
    pub fn new(message: Message, usage: ProviderUsage, runtime_metrics: RuntimeMetrics) -> Self {
        Self {
            message,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
