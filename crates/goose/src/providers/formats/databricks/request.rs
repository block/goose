use super::*;

pub(super) fn apply_claude_thinking_config(payload: &mut Value, model_config: &ModelConfig) {
    let obj = payload.as_object_mut().unwrap();

    match thinking_type(model_config) {
        ThinkingType::Adaptive => {
            obj.insert("thinking".to_string(), json!({ "type": "adaptive" }));
            obj.insert(
                "output_config".to_string(),
                json!({ "effort": thinking_effort(model_config).to_string() }),
            );
            obj.insert(
                "max_completion_tokens".to_string(),
                json!(model_config.max_output_tokens()),
            );
        }
        ThinkingType::Enabled => {
            let budget_tokens = thinking_budget_tokens(model_config);
            let max_tokens = model_config.max_output_tokens() + budget_tokens;
            obj.insert("max_tokens".to_string(), json!(max_tokens));
            obj.insert(
                "thinking".to_string(),
                json!({
                    "type": "enabled",
                    "budget_tokens": budget_tokens
                }),
            );
            obj.insert("temperature".to_string(), json!(2));
        }
        ThinkingType::Disabled => {
            if let Some(temp) = model_config.temperature {
                obj.insert("temperature".to_string(), json!(temp));
            }
            obj.insert(
                "max_completion_tokens".to_string(),
                json!(model_config.max_output_tokens()),
            );
        }
    }
}

pub fn format_tools(tools: &[Tool], model_name: &str) -> anyhow::Result<Vec<Value>> {
    let mut tool_names = std::collections::HashSet::new();
    let mut result = Vec::new();

    let is_gemini = model_name.contains("gemini");

    for tool in tools {
        if !tool_names.insert(&tool.name) {
            return Err(anyhow!("Duplicate tool name: {}", tool.name));
        }

        let has_properties = tool
            .input_schema
            .get("properties")
            .and_then(|v| v.as_object())
            .is_some_and(|p| !p.is_empty());

        let function_def = if is_gemini {
            let mut def = json!({
                "name": tool.name,
                "description": tool.description,
            });
            if has_properties {
                def["parametersJsonSchema"] = json!(tool.input_schema);
            }
            def
        } else {
            let mut def = json!({
                "name": tool.name,
                "description": tool.description,
            });
            if has_properties {
                def["parameters"] = json!(tool.input_schema);
            }
            def
        };

        result.push(json!({
            "type": "function",
            "function": function_def,
        }));
    }

    Ok(result)
}

/// Add Anthropic-style cache_control fields to the request payload for Claude models.
/// This enables prompt caching to reduce costs when using Claude via Databricks.
///
/// Cache control is added to:
/// - The system message
/// - The last two user messages (for incremental caching across turns)
/// - The last tool definition (so all tools are cached as a single prefix)
pub fn apply_cache_control_for_claude(payload: &mut Value) {
    if let Some(messages_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("messages"))
        .and_then(|messages| messages.as_array_mut())
    {
        // Add cache_control to the last two user messages for incremental caching.
        // The last message gets cached so future turns can read from it.
        // The second-to-last user message is also cached to read from the previous cache.
        let mut user_count = 0;
        for message in messages_spec.iter_mut().rev() {
            if message.get("role") == Some(&json!("user")) {
                if let Some(content) = message.get_mut("content") {
                    if let Some(content_str) = content.as_str() {
                        *content = json!([{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]);
                    } else if let Some(content_array) = content.as_array_mut() {
                        // Content is already an array, add cache_control to the last element
                        if let Some(last_content) = content_array.last_mut() {
                            if let Some(obj) = last_content.as_object_mut() {
                                obj.insert(
                                    "cache_control".to_string(),
                                    json!({ "type": "ephemeral" }),
                                );
                            }
                        }
                    }
                }
                user_count += 1;
                if user_count >= 2 {
                    break;
                }
            }
        }

        // Add cache_control to the system message
        if let Some(system_message) = messages_spec
            .iter_mut()
            .find(|msg| msg.get("role") == Some(&json!("system")))
        {
            if let Some(content) = system_message.get_mut("content") {
                if let Some(content_str) = content.as_str() {
                    *system_message = json!({
                        "role": "system",
                        "content": [{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]
                    });
                }
            }
        }
    }

    // Add cache_control to the last tool definition
    if let Some(tools_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("tools"))
        .and_then(|tools| tools.as_array_mut())
    {
        if let Some(last_tool) = tools_spec.last_mut() {
            if let Some(function) = last_tool.get_mut("function") {
                if let Some(obj) = function.as_object_mut() {
                    obj.insert("cache_control".to_string(), json!({ "type": "ephemeral" }));
                }
            }
        }
    }
}

/// Validates and fixes tool schemas to ensure they have proper parameter structure.
/// If parameters exist, ensures they have properties and required fields, or removes parameters entirely.
pub fn validate_tool_schemas(tools: &mut [Value]) {
    for tool in tools.iter_mut() {
        if let Some(function) = tool.get_mut("function") {
            if let Some(parameters) = function.get_mut("parameters") {
                if parameters.is_object() {
                    ensure_valid_json_schema(parameters);
                }
            }
        }
    }
}

/// Ensures that the given JSON value follows the expected JSON Schema structure.
fn ensure_valid_json_schema(schema: &mut Value) {
    if let Some(params_obj) = schema.as_object_mut() {
        // Check if this is meant to be an object type schema
        let is_object_type = params_obj
            .get("type")
            .and_then(|t| t.as_str())
            .is_none_or(|t| t == "object"); // Default to true if no type is specified

        // Only apply full schema validation to object types
        if is_object_type {
            // Ensure required fields exist with default values
            params_obj.entry("properties").or_insert_with(|| json!({}));
            params_obj.entry("required").or_insert_with(|| json!([]));
            params_obj.entry("type").or_insert_with(|| json!("object"));

            // Recursively validate properties if it exists
            if let Some(properties) = params_obj.get_mut("properties") {
                if let Some(properties_obj) = properties.as_object_mut() {
                    for (_key, prop) in properties_obj.iter_mut() {
                        if prop.is_object()
                            && prop.get("type").and_then(|t| t.as_str()) == Some("object")
                        {
                            ensure_valid_json_schema(prop);
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
) -> anyhow::Result<Value, Error> {
    if model_config.model_name.starts_with("o1-mini") {
        return Err(anyhow!(
            "o1-mini model is not currently supported since goose uses tool calling and o1-mini does not support it. Please use o1 or o3 models instead."
        ));
    }

    let (model_name, legacy_reasoning_effort) = extract_reasoning_effort(&model_config.model_name);
    let is_openai_reasoning_model = is_openai_responses_model(&model_name);
    let reasoning_effort = if is_openai_reasoning_model {
        model_config
            .thinking_effort()
            .map_or(legacy_reasoning_effort, |effort| {
                openai_reasoning_effort_for_thinking(&model_name, effort)
            })
    } else {
        None
    };

    let system_message = DatabricksMessage {
        role: "system".to_string(),
        content: system.into(),
        tool_calls: None,
        tool_call_id: None,
    };

    let messages_spec = format_messages(messages, image_format);
    let mut tools_spec = if !tools.is_empty() {
        format_tools(tools, &model_config.model_name)?
    } else {
        vec![]
    };

    // Validate tool schemas
    validate_tool_schemas(&mut tools_spec);

    let mut messages_array = vec![system_message];
    messages_array.extend(messages_spec);

    let mut payload = json!({
        "model": model_name,
        "messages": messages_array
    });

    if let Some(effort) = reasoning_effort {
        payload
            .as_object_mut()
            .unwrap()
            .insert("reasoning_effort".to_string(), json!(effort));
    }

    if !tools_spec.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tools_spec));
    }

    if is_claude_model(&model_config.model_name) {
        apply_claude_thinking_config(&mut payload, model_config);
    } else {
        // open ai reasoning models currently don't support temperature
        if !is_openai_reasoning_model {
            if let Some(temp) = model_config.temperature {
                payload
                    .as_object_mut()
                    .unwrap()
                    .insert("temperature".to_string(), json!(temp));
            }
        }

        payload.as_object_mut().unwrap().insert(
            "max_completion_tokens".to_string(),
            json!(model_config.max_output_tokens()),
        );
    }

    // Apply cache control for Claude models to enable prompt caching
    if is_claude_model(&model_config.model_name) {
        apply_cache_control_for_claude(&mut payload);
    }

    // Add request_params to the payload (e.g., anthropic_beta for extended context)
    if let Some(params) = &model_config.request_params {
        if let Some(obj) = payload.as_object_mut() {
            for (key, value) in params {
                if key == "thinking_effort" {
                    continue;
                }
                obj.insert(key.clone(), value.clone());
            }
        }
    }

    Ok(payload)
}
