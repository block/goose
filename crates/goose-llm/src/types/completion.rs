// This file defines types for completion interfaces, including the request and response structures.
// Many of these are adapted based on the Goose Service API:
// https://docs.google.com/document/d/1r5vjSK3nBQU1cIRf0WKysDigqMlzzrzl_bxEE4msOiw/edit?tab=t.0

use std::collections::HashMap;
use thiserror::Error;

use serde::{Deserialize, Serialize};

use crate::types::json_value_ffi::JsonValueFfi;
use crate::{message::Message, providers::Usage};
use crate::{model::ModelConfig, providers::errors::ProviderError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub provider_name: String,
    pub provider_config: serde_json::Value,
    pub model_config: ModelConfig,
    pub system_preamble: Option<String>,
    pub system_prompt_override: Option<String>,
    pub messages: Vec<Message>,
    pub extensions: Vec<ExtensionConfig>,
    pub request_id: Option<String>,
}

impl CompletionRequest {
    pub fn new(
        provider_name: String,
        provider_config: serde_json::Value,
        model_config: ModelConfig,
        system_preamble: Option<String>,
        system_prompt_override: Option<String>,
        messages: Vec<Message>,
        extensions: Vec<ExtensionConfig>,
    ) -> Self {
        Self {
            provider_name,
            provider_config,
            model_config,
            system_prompt_override,
            system_preamble,
            messages,
            extensions,
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

#[allow(clippy::too_many_arguments)]
#[uniffi::export(default(system_preamble = None,  system_prompt_override = None))]
pub fn create_completion_request(
    provider_name: &str,
    provider_config: JsonValueFfi,
    model_config: ModelConfig,
    system_preamble: Option<String>,
    system_prompt_override: Option<String>,
    messages: Vec<Message>,
    extensions: Vec<ExtensionConfig>,
    request_id: Option<String>,
) -> CompletionRequest {
    let mut request = CompletionRequest::new(
        provider_name.to_string(),
        provider_config,
        model_config,
        system_preamble,
        system_prompt_override,
        messages,
        extensions,
    );

    if let Some(req_id) = request_id {
        request = request.with_request_id(req_id);
    }

    request
}

uniffi::custom_type!(CompletionRequest, String, {
    lower: |tc: &CompletionRequest| {
        serde_json::to_string(&tc).unwrap()
    },
    try_lift: |s: String| {
        Ok(serde_json::from_str(&s).unwrap())
    },
});

// https://mozilla.github.io/uniffi-rs/latest/proc_macro/errors.html
#[derive(Debug, Error, uniffi::Error)]
#[uniffi(flat_error)]
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

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
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

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct RuntimeMetrics {
    pub total_time_sec: f32,
    pub total_time_sec_provider: f32,
    pub tokens_per_second: Option<f64>,
}

impl RuntimeMetrics {
    pub fn new(
        total_time_sec: f32,
        total_time_sec_provider: f32,
        tokens_per_second: Option<f64>,
    ) -> Self {
        Self {
            total_time_sec,
            total_time_sec_provider,
            tokens_per_second,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
pub enum ToolApprovalMode {
    Auto,
    Manual,
    Smart,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValueFfi,
    pub approval_mode: ToolApprovalMode,
}

impl ToolConfig {
    pub fn new(
        name: &str,
        description: &str,
        input_schema: JsonValueFfi,
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

#[uniffi::export]
pub fn create_tool_config(
    name: &str,
    description: &str,
    input_schema: JsonValueFfi,
    approval_mode: ToolApprovalMode,
) -> ToolConfig {
    ToolConfig::new(name, description, input_schema, approval_mode)
}

// — Register the newtypes with UniFFI, converting via JSON strings —

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
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
