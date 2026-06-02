use super::*;

pub(super) fn extract_content_and_signature(
    delta_content: Option<&DeltaContent>,
) -> (Option<String>, Option<String>) {
    match delta_content {
        Some(DeltaContent::String(s)) => (Some(s.clone()), None),
        Some(DeltaContent::Array(parts)) => {
            let text_parts: Vec<_> = parts.iter().filter(|p| p.r#type == "text").collect();

            let text = text_parts
                .iter()
                .filter_map(|p| p.text.as_deref())
                .collect::<String>();

            let signature = text_parts
                .iter()
                .find_map(|p| p.thought_signature.as_ref())
                .cloned();

            let text = if text.is_empty() { None } else { Some(text) };

            (text, signature)
        }
        None => (None, None),
    }
}

pub fn format_messages(messages: &[Message], image_format: &ImageFormat) -> Vec<Value> {
    format_messages_with_options(
        messages,
        image_format,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
        },
    )
}

pub fn format_messages_with_options(
    messages: &[Message],
    image_format: &ImageFormat,
    options: OpenAiFormatOptions,
) -> Vec<Value> {
    let mut messages_spec = Vec::new();
    let mut pending_assistant_reasoning = String::new();

    for message in messages {
        if options.preserve_thinking_context && message.role != Role::Assistant {
            pending_assistant_reasoning.clear();
        }

        let mut converted = json!({
            "role": message.role
        });

        let mut output = Vec::new();
        let mut content_array = Vec::new();
        let mut has_non_text_content = false;
        let mut reasoning_text = String::new();

        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    if !text.text.is_empty() {
                        if message.role == Role::User {
                            if let Some(image_path) = detect_image_path(&text.text) {
                                if let Ok(image) = load_image_file(image_path) {
                                    has_non_text_content = true;
                                    content_array.push(json!({"type": "text", "text": text.text}));
                                    content_array.push(convert_image(&image, image_format));
                                } else {
                                    content_array.push(json!({"type": "text", "text": text.text}));
                                }
                            } else {
                                content_array.push(json!({"type": "text", "text": text.text}));
                            }
                        } else {
                            content_array.push(json!({"type": "text", "text": text.text}));
                        }
                    }
                }
                MessageContent::Thinking(t) => {
                    reasoning_text.push_str(&t.thinking);
                }
                MessageContent::RedactedThinking(_) => {
                    continue;
                }
                MessageContent::SystemNotification(_) => {
                    continue;
                }
                MessageContent::ToolRequest(request) => match &request.tool_call {
                    Ok(tool_call) => {
                        let sanitized_name = sanitize_function_name(&tool_call.name);
                        let arguments_str = match &tool_call.arguments {
                            Some(args) => {
                                serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
                            }
                            None => "{}".to_string(),
                        };

                        let tool_calls = converted
                            .as_object_mut()
                            .unwrap()
                            .entry("tool_calls")
                            .or_insert(json!([]));

                        let mut tool_call_json = json!({
                            "id": request.id,
                            "type": "function",
                            "function": {
                                "name": sanitized_name,
                                "arguments": arguments_str,
                            }
                        });

                        if let Some(metadata) = &request.metadata {
                            for (key, value) in metadata {
                                tool_call_json[key] = value.clone();
                            }
                        }

                        tool_calls.as_array_mut().unwrap().push(tool_call_json);
                    }
                    Err(e) => {
                        output.push(json!({
                            "role": "tool",
                            "content": format!("Error: {}", e),
                            "tool_call_id": request.id
                        }));
                    }
                },
                MessageContent::ToolResponse(response) => {
                    match &response.tool_result {
                        Ok(result) => {
                            // Process all content, replacing images with placeholder text
                            let mut tool_content = Vec::new();
                            let mut image_messages = Vec::new();

                            for content in result.content.iter() {
                                match content.deref() {
                                    RawContent::Image(image) => {
                                        // Add placeholder text in the tool response
                                        tool_content.push(Content::text("This tool result included an image that is uploaded in the next message."));

                                        // Create a separate image message
                                        image_messages.push(json!({
                                            "role": "user",
                                            "content": [convert_image(&image.clone().no_annotation(), image_format)]
                                        }));
                                    }
                                    RawContent::Resource(resource) => {
                                        let text = extract_text_from_resource(&resource.resource);
                                        tool_content.push(Content::text(text));
                                    }
                                    _ => {
                                        tool_content.push(content.clone());
                                    }
                                }
                            }
                            let tool_response_content: Value = json!(tool_content
                                .iter()
                                .map(|content| match content.deref() {
                                    RawContent::Text(text) => text.text.clone(),
                                    _ => String::new(),
                                })
                                .collect::<Vec<String>>()
                                .join(" "));

                            // First add the tool response with all content
                            output.push(json!({
                                "role": "tool",
                                "content": tool_response_content,
                                "tool_call_id": response.id
                            }));
                            // Then add any image messages that need to follow
                            output.extend(image_messages);
                        }
                        Err(e) => {
                            // A tool result error is shown as output so the model can interpret the error message
                            output.push(json!({
                                "role": "tool",
                                "content": format!("The tool call returned the following error:\n{}", e),
                                "tool_call_id": response.id
                            }));
                        }
                    }
                }
                MessageContent::ToolConfirmationRequest(_) => {}
                MessageContent::ActionRequired(_) => {}
                MessageContent::Image(image) => {
                    if message.role == Role::User {
                        has_non_text_content = true;
                        content_array.push(convert_image(image, image_format));
                    } else {
                        content_array.push(json!({
                            "type": "text",
                            "text": "[Image content removed - not supported in assistant messages]"
                        }));
                    }
                }
                MessageContent::FrontendToolRequest(request) => match &request.tool_call {
                    Ok(tool_call) => {
                        let sanitized_name = sanitize_function_name(&tool_call.name);
                        let arguments_str = match &tool_call.arguments {
                            Some(args) => {
                                serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
                            }
                            None => "{}".to_string(),
                        };

                        let tool_calls = converted
                            .as_object_mut()
                            .unwrap()
                            .entry("tool_calls")
                            .or_insert(json!([]));

                        tool_calls.as_array_mut().unwrap().push(json!({
                            "id": request.id,
                            "type": "function",
                            "function": {
                                "name": sanitized_name,
                                "arguments": arguments_str,
                            }
                        }));
                    }
                    Err(e) => {
                        output.push(json!({
                            "role": "tool",
                            "content": format!("Error: {}", e),
                            "tool_call_id": request.id
                        }));
                    }
                },
            }
        }

        if !content_array.is_empty() {
            if has_non_text_content {
                converted["content"] = json!(content_array);
            } else {
                let texts: Vec<String> = content_array
                    .iter()
                    .filter_map(|v| v["text"].as_str().map(|s| s.to_string()))
                    .collect();
                converted["content"] = json!(texts.join("\n"));
            }
        }

        // Some strict OpenAI-compatible providers require "content" to be present
        // (even as null) when tool_calls are provided. See #6717.
        if message.role == Role::Assistant
            && converted.get("tool_calls").is_some()
            && converted.get("content").is_none()
        {
            converted["content"] = json!(null);
        }

        let has_message_payload =
            converted.get("content").is_some() || converted.get("tool_calls").is_some();

        if options.preserve_thinking_context && message.role == Role::Assistant {
            if !has_message_payload && output.is_empty() && !reasoning_text.is_empty() {
                pending_assistant_reasoning.push_str(&reasoning_text);
                continue;
            }

            if !pending_assistant_reasoning.is_empty() {
                reasoning_text =
                    merge_reasoning_text(&pending_assistant_reasoning, &reasoning_text);
                pending_assistant_reasoning.clear();
            }
        }

        // Include reasoning_content only when non-empty. Kimi rejects empty
        // reasoning_content (""), so we must omit it entirely.
        if options.preserve_thinking_context && !reasoning_text.is_empty() {
            converted["reasoning_content"] = json!(reasoning_text);
        }

        if has_message_payload {
            output.insert(0, converted);
        }

        messages_spec.extend(output);
    }

    merge_split_tool_call_messages(&mut messages_spec);
    messages_spec
}

/// The agent splits a single assistant response with N tool_calls into N
/// interleaved `asst(TC)/tool` pairs, cloning `reasoning_content` onto each.
/// This function merges them back into one assistant message with all tool_calls,
/// followed by the tool results — the standard OpenAI format.
///
/// Only merges when `reasoning_content` is present and matches, since that is
/// the only signal that messages were split from the same turn.
pub(super) fn merge_split_tool_call_messages(messages: &mut Vec<Value>) {
    let mut i = 0;
    while i < messages.len() {
        let is_assistant_tool_call = messages[i].get("role") == Some(&json!("assistant"))
            && messages[i]
                .get("tool_calls")
                .and_then(|tc| tc.as_array())
                .is_some_and(|a| !a.is_empty());
        let base_reasoning = messages[i].get("reasoning_content");

        if !is_assistant_tool_call || base_reasoning.is_none() {
            i += 1;
            continue;
        }
        let base_reasoning = base_reasoning.unwrap().clone();

        let mut extra_tool_calls: Vec<Value> = Vec::new();
        let mut collected: Vec<Value> = Vec::new();
        let mut scan = i + 1;

        loop {
            if scan >= messages.len() || messages[scan].get("role") != Some(&json!("tool")) {
                break;
            }

            // Skip past tool result and any image-only user messages that
            // format_messages inserts after tool results containing images.
            let mut peek = scan + 1;
            while peek < messages.len() && is_image_only_user_message(&messages[peek]) {
                peek += 1;
            }

            if peek >= messages.len() {
                break;
            }
            let next = &messages[peek];
            let has_no_content = next.get("content").is_none_or(|c| {
                c.is_null()
                    || c.as_str().is_some_and(|s| s.is_empty())
                    || c.as_array().is_some_and(|a| a.is_empty())
            });
            let is_split = next.get("role") == Some(&json!("assistant"))
                && next
                    .get("tool_calls")
                    .and_then(|tc| tc.as_array())
                    .is_some_and(|a| !a.is_empty())
                && has_no_content
                && next.get("reasoning_content") == Some(&base_reasoning);

            if !is_split {
                break;
            }

            collected.extend(messages[scan..peek].iter().cloned());
            if let Some(tc) = messages[peek]
                .get("tool_calls")
                .and_then(|tc| tc.as_array())
            {
                extra_tool_calls.extend(tc.iter().cloned());
            }
            scan = peek + 1;
        }

        if extra_tool_calls.is_empty() {
            i += 1;
            continue;
        }

        if let Some(base_tc) = messages[i]
            .get_mut("tool_calls")
            .and_then(|tc| tc.as_array_mut())
        {
            base_tc.extend(extra_tool_calls);
        }

        let insert_at = i + 1;
        messages.drain(insert_at..scan);
        let num_collected = collected.len();
        for (j, msg) in collected.into_iter().enumerate() {
            messages.insert(insert_at + j, msg);
        }

        i = insert_at + num_collected;
    }
}

/// True if `msg` is a synthetic image-only user message (content is exclusively image_url items).
pub(super) fn is_image_only_user_message(msg: &Value) -> bool {
    msg.get("role") == Some(&json!("user"))
        && msg
            .get("content")
            .and_then(|c| c.as_array())
            .is_some_and(|arr| {
                !arr.is_empty()
                    && arr
                        .iter()
                        .all(|item| item.get("type") == Some(&json!("image_url")))
            })
}

pub fn format_tools(tools: &[Tool]) -> anyhow::Result<Vec<Value>> {
    let mut tool_names = std::collections::HashSet::new();
    let mut result = Vec::new();

    for tool in tools {
        if !tool_names.insert(&tool.name) {
            return Err(anyhow!("Duplicate tool name: {}", tool.name));
        }

        result.push(json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema,
            }
        }));
    }

    Ok(result)
}

/// Convert OpenAI's API response to internal Message format
pub fn response_to_message(response: &Value) -> anyhow::Result<Message> {
    let Some(original) = response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|m| m.get("message"))
    else {
        if let Some(error) = response.get("error") {
            let error_message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow::anyhow!("API error: {}", error_message));
        }
        return Err(anyhow::anyhow!(
            "No message in API response. This may indicate a quota limit or other restriction."
        ));
    };

    let mut content = Vec::new();

    // Capture reasoning content if present (DeepSeek uses "reasoning_content", vLLM uses "reasoning")
    let reasoning_value = original
        .get("reasoning_content")
        .or_else(|| original.get("reasoning"));
    let mut has_structured_thinking = false;
    if let Some(reasoning_content) = reasoning_value {
        if let Some(reasoning_str) = reasoning_content.as_str() {
            if !reasoning_str.is_empty() {
                has_structured_thinking = true;
                content.push(MessageContent::thinking(reasoning_str, ""));
            }
        }
    }

    if let Some(text) = original.get("content") {
        if let Some(text_str) = text.as_str() {
            let (cleaned, inline_thinking) = split_think_blocks(text_str);

            if !has_structured_thinking && !inline_thinking.is_empty() {
                content.push(MessageContent::thinking(inline_thinking, ""));
            }

            if !cleaned.is_empty() {
                content.push(MessageContent::text(cleaned));
            }
        }
    }

    if let Some(tool_calls) = original.get("tool_calls") {
        if let Some(tool_calls_array) = tool_calls.as_array() {
            for tool_call in tool_calls_array {
                let id = tool_call["id"].as_str().unwrap_or_default().to_string();
                let function_name = tool_call["function"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                // Get the raw arguments string from the LLM.
                let arguments_str = tool_call["function"]["arguments"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                // If arguments_str is empty, default to an empty JSON object string.
                let arguments_str = if arguments_str.is_empty() {
                    "{}".to_string()
                } else {
                    arguments_str
                };

                let standard_fields = ["id", "function", "type", "index"];
                let metadata: Option<serde_json::Map<String, Value>> = tool_call
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .filter(|(k, _)| !standard_fields.contains(&k.as_str()))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .filter(|m: &serde_json::Map<String, Value>| !m.is_empty());

                if !is_valid_function_name(&function_name) {
                    let error = ErrorData {
                        code: ErrorCode::INVALID_REQUEST,
                        message: Cow::from(format!(
                            "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                            function_name
                        )),
                        data: None,
                    };
                    content.push(MessageContent::tool_request_with_metadata(
                        id,
                        Err(error),
                        metadata.as_ref(),
                    ));
                } else {
                    match safely_parse_json(&arguments_str) {
                        Ok(params) => {
                            content.push(MessageContent::tool_request_with_metadata(
                                id,
                                Ok(CallToolRequestParams::new(function_name)
                                    .with_arguments(object(params))),
                                metadata.as_ref(),
                            ));
                        }
                        Err(e) => {
                            let error = ErrorData {
                                code: ErrorCode::INVALID_PARAMS,
                                message: Cow::from(format!(
                                    "Could not interpret tool use parameters for id {}: {}. Raw arguments: '{}'",
                                    id, e, arguments_str
                                )),
                                data: None,
                            };
                            content.push(MessageContent::tool_request_with_metadata(
                                id,
                                Err(error),
                                metadata.as_ref(),
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(Message::new(
        Role::Assistant,
        chrono::Utc::now().timestamp(),
        content,
    ))
}

pub fn get_usage(usage: &Value) -> Usage {
    let usage = usage
        .get("usage")
        .filter(|nested| nested.is_object())
        .unwrap_or(usage);

    // Try standard OpenAI fields first, then fall back to Ollama-native fields
    // (prompt_eval_count / eval_count) for compatibility with older Ollama builds
    // that don't translate to OpenAI field names.
    // Parse the value before falling back so that present-but-null keys
    // (e.g. "completion_tokens": null) don't block the fallback.
    let input_tokens = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| usage.get("prompt_eval_count").and_then(|v| v.as_i64()))
        .map(|v| v as i32);

    let output_tokens = usage
        .get("completion_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| usage.get("eval_count").and_then(|v| v.as_i64()))
        .map(|v| v as i32);

    let cache_read_input_tokens = usage
        .get("cache_read_input_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let cache_write_input_tokens = usage
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let total_tokens = usage
        .get("total_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .or_else(|| match (input_tokens, output_tokens) {
            (Some(input), Some(output)) => Some(input.saturating_add(output)),
            _ => None,
        });

    Usage::new(input_tokens, output_tokens, total_tokens)
        .with_cache_tokens(cache_read_input_tokens, cache_write_input_tokens)
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
                        normalize_nullable(prop);
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

/// Normalizes nullable type representations that some providers (e.g. Vertex Gemini via Bifrost)
/// don't support:
/// - `"type": ["integer", "null"]` → `"type": "integer"` (drops the null variant)
/// - `"anyOf": [T, {"type": "null"}]` → T (unwraps to the non-null schema)
///
/// Optional-ness is already conveyed by the field being absent from `required`.
fn normalize_nullable(schema: &mut Value) {
    let Some(obj) = schema.as_object_mut() else {
        return;
    };

    // Handle type: ["T", "null"] array form (schemars 1.x style for nullable primitives)
    if let Some(type_val) = obj.get("type").cloned() {
        if let Some(types) = type_val.as_array() {
            let non_null: Vec<&Value> = types
                .iter()
                .filter(|t| t.as_str() != Some("null"))
                .collect();
            if non_null.len() == 1 {
                let scalar = non_null[0].clone();
                obj.insert("type".to_string(), scalar);
                return;
            }
        }
    }

    // Handle anyOf: [T, {type: "null"}] form — merge the non-null variant's fields
    // into the current object (preserving sibling keys like "description" or "default")
    // rather than replacing the whole schema.
    if let Some(any_of) = obj.remove("anyOf") {
        if let Some(variants) = any_of.as_array() {
            if variants.len() == 2 {
                let is_null = |v: &Value| v.get("type").and_then(|t| t.as_str()) == Some("null");
                let non_null = if is_null(&variants[0]) {
                    Some(&variants[1])
                } else if is_null(&variants[1]) {
                    Some(&variants[0])
                } else {
                    None
                };
                if let Some(replacement) = non_null {
                    if let Some(replacement_obj) = replacement.as_object() {
                        for (k, v) in replacement_obj {
                            obj.entry(k.clone()).or_insert(v.clone());
                        }
                        return;
                    }
                }
            }
        }
        // Put it back if we couldn't simplify
        obj.insert("anyOf".to_string(), any_of);
    }
}
