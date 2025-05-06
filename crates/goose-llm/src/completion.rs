use std::{collections::HashMap, time::Instant};

use anyhow::Result;
use chrono::Utc;
use serde_json::Value;

use crate::{
    message::{Message, MessageContent},
    prompt_template,
    providers::create,
    types::completion::{
        CompletionError, CompletionRequest, CompletionResponse, ExtensionConfig, RuntimeMetrics,
        ToolApprovalMode, ToolConfig,
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
pub async fn completion(req: CompletionRequest<'_>) -> Result<CompletionResponse, CompletionError> {
    let start_total = Instant::now();

    let provider = create(req.provider_name, req.model_config)
        .map_err(|_| CompletionError::UnknownProvider(req.provider_name.to_string()))?;

    let system_prompt = construct_system_prompt(req.system_preamble, req.extensions)?;
    let tools = collect_prefixed_tools(req.extensions);

    // Call the LLM provider
    let start_provider = Instant::now();
    let mut response = provider
        .complete(&system_prompt, req.messages, &tools)
        .await?;
    let usage_tokens = response.usage.total_tokens;

    let tool_configs = collect_prefixed_tool_configs(req.extensions);
    update_needs_approval_for_tool_calls(&mut response.message, &tool_configs);

    Ok(CompletionResponse::new(
        response.message,
        response.model,
        response.usage,
        calculate_runtime_metrics(start_total, start_provider, usage_tokens),
    ))
}

/// Render the global `system.md` template with the provided context.
fn construct_system_prompt(
    system_preamble: &str,
    extensions: &[ExtensionConfig],
) -> Result<String, CompletionError> {
    let mut context: HashMap<&str, Value> = HashMap::new();
    context.insert("system_preamble", Value::String(system_preamble.to_owned()));
    context.insert("extensions", serde_json::to_value(extensions)?);
    context.insert(
        "current_date",
        Value::String(Utc::now().format("%Y-%m-%d").to_string()),
    );

    Ok(prompt_template::render_global_file("system.md", &context)?)
}

/// Collect all `Tool` instances from the extensions.
fn collect_prefixed_tools(extensions: &[ExtensionConfig]) -> Vec<crate::types::core::Tool> {
    extensions
        .iter()
        .flat_map(|ext| ext.get_prefixed_tools())
        .collect()
}

/// Collect all `ToolConfig` entries from the extensions into a map.
fn collect_prefixed_tool_configs(extensions: &[ExtensionConfig]) -> HashMap<String, ToolConfig> {
    extensions
        .iter()
        .flat_map(|ext| ext.get_prefixed_tool_configs())
        .collect()
}

/// Compute runtime metrics for the request.
fn calculate_runtime_metrics(
    total_start: Instant,
    provider_start: Instant,
    token_count: Option<i32>,
) -> RuntimeMetrics {
    let total_ms = total_start.elapsed().as_millis();
    let provider_ms = provider_start.elapsed().as_millis();
    let tokens_per_sec = token_count.and_then(|toks| {
        if provider_ms > 0 {
            Some(toks as f64 / (provider_ms as f64 / 1_000.0))
        } else {
            None
        }
    });
    RuntimeMetrics::new(total_ms, provider_ms, tokens_per_sec)
}
