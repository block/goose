use anyhow::Result;
use std::collections::HashSet;

use goose::config::PermissionManager;
use goose::message::{Message, MessageContent, ToolRequest};
use goose::model::ModelConfig;
use goose::providers::base::ProviderUsage;
use goose::providers::create;
use goose::providers::errors::ProviderError;
use mcp_core::tool::Tool;

use goose::permission::permission_judge::{check_tool_permissions, PermissionCheckResult};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    message: Message,
    usage: ProviderUsage
}

impl CompletionResponse {
    pub fn new(message: Message, usage: ProviderUsage) -> Self {
        Self {
            message,
            usage,
        }
    }
}

/// Public API for the Goose LLM completion function
pub async fn completion(
    provider: &str,
    model_config: ModelConfig,
    system_preamble: &str,
    messages: &[Message],
    tools: &[Tool]
) -> Result<CompletionResponse, ProviderError> {
    let provider = create(provider, model_config).unwrap();
    let system_prompt = construct_system_prompt(system_preamble, tools);

    let (response, usage) = provider.complete(&system_prompt, messages, tools).await?;
    let mut result = CompletionResponse::new(response.clone(), usage.clone());

    Ok(result)
}


fn get_parameter_names(tool: &Tool) -> Vec<String> {
    tool.input_schema
        .get("properties")
        .and_then(|props| props.as_object())
        .map(|props| props.keys().cloned().collect())
        .unwrap_or_default()
}

fn construct_system_prompt(system_preamble: &str, tools: &[Tool]) -> String {
    let mut system_prompt = system_preamble.to_string();
    if !tools.is_empty() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str("Tools available:\n");
        for tool in tools {
            system_prompt.push_str(&format!(
                "## {}\nDescription: {}\nParameters: {:?}\n",
                tool.name,
                tool.description,
                get_parameter_names(tool)
            ));
        }
    } else {
        system_prompt.push_str("\n\n");
        system_prompt.push_str("No tools available.\n");
    }
    system_prompt
}
