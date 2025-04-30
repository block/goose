use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::base::ProviderUsage;
use goose::providers::create;
use goose::providers::errors::ProviderError;
use mcp_core::tool::Tool;

use crate::prompt_template;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    message: Message,
    usage: ProviderUsage,
}

impl CompletionResponse {
    pub fn new(message: Message, usage: ProviderUsage) -> Self {
        Self { message, usage }
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

/// Public API for the Goose LLM completion function
pub async fn completion(
    provider: &str,
    model_config: ModelConfig,
    system_preamble: &str,
    messages: &[Message],
    extensions: &[Extension],
) -> Result<CompletionResponse, ProviderError> {
    let provider = create(provider, model_config).unwrap();
    let system_prompt = construct_system_prompt(system_preamble, extensions);
    // println!("\nSystem prompt: {}\n", system_prompt);

    let tools = extensions
        .iter()
        .flat_map(|ext| ext.get_prefixed_tools())
        .collect::<Vec<_>>();
    let (response, usage) = provider.complete(&system_prompt, messages, &tools).await?;
    let result = CompletionResponse::new(response.clone(), usage.clone());

    Ok(result)
}

fn construct_system_prompt(system_preamble: &str, extensions: &[Extension]) -> String {
    let mut context: HashMap<&str, Value> = HashMap::new();

    context.insert(
        "system_preamble",
        Value::String(system_preamble.to_string()),
    );
    context.insert("extensions", serde_json::to_value(extensions).unwrap());

    let current_date_time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    context.insert("current_date_time", Value::String(current_date_time));

    prompt_template::render_global_file("system.md", &context).expect("Prompt should render")
}
