use std::{collections::HashMap, time::Instant};

use anyhow::Result;
use chrono::Utc;
use serde_json::Value;

use crate::{
    message::{Message, MessageContent},
    prompt_template,
    providers::create,
    types::{
        completion::{
            CompletionError, CompletionRequest, CompletionResponse, ExtensionConfig,
            RuntimeMetrics, ToolApprovalMode, ToolConfig,
        },
        core::ToolCall,
    },
};

#[uniffi::export]
pub fn print_messages(messages: Vec<Message>) {
    for msg in messages {
        println!("[{:?} @ {}] {:?}", msg.role, msg.created, msg.content);
    }
}

/// Public API for the Goose LLM completion function
#[uniffi::export(async_runtime = "tokio")]
pub async fn completion(req: CompletionRequest) -> Result<CompletionResponse, CompletionError> {
    let start_total = Instant::now();

    let provider = create(
        &req.provider_name,
        req.provider_config.clone(),
        req.model_config.clone(),
    )
    .map_err(|_| CompletionError::UnknownProvider(req.provider_name.to_string()))?;

    let system_prompt = construct_system_prompt(
        &req.system_preamble,
        &req.system_prompt_override,
        &req.extensions,
    )?;
    let tools = collect_prefixed_tools(&req.extensions);

    // Call the LLM provider
    let start_provider = Instant::now();
    let mut response = provider
        .complete(
            &system_prompt,
            &req.messages,
            &tools,
            req.request_id.as_deref(),
        )
        .await?;
    let provider_elapsed_sec = start_provider.elapsed().as_secs_f32();
    let usage_tokens = response.usage.total_tokens;

    let tool_configs = collect_prefixed_tool_configs(&req.extensions);
    update_needs_approval_for_tool_calls(&mut response.message, &tool_configs)?;

    Ok(CompletionResponse::new(
        response.message,
        response.model,
        response.usage,
        calculate_runtime_metrics(start_total, provider_elapsed_sec, usage_tokens),
    ))
}

/// Render the global `system.md` template with the provided context.
fn construct_system_prompt(
    preamble: &Option<String>,
    prompt_override: &Option<String>,
    extensions: &[ExtensionConfig],
) -> Result<String, CompletionError> {
    // If both system_preamble and system_prompt_override are provided, then prompt_override takes precedence
    // and we don't render the template using preamble and extensions. Just return the prompt_override as is.
    if prompt_override.is_some() {
        return Ok(prompt_override.clone().unwrap());
    }

    let system_preamble = {
        if let Some(p) = preamble {
            p
        } else {
            "You are a helpful assistant."
        }
    };

    let mut context: HashMap<&str, Value> = HashMap::new();
    context.insert("system_preamble", Value::String(system_preamble.to_owned()));
    context.insert("extensions", serde_json::to_value(extensions)?);
    context.insert(
        "current_date",
        Value::String(Utc::now().format("%Y-%m-%d").to_string()),
    );

    Ok(prompt_template::render_global_file("system.md", &context)?)
}

/// Determine if a tool call requires manual approval.
fn determine_needs_approval(config: &ToolConfig, _call: &ToolCall) -> bool {
    match config.approval_mode {
        ToolApprovalMode::Auto => false,
        ToolApprovalMode::Manual => true,
        ToolApprovalMode::Smart => {
            // TODO: Implement smart approval logic later
            true
        }
    }
}

/// Set `needs_approval` on every tool call in the message.
/// Returns a `ToolNotFound` error if the corresponding `ToolConfig` is missing.
pub fn update_needs_approval_for_tool_calls(
    message: &mut Message,
    tool_configs: &HashMap<String, ToolConfig>,
) -> Result<(), CompletionError> {
    for content in &mut message.content.iter_mut() {
        if let MessageContent::ToolReq(req) = content {
            if let Ok(call) = &mut req.tool_call.0 {
                // Provide a clear error message when the tool config is missing
                let config = tool_configs.get(&call.name).ok_or_else(|| {
                    CompletionError::ToolNotFound(format!(
                        "could not find tool config for '{}'",
                        call.name
                    ))
                })?;
                let needs_approval = determine_needs_approval(config, call);
                call.set_needs_approval(needs_approval);
            }
        }
    }
    Ok(())
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
    provider_elapsed_sec: f32,
    token_count: Option<i32>,
) -> RuntimeMetrics {
    let total_ms = total_start.elapsed().as_secs_f32();
    let tokens_per_sec = token_count.and_then(|toks| {
        if provider_elapsed_sec > 0.0 {
            Some(toks as f64 / (provider_elapsed_sec as f64))
        } else {
            None
        }
    });
    RuntimeMetrics::new(total_ms, provider_elapsed_sec, tokens_per_sec)
}
