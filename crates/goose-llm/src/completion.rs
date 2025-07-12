use std::{collections::HashMap, time::Instant};

use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use tracing::{debug, error, info};

use crate::{
    message::{Message, MessageContent},
    prompt_template,
    providers::{create, global_pool_manager, base::Provider},
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

/// Initialize the provider pool with default configuration
#[uniffi::export]
pub fn init_provider_pool() {
    // Initialize the provider pool with default configuration
    global_pool_manager();
    debug!("Provider pool initialized with default configuration");
}

/// Configure the provider pool with custom settings
#[uniffi::export]
pub fn configure_provider_pool(max_size: u32, max_idle_sec: u64, max_lifetime_sec: u64, max_uses: u32) {
    use std::time::Duration;
    use crate::providers::PoolConfig;
    
    let _config = PoolConfig {
        max_size: max_size as usize,
        max_idle_time: Duration::from_secs(max_idle_sec),
        max_lifetime: Duration::from_secs(max_lifetime_sec),
        max_uses: max_uses as usize,
    };
    
    global_pool_manager(); // Initialize if not already
    
    // We can't directly configure the global pool manager after it's initialized,
    // but this is useful for per-provider configuration
    info!("Provider pool configured with max_size={}, max_idle_time={}s, max_lifetime={}s, max_uses={}",
          max_size, max_idle_sec, max_lifetime_sec, max_uses);
}

/// Get statistics about the provider pool
#[uniffi::export]
pub fn get_pool_stats() -> String {
    let stats = global_pool_manager().get_all_stats();
    
    if stats.is_empty() {
        return "No active provider pools".to_string();
    }
    
    let mut result = String::new();
    for (name, stat) in stats {
        result.push_str(&format!("Pool: {}\n", name));
        result.push_str(&format!("  Created: {}\n", stat.created));
        result.push_str(&format!("  Borrowed: {}\n", stat.borrowed));
        result.push_str(&format!("  Returned: {}\n", stat.returned));
        result.push_str(&format!("  Errors: {}\n", stat.errors));
        result.push_str(&format!("  Max Pool Size: {}\n", stat.max_pool_size));
        result.push_str(&format!("  Current Pool Size: {}\n", stat.current_pool_size));
        result.push_str(&format!("  Waiting: {}\n", stat.waiting));
        result.push_str("\n");
    }
    
    result
}

/// Public API for the Goose LLM completion function
#[uniffi::export(async_runtime = "tokio")]
pub async fn completion(req: CompletionRequest) -> Result<CompletionResponse, CompletionError> {
    let start_total = Instant::now();

    let system_prompt = construct_system_prompt(
        &req.system_preamble,
        &req.system_prompt_override,
        &req.extensions,
    )?;
    let tools = collect_prefixed_tools(&req.extensions);

    // Create a pooled provider or a direct provider based on the request
    if req.use_pool {
        // Try to get a provider from the pool by calling directly on the pool manager
        let pool_manager = global_pool_manager();
        let pool = pool_manager.get_or_create_pool(
            &req.provider_name,
            req.provider_config.clone(),
            req.model_config.clone(),
        );
        
        // Clone the Arc so we can move it into the match
        let pool_clone = pool.clone();
        match pool_clone.get().await {
            Ok(pooled_provider) => {
                // Call the pooled provider
                let start_provider_time = Instant::now();
                let mut response = pooled_provider
                    .complete(&system_prompt, &req.messages, &tools)
                    .await?;
                
                let provider_elapsed_sec = start_provider_time.elapsed().as_secs_f32();
                let usage_tokens = response.usage.total_tokens;
                
                let tool_configs = collect_prefixed_tool_configs(&req.extensions);
                update_needs_approval_for_tool_calls(&mut response.message, &tool_configs)?;
                
                return Ok(CompletionResponse::new(
                    response.message,
                    response.model,
                    response.usage,
                    calculate_runtime_metrics(start_total, provider_elapsed_sec, usage_tokens),
                ));
            },
            Err(e) => {
                error!("Failed to get provider from pool: {}", e);
                // Fall back to creating a provider directly
                debug!("Falling back to direct provider creation");
            }
        }
    }
    
    // Create a provider directly (either by choice or as fallback)
    debug!("Using direct provider creation");
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
        .complete(&system_prompt, &req.messages, &tools)
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
