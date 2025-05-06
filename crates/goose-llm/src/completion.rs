use std::{collections::HashMap, time::Instant};

use anyhow::Result;
use chrono::Utc;
use serde_json::Value;

use crate::{
    message::{Message, MessageContent},
    model::ModelConfig,
    prompt_template,
    providers::{create, errors::ProviderError},
    types::completion::{
        CompletionResponse, ExtensionConfig, RuntimeMetrics, ToolApprovalMode, ToolConfig,
    },
};

/// Set `needs_approval` on *every* tool call in the message based on approval mode.
pub fn update_needs_approval_for_tool_calls(
    message: &mut Message,
    tool_configs: &HashMap<String, ToolConfig>,
) {
    for content in message.content.iter_mut() {
        if let MessageContent::ToolRequest(req) = content {
            if let Ok(call) = &mut req.tool_call {
                let needs = match tool_configs.get(&call.name) {
                    Some(cfg) => match cfg.approval_mode {
                        ToolApprovalMode::Auto => false,
                        ToolApprovalMode::Manual => true,
                        ToolApprovalMode::Smart => true, // TODO: implement smart approval later
                    },
                    None => call.needs_approval, // unknown tool: leave flag unchanged
                };

                call.set_needs_approval(needs);
            }
        }
    }
}

/// Public API for the Goose LLM completion function
pub async fn completion(
    provider: &str,
    model_config: ModelConfig,
    system_preamble: &str,
    messages: &[Message],
    extensions: &[ExtensionConfig],
) -> Result<CompletionResponse, ProviderError> {
    let start_total = Instant::now();
    let provider = create(provider, model_config).unwrap();
    let system_prompt = construct_system_prompt(system_preamble, extensions);
    // println!("\nSystem prompt: {}\n", system_prompt);

    let tools = extensions
        .iter()
        .flat_map(|ext| ext.get_prefixed_tools())
        .collect::<Vec<_>>();

    let start_provider = Instant::now();
    let mut response = provider.complete(&system_prompt, messages, &tools).await?;
    let total_time_ms_provider = start_provider.elapsed().as_millis();
    let tokens_per_second = response.usage.total_tokens.and_then(|toks| {
        if total_time_ms_provider > 0 {
            Some(toks as f64 / (total_time_ms_provider as f64 / 1000.0))
        } else {
            None
        }
    });

    let tool_configs: HashMap<String, ToolConfig> = extensions
        .iter()
        .flat_map(|ext| ext.get_prefixed_tool_configs().into_iter())
        .collect();

    update_needs_approval_for_tool_calls(&mut response.message, &tool_configs);

    let total_time_ms = start_total.elapsed().as_millis();
    Ok(CompletionResponse::new(
        response.message,
        response.model,
        response.usage,
        RuntimeMetrics::new(total_time_ms, total_time_ms_provider, tokens_per_second),
    ))
}

fn construct_system_prompt(system_preamble: &str, extensions: &[ExtensionConfig]) -> String {
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
