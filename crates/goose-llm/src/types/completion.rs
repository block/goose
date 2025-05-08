// This file defines types for completion interfaces, including the request and response structures.
// Many of these are adapted based on the Goose Service API:
// https://docs.google.com/document/d/1r5vjSK3nBQU1cIRf0WKysDigqMlzzrzl_bxEE4msOiw/edit?tab=t.0

use std::collections::HashMap;
use thiserror::Error;

use serde::{Deserialize, Serialize};

use crate::{message::Message, providers::Usage};
use crate::{model::ModelConfig, providers::errors::ProviderError};

pub struct CompletionRequest<'a> {
    pub provider_name: &'a str,
    pub model_config: ModelConfig,
    pub system_preamble: &'a str,
    pub messages: &'a [Message],
    pub extensions: &'a [ExtensionConfig],
}

impl<'a> CompletionRequest<'a> {
    pub fn new(
        provider_name: &'a str,
        model_config: ModelConfig,
        system_preamble: &'a str,
        messages: &'a [Message],
        extensions: &'a [ExtensionConfig],
    ) -> Self {
        Self {
            provider_name,
            model_config,
            system_preamble,
            messages,
            extensions,
        }
    }
}

#[derive(Debug, Error)]
pub enum CompletionError {
    #[error("failed to create provider: {0}")]
    UnknownProvider(String),

    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("template rendering error: {0}")]
    Template(#[from] minijinja::Error),

    #[error("json serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("tool not found error: {0}")]
    ToolNotFound(String),
}

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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ToolApprovalMode {
    Auto,
    Manual,
    Smart,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub approval_mode: ToolApprovalMode,
}

impl ToolConfig {
    pub fn new(
        name: &str,
        description: &str,
        input_schema: serde_json::Value,
        approval_mode: ToolApprovalMode,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
            approval_mode,
        }
    }

    /// Convert the tool config to a core tool
    pub fn to_core_tool(&self, name: Option<&str>) -> super::core::Tool {
        let tool_name = name.unwrap_or(&self.name);
        super::core::Tool::new(
            tool_name,
            self.description.clone(),
            self.input_schema.clone(),
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionConfig {
    name: String,
    instructions: Option<String>,
    tools: Vec<ToolConfig>,
}

impl ExtensionConfig {
    pub fn new(name: String, instructions: Option<String>, tools: Vec<ToolConfig>) -> Self {
        Self {
            name,
            instructions,
            tools,
        }
    }

    /// Convert the tools to core tools with the extension name as a prefix
    pub fn get_prefixed_tools(&self) -> Vec<super::core::Tool> {
        self.tools
            .iter()
            .map(|tool| {
                let name = format!("{}__{}", self.name, tool.name);
                tool.to_core_tool(Some(&name))
            })
            .collect()
    }

    /// Get a map of prefixed tool names to their approval modes
    pub fn get_prefixed_tool_configs(&self) -> HashMap<String, ToolConfig> {
        self.tools
            .iter()
            .map(|tool| {
                let name = format!("{}__{}", self.name, tool.name);
                (name, tool.clone())
            })
            .collect()
    }
}
