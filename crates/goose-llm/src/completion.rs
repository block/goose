use std::{collections::HashMap, time::Instant};

use anyhow::Result;
use chrono::Utc;
use serde_json::Value;

use crate::{
    message::{FrontendToolRequest, Message, MessageContent},
    model::ModelConfig,
    prompt_template,
    providers::{create, errors::ProviderError},
    types::completion::{
        CompletionResponse, ExtensionConfig, ExtensionType, RuntimeMetrics, ToolApprovalMode,
        ToolConfig,
    },
};

/// Convert assistant tool-requests into frontend tool-requests when
/// `extension_type == Frontend`, and mark them as `needs_approval = true`.
pub fn update_tool_request_type(message: &mut Message, tool_configs: &HashMap<String, ToolConfig>) {
    for content in message.content.iter_mut() {
        // Only assistant-initiated tool calls can turn into frontend calls
        if let MessageContent::ToolRequest(req) = content {
            if let Ok(tool_call) = req.tool_call.as_mut() {
                // `as_mut()` gives us `&mut ToolCall`
                if matches!(
                    tool_configs.get(&tool_call.name).map(|c| &c.extension_type),
                    Some(ExtensionType::Frontend)
                ) {
                    // 1️⃣ flip the flag in-place
                    tool_call.set_needs_approval(true);

                    // 2️⃣ rewrite the enum variant
                    let replacement = FrontendToolRequest {
                        id: req.id.clone(),
                        tool_call: req.tool_call.clone(), // clone *after* the mutation
                    };
                    *content = MessageContent::FrontendToolRequest(replacement);
                }
            }
        }
    }
}

/// Set `needs_approval` on *every* tool call in the message.
///
/// Rules  
/// • Manual  → true  
/// • Auto    → false  
/// • Smart   → true (current spec)  
/// • Any Frontend-type tool call  → true always
pub fn update_needs_approval_for_tool_calls(
    message: &mut Message,
    tool_configs: &HashMap<String, ToolConfig>,
) {
    for content in message.content.iter_mut() {
        // cover both assistant & frontend variants
        if let MessageContent::ToolRequest(req) = content {
            if let Ok(call) = &mut req.tool_call {
                // Frontend tool calls (either by type or by config) are always manual.
                let needs = match tool_configs.get(&call.name) {
                    Some(cfg) if matches!(cfg.extension_type, ExtensionType::Frontend) => true,
                    Some(cfg) => match cfg.approval_mode {
                        ToolApprovalMode::Auto => false,
                        ToolApprovalMode::Manual | ToolApprovalMode::Smart => true,
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

    // Update tool requests to frontend tool requests based on extension config
    update_tool_request_type(&mut response.message, &tool_configs);

    // Update the `needs_approval` field in the response message
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
