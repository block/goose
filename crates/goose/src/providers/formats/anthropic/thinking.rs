use super::*;

pub fn supports_adaptive_thinking(model_name: &str) -> bool {
    let lower = model_name.to_lowercase();
    lower.contains("claude-opus-4-6") || lower.contains("claude-sonnet-4-6")
}

pub fn thinking_type(model_config: &ModelConfig) -> ThinkingType {
    let model_lower = model_config.model_name.to_lowercase();
    if !model_lower.contains("claude") {
        return ThinkingType::Disabled;
    }

    let is_adaptive_model = supports_adaptive_thinking(&model_config.model_name);
    let effort = model_config.thinking_effort();

    if effort.is_none() && legacy_thinking_budget_tokens().is_some() {
        return ThinkingType::Enabled;
    }

    match effort.unwrap_or(ThinkingEffort::Off) {
        ThinkingEffort::Off => ThinkingType::Disabled,
        _ if is_adaptive_model => ThinkingType::Adaptive,
        _ => ThinkingType::Enabled,
    }
}

pub fn thinking_effort(model_config: &ModelConfig) -> ThinkingEffort {
    model_config
        .thinking_effort()
        .unwrap_or(ThinkingEffort::High)
}

pub fn thinking_budget_tokens(model_config: &ModelConfig) -> i32 {
    if let Some(request_param) = model_config
        .request_params
        .as_ref()
        .and_then(|params| params.get("budget_tokens"))
        .and_then(|v| serde_json::from_value::<i32>(v.clone()).ok())
    {
        return request_param.max(1024);
    }

    if let Some(budget) = legacy_thinking_budget_tokens() {
        return budget;
    }

    let effort = model_config
        .thinking_effort()
        .unwrap_or(ThinkingEffort::High);
    match effort {
        ThinkingEffort::Off => 1024,
        ThinkingEffort::Low => 4000,
        ThinkingEffort::Medium => 10000,
        ThinkingEffort::High => 16000,
        ThinkingEffort::Max => 32000,
    }
}

pub(super) fn legacy_thinking_budget_tokens() -> Option<i32> {
    let config = crate::config::Config::global();
    for key in ["ANTHROPIC_THINKING_BUDGET", "CLAUDE_THINKING_BUDGET"] {
        if let Ok(budget) = config.get_param::<i32>(key) {
            return Some(budget.max(1024));
        }
    }
    None
}

pub(super) fn apply_thinking_config(
    payload: &mut Value,
    model_config: &ModelConfig,
    max_tokens: i32,
    options: AnthropicFormatOptions,
) {
    let obj = payload.as_object_mut().unwrap();
    match thinking_type(model_config) {
        ThinkingType::Adaptive => {
            obj.insert("thinking".to_string(), json!({"type": "adaptive"}));
            let effort = thinking_effort(model_config).to_string();
            obj.insert("output_config".to_string(), json!({"effort": effort}));
        }
        ThinkingType::Enabled => {
            let budget_tokens = thinking_budget_tokens(model_config);

            obj.insert("max_tokens".to_string(), json!(max_tokens + budget_tokens));
            obj.insert(
                "thinking".to_string(),
                json!({
                    "type": "enabled",
                    "budget_tokens": budget_tokens
                }),
            );
        }
        ThinkingType::Disabled => {}
    }

    if options.preserve_thinking_context {
        if !obj.contains_key("thinking") {
            let budget_tokens = thinking_budget_tokens(model_config);
            obj.insert("max_tokens".to_string(), json!(max_tokens + budget_tokens));
            obj.insert(
                "thinking".to_string(),
                json!({
                    "type": "enabled",
                    "budget_tokens": budget_tokens
                }),
            );
        }

        if let Some(thinking) = obj.get_mut("thinking").and_then(|t| t.as_object_mut()) {
            thinking.insert("clear_thinking".to_string(), json!(false));
        }
    }
}

/// Create a complete request payload for Anthropic's API
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    create_request_with_options(
        model_config,
        system,
        messages,
        tools,
        AnthropicFormatOptions::default(),
    )
}

pub fn create_request_with_options(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    options: AnthropicFormatOptions,
) -> Result<Value> {
    let options = options.for_model(model_config);
    let anthropic_messages = format_messages_with_options(messages, options);
    let tool_specs = format_tools(tools);
    let system_spec = format_system(system);

    if anthropic_messages.is_empty() {
        return Err(anyhow!("No valid messages to send to Anthropic API"));
    }

    let max_tokens = model_config.max_output_tokens();
    let mut payload = json!({
        "model": model_config.model_name,
        "messages": anthropic_messages,
        "max_tokens": max_tokens,
    });

    if !system.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("system".to_string(), json!(system_spec));
    }

    if !tool_specs.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tool_specs));
    }

    if let Some(temp) = model_config.temperature {
        payload
            .as_object_mut()
            .unwrap()
            .insert("temperature".to_string(), json!(temp));
    }

    apply_thinking_config(&mut payload, model_config, max_tokens, options);

    Ok(payload)
}
