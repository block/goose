use crate::conversation::message::{Message, MessageContent, ProviderMetadata};
use crate::mcp_utils::extract_text_from_resource;
use crate::model::ModelConfig;
use crate::providers::base::{ProviderUsage, Usage};
use crate::providers::utils::{
    convert_image, detect_image_path, is_valid_function_name, load_image_file, safely_parse_json,
    sanitize_function_name, ImageFormat,
};
use anyhow::{anyhow, Error};
use async_stream::try_stream;
use chrono;
use futures::Stream;
use rmcp::model::{
    object, AnnotateAble, CallToolRequestParams, Content, ErrorCode, ErrorData, RawContent, Role,
    Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;

type ToolCallData = HashMap<
    i32,
    (
        String,
        String,
        String,
        Option<serde_json::Map<String, Value>>,
    ),
>;

#[derive(Serialize, Deserialize, Debug, Default)]
struct DeltaToolCallFunction {
    name: Option<String>,
    #[serde(default)]
    arguments: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeltaToolCall {
    id: Option<String>,
    function: DeltaToolCallFunction,
    index: Option<i32>,
    r#type: Option<String>,
    #[serde(flatten)]
    extra: Option<serde_json::Map<String, Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Delta {
    content: Option<String>,
    role: Option<String>,
    tool_calls: Option<Vec<DeltaToolCall>>,
    reasoning_details: Option<Vec<Value>>,
    reasoning_content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct StreamingChoice {
    delta: Delta,
    index: Option<i32>,
    finish_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct StreamingChunk {
    choices: Vec<StreamingChoice>,
    created: Option<i64>,
    id: Option<String>,
    usage: Option<Value>,
    model: Option<String>,
}

pub fn format_messages(messages: &[Message], image_format: &ImageFormat) -> Vec<Value> {
    let mut messages_spec = Vec::new();
    for message in messages {
        let mut converted = json!({
            "role": message.role
        });

        let mut output = Vec::new();
        let mut content_array = Vec::new();
        let mut text_array = Vec::new();
        let mut reasoning_text = String::new();

        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    if !text.text.is_empty() {
                        if message.role == Role::User {
                            if let Some(image_path) = detect_image_path(&text.text) {
                                if let Ok(image) = load_image_file(image_path) {
                                    content_array.push(json!({"type": "text", "text": text.text}));
                                    content_array.push(convert_image(&image, image_format));
                                } else {
                                    text_array.push(text.text.clone());
                                }
                            } else {
                                text_array.push(text.text.clone());
                            }
                        } else {
                            text_array.push(text.text.clone());
                        }
                    }
                }
                MessageContent::Thinking(_) => {
                    // Thinking blocks are not directly used in OpenAI format
                    continue;
                }
                MessageContent::RedactedThinking(_) => {
                    // Redacted thinking blocks are not directly used in OpenAI format
                    continue;
                }
                MessageContent::SystemNotification(_) => {
                    continue;
                }
                MessageContent::Reasoning(r) => {
                    reasoning_text.push_str(&r.text);
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
            converted["content"] = json!(content_array);
        } else if !text_array.is_empty() {
            converted["content"] = json!(text_array.join("\n"));
        }

        // Some strict OpenAI-compatible providers require "content" to be present
        // (even as null) when tool_calls are provided. See #6717.
        if message.role == Role::Assistant
            && converted.get("tool_calls").is_some()
            && converted.get("content").is_none()
        {
            converted["content"] = json!(null);
        }

        // Include reasoning_content only when non-empty.
        // Kimi rejects empty reasoning_content (""), so we must omit it entirely
        // when there's no reasoning to send.
        if !reasoning_text.is_empty() {
            converted["reasoning_content"] = json!(reasoning_text);
        }

        if converted.get("content").is_some() || converted.get("tool_calls").is_some() {
            output.insert(0, converted);
        }

        messages_spec.extend(output);
    }

    messages_spec
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

    // Capture reasoning_content if present (for DeepSeek reasoning models)
    if let Some(reasoning_content) = original.get("reasoning_content") {
        if let Some(reasoning_str) = reasoning_content.as_str() {
            if !reasoning_str.is_empty() {
                content.push(MessageContent::reasoning(reasoning_str));
            }
        }
    }

    if let Some(text) = original.get("content") {
        if let Some(text_str) = text.as_str() {
            content.push(MessageContent::text(text_str));
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
                                Ok(CallToolRequestParams {
                                    meta: None,
                                    task: None,
                                    name: function_name.into(),
                                    arguments: Some(object(params)),
                                }),
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
    let input_tokens = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let output_tokens = usage
        .get("completion_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let total_tokens = usage
        .get("total_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .or_else(|| match (input_tokens, output_tokens) {
            (Some(input), Some(output)) => Some(input + output),
            _ => None,
        });

    Usage::new(input_tokens, output_tokens, total_tokens)
}

fn extract_usage_with_output_tokens(chunk: &StreamingChunk) -> Option<ProviderUsage> {
    chunk
        .usage
        .as_ref()
        .and_then(|u| {
            chunk.model.as_ref().map(|model| ProviderUsage {
                usage: get_usage(u),
                model: model.clone(),
            })
        })
        .filter(|u| u.usage.output_tokens.is_some())
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

fn strip_data_prefix(line: &str) -> Option<&str> {
    line.strip_prefix("data: ").map(|s| s.trim())
}

pub fn response_to_streaming_message<S>(
    mut stream: S,
) -> impl Stream<Item = anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> + 'static
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    try_stream! {
        use futures::StreamExt;

        let mut accumulated_reasoning: Vec<Value> = Vec::new();
        let mut accumulated_reasoning_content = String::new();

        'outer: while let Some(response) = stream.next().await {
            if response.as_ref().is_ok_and(|s| s == "data: [DONE]") {
                break 'outer;
            }
            let response_str = response?;
            let line = strip_data_prefix(&response_str);

            if line.is_none() || line.is_some_and(|l| l.is_empty()) {
                continue
            }

            let chunk: StreamingChunk = serde_json::from_str(line
                .ok_or_else(|| anyhow!("unexpected stream format"))?)
                .map_err(|e| anyhow!("Failed to parse streaming chunk: {}: {:?}", e, &line))?;

            if !chunk.choices.is_empty() {
                if let Some(details) = &chunk.choices[0].delta.reasoning_details {
                    accumulated_reasoning.extend(details.iter().cloned());
                }
                if let Some(rc) = &chunk.choices[0].delta.reasoning_content {
                    accumulated_reasoning_content.push_str(rc);
                }
            }

            let mut usage = extract_usage_with_output_tokens(&chunk);

            if chunk.choices.is_empty() {
                yield (None, usage)
            } else if chunk.choices[0].delta.tool_calls.as_ref().is_some_and(|tc| !tc.is_empty()) {
                let mut tool_call_data: ToolCallData = HashMap::new();

                if let Some(tool_calls) = &chunk.choices[0].delta.tool_calls {
                    for tool_call in tool_calls {
                        if let (Some(index), Some(id), Some(name)) = (tool_call.index, &tool_call.id, &tool_call.function.name) {
                            tool_call_data.insert(index, (id.clone(), name.clone(), tool_call.function.arguments.clone(), tool_call.extra.clone()));
                        }
                    }
                }

                let is_complete = chunk.choices[0].finish_reason == Some("tool_calls".to_string());

                if !is_complete {
                    let mut done = false;
                    while !done {
                        if let Some(response_chunk) = stream.next().await {
                            if response_chunk.as_ref().is_ok_and(|s| s == "data: [DONE]") {
                                break 'outer;
                            }
                            let response_str = response_chunk?;
                            if let Some(line) = strip_data_prefix(&response_str) {

                                let tool_chunk: StreamingChunk = serde_json::from_str(line)
                                    .map_err(|e| anyhow!("Failed to parse streaming chunk: {}: {:?}", e, &line))?;

                                if let Some(chunk_usage) = extract_usage_with_output_tokens(&tool_chunk) {
                                    usage = Some(chunk_usage);
                                }

                                if !tool_chunk.choices.is_empty() {
                                    if let Some(details) = &tool_chunk.choices[0].delta.reasoning_details {
                                        accumulated_reasoning.extend(details.iter().cloned());
                                    }
                                    if let Some(rc) = &tool_chunk.choices[0].delta.reasoning_content {
                                        accumulated_reasoning_content.push_str(rc);
                                    }
                                    if let Some(delta_tool_calls) = &tool_chunk.choices[0].delta.tool_calls {
                                        for delta_call in delta_tool_calls {
                                            if let Some(index) = delta_call.index {
                                                if let Some((_, _, ref mut args, ref mut extra)) = tool_call_data.get_mut(&index) {
                                                    args.push_str(&delta_call.function.arguments);
                                                    if extra.is_none() && delta_call.extra.is_some() {
                                                        *extra = delta_call.extra.clone();
                                                    } else if let (Some(existing), Some(new_extra)) = (extra.as_mut(), &delta_call.extra) {
                                                        for (key, value) in new_extra {
                                                            existing.entry(key.clone()).or_insert(value.clone());
                                                        }
                                                    }
                                                } else if let (Some(id), Some(name)) = (&delta_call.id, &delta_call.function.name) {
                                                    tool_call_data.insert(index, (id.clone(), name.clone(), delta_call.function.arguments.clone(), delta_call.extra.clone()));
                                                }
                                            }
                                        }
                                    }
                                    if tool_chunk.choices[0].finish_reason.is_some() {
                                        done = true;
                                    }
                                } else {
                                    done = true;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }

                let _metadata: Option<ProviderMetadata> = if !accumulated_reasoning.is_empty() {
                    let mut map = ProviderMetadata::new();
                    map.insert("reasoning_details".to_string(), json!(accumulated_reasoning));
                    Some(map)
                } else {
                    None
                };

                let mut contents = Vec::new();
                if !accumulated_reasoning_content.is_empty() {
                    contents.push(MessageContent::reasoning(&accumulated_reasoning_content));
                    accumulated_reasoning_content.clear();
                }
                let mut sorted_indices: Vec<_> = tool_call_data.keys().cloned().collect();
                sorted_indices.sort();

                for index in sorted_indices {
                    if let Some((id, function_name, arguments, extra_fields)) = tool_call_data.get(&index) {
                        let parsed = if arguments.is_empty() {
                            Ok(json!({}))
                        } else {
                            serde_json::from_str::<Value>(arguments)
                        };

                        let metadata = extra_fields.as_ref().filter(|m| !m.is_empty());

                        let content = match parsed {
                            Ok(params) => {
                                MessageContent::tool_request_with_metadata(
                                    id.clone(),
                                    Ok(CallToolRequestParams {
                                        meta: None,
                                        task: None,
                                        name: function_name.clone().into(),
                                        arguments: Some(object(params)),
                                    }),
                                    metadata,
                                )
                            },
                            Err(e) => {
                                let error = ErrorData {
                                    code: ErrorCode::INVALID_PARAMS,
                                    message: Cow::from(format!(
                                        "Could not interpret tool use parameters for id {}: {}",
                                        id, e
                                    )),
                                    data: None,
                                };
                                MessageContent::tool_request_with_metadata(id.clone(), Err(error), metadata)
                            }
                        };
                        contents.push(content);
                    }
                }

                let mut msg = Message::new(
                    Role::Assistant,
                    chrono::Utc::now().timestamp(),
                    contents,
                );

                // Add ID if present
                if let Some(id) = chunk.id {
                    msg = msg.with_id(id);
                }

                yield (
                    Some(msg),
                    usage,
                )
            } else if chunk.choices[0].delta.content.is_some() || chunk.choices[0].delta.reasoning_content.is_some() {
                let mut content = Vec::new();

                if let Some(reasoning) = &chunk.choices[0].delta.reasoning_content {
                    if !reasoning.is_empty() {
                        content.push(MessageContent::reasoning(reasoning));
                    }
                }

                if let Some(text) = &chunk.choices[0].delta.content {
                    if !text.is_empty() {
                        content.push(MessageContent::text(text));
                    }
                }

                if !content.is_empty() {
                    let mut msg = Message::new(
                        Role::Assistant,
                        chrono::Utc::now().timestamp(),
                        content,
                    );

                    // Add ID if present
                    if let Some(id) = chunk.id {
                        msg = msg.with_id(id);
                    }

                    yield (
                        Some(msg),
                        if chunk.choices[0].finish_reason.is_some() {
                            usage
                        } else {
                            None
                        },
                    )
                } else if usage.is_some() {
                    yield (None, usage)
                }
            } else if usage.is_some() {
                yield (None, usage)
            }
        }
    }
}

pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
    for_streaming: bool,
) -> anyhow::Result<Value, Error> {
    if model_config.model_name.starts_with("o1-mini") {
        return Err(anyhow!(
            "o1-mini model is not currently supported since goose uses tool calling and o1-mini does not support it. Please use o1 or o3 models instead."
        ));
    }

    let is_ox_model = model_config.model_name.starts_with("o1")
        || model_config.model_name.starts_with("o2")
        || model_config.model_name.starts_with("o3")
        || model_config.model_name.starts_with("o4")
        || model_config.model_name.starts_with("gpt-5");

    // Only extract reasoning effort for O-series models
    let (model_name, reasoning_effort) = if is_ox_model {
        let parts: Vec<&str> = model_config.model_name.split('-').collect();
        let last_part = parts.last().unwrap();

        match *last_part {
            "low" | "medium" | "high" => {
                let base_name = parts[..parts.len() - 1].join("-");
                (base_name, Some(last_part.to_string()))
            }
            _ => (
                model_config.model_name.to_string(),
                Some("medium".to_string()),
            ),
        }
    } else {
        // For non-O family models, use the model name as is and no reasoning effort
        (model_config.model_name.to_string(), None)
    };

    let system_message = json!({
        "role": if is_ox_model { "developer" } else { "system" },
        "content": system
    });

    let messages_spec = format_messages(messages, image_format);
    let mut tools_spec = format_tools(tools)?;

    validate_tool_schemas(&mut tools_spec);

    let mut messages_array = vec![system_message];
    messages_array.extend(messages_spec);

    let mut payload = json!({
        "model": model_name,
        "messages": messages_array
    });

    if let Some(effort) = reasoning_effort {
        payload["reasoning_effort"] = json!(effort);
    }

    if !tools_spec.is_empty() {
        payload["tools"] = json!(tools_spec);
    }

    // o1, o3 models currently don't support temperature
    if !is_ox_model {
        if let Some(temp) = model_config.temperature {
            payload["temperature"] = json!(temp);
        }
    }

    // o1 models use max_completion_tokens instead of max_tokens
    if let Some(tokens) = model_config.max_tokens {
        let key = if is_ox_model {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        payload
            .as_object_mut()
            .unwrap()
            .insert(key.to_string(), json!(tokens));
    }

    if for_streaming {
        payload["stream"] = json!(true);
        payload["stream_options"] = json!({"include_usage": true});
    }

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::model::CallToolResult;
    use rmcp::object;
    use serde_json::json;
    use tokio::pin;
    use tokio_stream::{self, StreamExt};

    #[test]
    fn test_validate_tool_schemas() {
        // Test case 1: Empty parameters object
        // Input JSON with an incomplete parameters object
        let mut actual = vec![json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "type": "object"
                }
            }
        })];

        // Run the function to validate and update schemas
        validate_tool_schemas(&mut actual);

        // Expected JSON after validation
        let expected = vec![json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        })];

        // Compare entire JSON structures instead of individual fields
        assert_eq!(actual, expected);

        // Test case 2: Missing type field
        let mut tools = vec![json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "properties": {}
                }
            }
        })];

        validate_tool_schemas(&mut tools);

        let params = tools[0]["function"]["parameters"].as_object().unwrap();
        assert_eq!(params["type"], "object");

        // Test case 3: Complete valid schema should remain unchanged
        let original_schema = json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "City and country"
                        }
                    },
                    "required": ["location"]
                }
            }
        });

        let mut tools = vec![original_schema.clone()];
        validate_tool_schemas(&mut tools);
        assert_eq!(tools[0], original_schema);
    }

    const OPENAI_TOOL_USE_RESPONSE: &str = r#"{
        "choices": [{
            "role": "assistant",
            "message": {
                "tool_calls": [{
                    "id": "1",
                    "function": {
                        "name": "example_fn",
                        "arguments": "{\"param\": \"value\"}"
                    }
                }]
            }
        }],
        "usage": {
            "input_tokens": 10,
            "output_tokens": 25,
            "total_tokens": 35
        }
    }"#;

    #[test]
    fn test_format_messages() -> anyhow::Result<()> {
        let message = Message::user().with_text("Hello");
        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(spec[0]["content"], "Hello");
        Ok(())
    }

    #[test]
    fn test_format_tools() -> anyhow::Result<()> {
        let tool = Tool::new(
            "test_tool",
            "A test tool",
            object!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let spec = format_tools(&[tool])?;

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["type"], "function");
        assert_eq!(spec[0]["function"]["name"], "test_tool");
        Ok(())
    }

    #[test]
    fn test_format_messages_complex() -> anyhow::Result<()> {
        let mut messages = vec![
            Message::assistant().with_text("Hello!"),
            Message::user().with_text("How are you?"),
            Message::assistant().with_tool_request(
                "tool1",
                Ok(CallToolRequestParams {
                    meta: None,
                    task: None,
                    name: "example".into(),
                    arguments: Some(object!({"param1": "value1"})),
                }),
            ),
        ];

        // Get the ID from the tool request to use in the response
        let tool_id = if let MessageContent::ToolRequest(request) = &messages[2].content[0] {
            request.id.clone()
        } else {
            panic!("should be tool request");
        };

        messages.push(Message::user().with_tool_response(
            tool_id,
            Ok(CallToolResult {
                content: vec![Content::text("Result")],
                structured_content: None,
                is_error: Some(false),
                meta: None,
            }),
        ));

        let spec = format_messages(&messages, &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 4);
        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"], "Hello!");
        assert_eq!(spec[1]["role"], "user");
        assert_eq!(spec[1]["content"], "How are you?");
        assert_eq!(spec[2]["role"], "assistant");
        assert!(spec[2]["tool_calls"].is_array());
        assert_eq!(spec[3]["role"], "tool");
        assert_eq!(spec[3]["content"], "Result");
        assert_eq!(spec[3]["tool_call_id"], spec[2]["tool_calls"][0]["id"]);

        Ok(())
    }

    #[test]
    fn test_format_messages_multiple_content() -> anyhow::Result<()> {
        let mut messages = vec![Message::assistant().with_tool_request(
            "tool1",
            Ok(CallToolRequestParams {
                meta: None,
                task: None,
                name: "example".into(),
                arguments: Some(object!({"param1": "value1"})),
            }),
        )];

        // Get the ID from the tool request to use in the response
        let tool_id = if let MessageContent::ToolRequest(request) = &messages[0].content[0] {
            request.id.clone()
        } else {
            panic!("should be tool request");
        };

        messages.push(Message::user().with_tool_response(
            tool_id,
            Ok(CallToolResult {
                content: vec![Content::text("Result")],
                structured_content: None,
                is_error: Some(false),
                meta: None,
            }),
        ));

        let spec = format_messages(&messages, &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());
        assert_eq!(spec[1]["role"], "tool");
        assert_eq!(spec[1]["content"], "Result");
        assert_eq!(spec[1]["tool_call_id"], spec[0]["tool_calls"][0]["id"]);

        Ok(())
    }

    #[test]
    fn test_format_tools_duplicate() -> anyhow::Result<()> {
        let tool1 = Tool::new(
            "test_tool",
            "Test tool",
            object!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let tool2 = Tool::new(
            "test_tool",
            "Test tool",
            object!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let result = format_tools(&[tool1, tool2]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate tool name"));

        Ok(())
    }

    #[test]
    fn test_format_tools_empty() -> anyhow::Result<()> {
        let spec = format_tools(&[])?;
        assert!(spec.is_empty());
        Ok(())
    }

    #[test]
    fn test_format_messages_with_image_path() -> anyhow::Result<()> {
        // Create a temporary PNG file with valid PNG magic numbers
        let temp_dir = tempfile::tempdir()?;
        let png_path = temp_dir.path().join("test.png");
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, // PNG magic number
            0x0D, 0x0A, 0x1A, 0x0A, // PNG header
            0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
        ];
        std::fs::write(&png_path, png_data)?;
        let png_path_str = png_path.to_str().unwrap();

        // Create user message with image path - should load the image
        let user_message = Message::user().with_text(format!("Here is an image: {}", png_path_str));
        let spec = format_messages(&[user_message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");

        // Content should be an array with text and image
        let content = spec[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert!(content[0]["text"].as_str().unwrap().contains(png_path_str));
        assert_eq!(content[1]["type"], "image_url");
        assert!(content[1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));

        // Create assistant message with same text - should NOT load the image
        let assistant_message =
            Message::assistant().with_text(format!("I saved the output to {}", png_path_str));
        let spec = format_messages(&[assistant_message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");

        // Content should be plain text, NOT an array with image
        let content = spec[0]["content"].as_str();
        assert!(
            content.is_some(),
            "Assistant message content should be a string, not an array with image"
        );
        assert!(content.unwrap().contains(png_path_str));

        Ok(())
    }

    #[test]
    fn test_response_to_message_text() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "content": "Hello from John Cena!"
                }
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 25,
                "total_tokens": 35
            }
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 1);
        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello from John Cena!");
        } else {
            panic!("Expected Text content");
        }
        assert!(matches!(message.role, Role::Assistant));

        Ok(())
    }

    #[test]
    fn test_response_to_message_valid_toolrequest() -> anyhow::Result<()> {
        let response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        let message = response_to_message(&response)?;

        assert_eq!(message.content.len(), 1);
        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "example_fn");
            assert_eq!(tool_call.arguments, Some(object!({"param": "value"})));
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_invalid_func_name() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["name"] =
            json!("invalid fn");

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ErrorData {
                    code: ErrorCode::INVALID_REQUEST,
                    message: msg,
                    data: None,
                }) => {
                    assert!(msg.starts_with("The provided function name"));
                }
                _ => panic!("Expected ToolNotFound error"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_json_decode_error() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            json!("invalid json {");

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ErrorData {
                    code: ErrorCode::INVALID_PARAMS,
                    message: msg,
                    data: None,
                }) => {
                    assert!(msg.starts_with("Could not interpret tool use parameters"));
                }
                _ => panic!("Expected InvalidParameters error"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_empty_argument() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            serde_json::Value::String("".to_string());

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "example_fn");
            assert_eq!(tool_call.arguments, Some(object!({})));
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_api_error() -> anyhow::Result<()> {
        // Test that API responses with an "error" field return the error message
        let response = json!({
            "error": {
                "message": "You have exceeded your quota",
                "type": "insufficient_quota",
                "code": "quota_exceeded"
            }
        });

        let result = response_to_message(&response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("API error:"));
        assert!(err.to_string().contains("You have exceeded your quota"));

        Ok(())
    }

    #[test]
    fn test_response_to_message_api_error_unknown() -> anyhow::Result<()> {
        // Test that API responses with an "error" field but no message return "Unknown error"
        let response = json!({
            "error": {
                "type": "some_error"
            }
        });

        let result = response_to_message(&response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("API error:"));
        assert!(err.to_string().contains("Unknown error"));

        Ok(())
    }

    #[test]
    fn test_response_to_message_no_choices() -> anyhow::Result<()> {
        // Test that responses without "choices" return an error
        let response = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1234567890
        });

        let result = response_to_message(&response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No message in API response"));

        Ok(())
    }

    #[test]
    fn test_format_messages_tool_request_with_none_arguments() -> anyhow::Result<()> {
        // Test that tool calls with None arguments are formatted as "{}" string
        let message = Message::assistant().with_tool_request(
            "tool1",
            Ok(CallToolRequestParams {
                meta: None,
                task: None,
                name: "test_tool".into(),
                arguments: None, // This is the key case the fix addresses
            }),
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());

        let tool_call = &spec[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "tool1");
        assert_eq!(tool_call["type"], "function");
        assert_eq!(tool_call["function"]["name"], "test_tool");
        // This should be the string "{}", not null
        assert_eq!(tool_call["function"]["arguments"], "{}");

        Ok(())
    }

    #[test]
    fn test_format_messages_tool_request_with_some_arguments() -> anyhow::Result<()> {
        // Test that tool calls with Some arguments are properly JSON-serialized
        let message = Message::assistant().with_tool_request(
            "tool1",
            Ok(CallToolRequestParams {
                meta: None,
                task: None,
                name: "test_tool".into(),
                arguments: Some(object!({"param": "value", "number": 42})),
            }),
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());

        let tool_call = &spec[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "tool1");
        assert_eq!(tool_call["type"], "function");
        assert_eq!(tool_call["function"]["name"], "test_tool");
        // This should be a JSON string representation
        let args_str = tool_call["function"]["arguments"].as_str().unwrap();
        let parsed_args: Value = serde_json::from_str(args_str)?;
        assert_eq!(parsed_args["param"], "value");
        assert_eq!(parsed_args["number"], 42);

        Ok(())
    }

    #[test]
    fn test_format_messages_frontend_tool_request_with_none_arguments() -> anyhow::Result<()> {
        // Test that FrontendToolRequest with None arguments are formatted as "{}" string
        let message = Message::assistant().with_frontend_tool_request(
            "frontend_tool1",
            Ok(CallToolRequestParams {
                meta: None,
                task: None,
                name: "frontend_test_tool".into(),
                arguments: None, // This is the key case the fix addresses
            }),
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());

        let tool_call = &spec[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "frontend_tool1");
        assert_eq!(tool_call["type"], "function");
        assert_eq!(tool_call["function"]["name"], "frontend_test_tool");
        // This should be the string "{}", not null
        assert_eq!(tool_call["function"]["arguments"], "{}");

        Ok(())
    }

    #[test]
    fn test_format_messages_frontend_tool_request_with_some_arguments() -> anyhow::Result<()> {
        // Test that FrontendToolRequest with Some arguments are properly JSON-serialized
        let message = Message::assistant().with_frontend_tool_request(
            "frontend_tool1",
            Ok(CallToolRequestParams {
                meta: None,
                task: None,
                name: "frontend_test_tool".into(),
                arguments: Some(object!({"action": "click", "element": "button"})),
            }),
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());

        let tool_call = &spec[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "frontend_tool1");
        assert_eq!(tool_call["type"], "function");
        assert_eq!(tool_call["function"]["name"], "frontend_test_tool");
        // This should be a JSON string representation
        let args_str = tool_call["function"]["arguments"].as_str().unwrap();
        let parsed_args: Value = serde_json::from_str(args_str)?;
        assert_eq!(parsed_args["action"], "click");
        assert_eq!(parsed_args["element"], "button");

        Ok(())
    }

    #[test]
    fn test_format_messages_multiple_text_blocks() -> anyhow::Result<()> {
        let message = Message::user()
            .with_text("--- Resource: file:///test.md ---\n# Test\n\n---\n")
            .with_text(" What is in the file?");

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(
            spec[0]["content"],
            "--- Resource: file:///test.md ---\n# Test\n\n---\n\n What is in the file?"
        );
        Ok(())
    }

    #[test]
    fn test_create_request_gpt_4o() -> anyhow::Result<()> {
        // Test default medium reasoning effort for O3 model
        let model_config = ModelConfig {
            model_name: "gpt-4o".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "gpt-4o",
            "messages": [
                {
                    "role": "system",
                    "content": "system"
                }
            ],
            "max_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }

        Ok(())
    }

    #[test]
    fn test_create_request_o1_default() -> anyhow::Result<()> {
        // Test default medium reasoning effort for O1 model
        let model_config = ModelConfig {
            model_name: "o1".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "o1",
            "messages": [
                {
                    "role": "developer",
                    "content": "system"
                }
            ],
            "reasoning_effort": "medium",
            "max_completion_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }

        Ok(())
    }

    #[test]
    fn test_create_request_o3_custom_reasoning_effort() -> anyhow::Result<()> {
        // Test custom reasoning effort for O3 model
        let model_config = ModelConfig {
            model_name: "o3-mini-high".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "o3-mini",
            "messages": [
                {
                    "role": "developer",
                    "content": "system"
                }
            ],
            "reasoning_effort": "high",
            "max_completion_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }

        Ok(())
    }

    struct StreamingUsageTestResult {
        usage_count: usize,
        usage: Option<ProviderUsage>,
        tool_calls: Vec<String>,
        has_text_content: bool,
    }

    async fn run_streaming_test(response_lines: &str) -> anyhow::Result<StreamingUsageTestResult> {
        let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = response_to_streaming_message(response_stream);
        pin!(messages);

        let mut result = StreamingUsageTestResult {
            usage_count: 0,
            usage: None,
            tool_calls: Vec::new(),
            has_text_content: false,
        };

        while let Some(Ok((message, usage))) = messages.next().await {
            if let Some(u) = usage {
                result.usage_count += 1;
                result.usage = Some(u);
            }
            if let Some(msg) = message {
                for content in &msg.content {
                    match content {
                        MessageContent::ToolRequest(req) => {
                            if let Ok(tool_call) = &req.tool_call {
                                result.tool_calls.push(tool_call.name.to_string());
                            }
                        }
                        MessageContent::Text(text) if !text.text.is_empty() => {
                            result.has_text_content = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(result)
    }

    fn assert_usage_yielded_once(
        result: &StreamingUsageTestResult,
        expected_input: i32,
        expected_output: i32,
        expected_total: i32,
    ) {
        assert_eq!(
            result.usage_count, 1,
            "Usage should be yielded exactly once, but was yielded {} times",
            result.usage_count
        );

        let usage = result.usage.as_ref().expect("Expected usage to be present");
        assert_eq!(usage.usage.input_tokens, Some(expected_input));
        assert_eq!(usage.usage.output_tokens, Some(expected_output));
        assert_eq!(usage.usage.total_tokens, Some(expected_total));
    }

    #[tokio::test]
    async fn test_streamed_multi_tool_response_to_messages() -> anyhow::Result<()> {
        let response_lines = r#"
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":"I'll run both"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":" `ls` commands in a"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":" single turn for you -"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":" one on the current directory an"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":"d one on the `working_dir`."},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"id":"toolu_bdrk_01RMTd7R9DzQjEEWgDwzcBsU","type":"function","function":{"name":"developer__shell","arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":"{\""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":"command\": \"l"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":"s\"}"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"id":"toolu_bdrk_016bgVTGZdpjP8ehjMWp9cWW","type":"function","function":{"name":"developer__shell","arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"{\""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"command\""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":": \"ls wor"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"king_dir"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"\"}"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":""},"index":0,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":4982,"completion_tokens":122,"total_tokens":5104},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: [DONE]
"#;

        let result = run_streaming_test(response_lines).await?;
        assert_eq!(
            result.tool_calls.len(),
            2,
            "Expected 2 tool calls, got {}",
            result.tool_calls.len()
        );
        assert!(result
            .tool_calls
            .iter()
            .all(|name| name == "developer__shell"));

        assert_usage_yielded_once(&result, 4982, 122, 5104);

        Ok(())
    }

    #[tokio::test]
    async fn test_openrouter_streaming_usage_yielded_once() -> anyhow::Result<()> {
        let response_lines = r#"
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":"","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":"There","reasoning":"","reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":" are","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":" **47**","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":" files.","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":"","reasoning":null,"reasoning_details":[]},"finish_reason":"stop","native_finish_reason":"stop","logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null,"native_finish_reason":null,"logprobs":null}],"usage":{"prompt_tokens":7007,"completion_tokens":49,"total_tokens":7056}}
data: [DONE]
"#;

        let result = run_streaming_test(response_lines).await?;

        assert!(result.has_text_content, "Expected text content in response");
        assert_usage_yielded_once(&result, 7007, 49, 7056);

        Ok(())
    }

    #[tokio::test]
    async fn test_openai_gpt5_streaming_usage_yielded_once() -> anyhow::Result<()> {
        let response_lines = r#"
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[{"index":0,"delta":{"role":"assistant","content":null,"tool_calls":[{"index":0,"id":"call_x4CIvBVfQhYMhyO0T1VEddua","type":"function","function":{"name":"developer__shell","arguments":""}}],"refusal":null},"logprobs":null,"finish_reason":null}],"usage":null}
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"command\":\"ls ~/Desktop | wc -l\"}"}}]},"logprobs":null,"finish_reason":null}],"usage":null}
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"tool_calls"}],"usage":null}
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[],"usage":{"prompt_tokens":8320,"completion_tokens":172,"total_tokens":8492}}
data: [DONE]
"#;

        let result = run_streaming_test(response_lines).await?;

        assert_eq!(result.tool_calls.len(), 1, "Expected 1 tool call");
        assert_eq!(result.tool_calls[0], "developer__shell");
        assert_usage_yielded_once(&result, 8320, 172, 8492);

        Ok(())
    }

    #[tokio::test]
    async fn test_tetrate_claude_streaming_usage_yielded_once() -> anyhow::Result<()> {
        let response_lines = r#"
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{"role":"assistant"}}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk"}
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"toolu_011Yj5pGczhs1597iLXp5XJK","type":"function","function":{"name":"developer__shell","arguments":""}}]}}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk"}
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"type":"function","function":{"arguments":"{\"command\": \"find ~/Desktop -type f | wc -l\"}"}}]}}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk"}
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk","usage":{"completion_tokens":79,"prompt_tokens":12376,"total_tokens":12455}}
data: [DONE]
"#;

        let result = run_streaming_test(response_lines).await?;

        assert_eq!(result.tool_calls.len(), 1, "Expected 1 tool call");
        assert_eq!(result.tool_calls[0], "developer__shell");
        assert_usage_yielded_once(&result, 12376, 79, 12455);

        Ok(())
    }

    #[test]
    fn test_response_to_message_with_nested_extra_content() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_456",
                        "type": "function",
                        "function": {
                            "name": "test_tool",
                            "arguments": "{}"
                        },
                        "extra_content": {
                            "google": {
                                "thought_signature": "nested_sig_xyz789"
                            }
                        }
                    }]
                }
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 1);

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            assert!(request.tool_call.is_ok());
            assert!(request.metadata.is_some());
            let metadata = request.metadata.as_ref().unwrap();
            let extra_content = metadata.get("extra_content").unwrap();
            assert_eq!(
                extra_content["google"]["thought_signature"],
                "nested_sig_xyz789"
            );
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_with_multiple_extra_fields() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_789",
                        "type": "function",
                        "function": {
                            "name": "test_tool",
                            "arguments": "{}"
                        },
                        "thoughtSignature": "sig_top_level",
                        "extra_content": {
                            "google": {
                                "thought_signature": "sig_nested"
                            }
                        },
                        "custom_field": "custom_value"
                    }]
                }
            }]
        });

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let metadata = request.metadata.as_ref().unwrap();
            assert_eq!(metadata.get("thoughtSignature").unwrap(), "sig_top_level");
            assert_eq!(
                metadata.get("extra_content").unwrap()["google"]["thought_signature"],
                "sig_nested"
            );
            assert_eq!(metadata.get("custom_field").unwrap(), "custom_value");
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_streaming_response_with_nested_extra_content() -> anyhow::Result<()> {
        let response_lines = r#"data: {"model":"test-model","choices":[{"delta":{"role":"assistant","tool_calls":[{"extra_content":{"google":{"thought_signature":"nested_stream_sig"}},"id":"call_nested","function":{"name":"test_tool","arguments":"{}"},"type":"function","index":0}]},"index":0,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110},"object":"chat.completion.chunk","id":"test-id","created":1234567890}
data: [DONE]"#;

        let response_stream =
            tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
        let messages = response_to_streaming_message(response_stream);
        pin!(messages);

        while let Some(Ok((message, _usage))) = messages.next().await {
            if let Some(msg) = message {
                if let MessageContent::ToolRequest(request) = &msg.content[0] {
                    assert!(request.tool_call.is_ok());
                    assert!(request.metadata.is_some());
                    let metadata = request.metadata.as_ref().unwrap();
                    let extra_content = metadata.get("extra_content").unwrap();
                    assert_eq!(
                        extra_content["google"]["thought_signature"],
                        "nested_stream_sig"
                    );
                    return Ok(());
                }
            }
        }

        panic!("Expected tool call message with nested extra_content metadata");
    }

    #[test]
    fn test_response_to_message_with_reasoning_content() -> anyhow::Result<()> {
        // Test capturing reasoning_content from DeepSeek reasoning models
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "reasoning_content": "Let me think about this step by step...",
                    "content": "The answer is 9.11 is greater than 9.8"
                }
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 25,
                "total_tokens": 35
            }
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        // First should be reasoning content
        if let MessageContent::Reasoning(reasoning) = &message.content[0] {
            assert_eq!(reasoning.text, "Let me think about this step by step...");
        } else {
            panic!("Expected Reasoning content");
        }

        // Second should be text content
        if let MessageContent::Text(text) = &message.content[1] {
            assert_eq!(text.text, "The answer is 9.11 is greater than 9.8");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_format_messages_with_reasoning_content() -> anyhow::Result<()> {
        // Test that reasoning_content is properly included in formatted messages
        let mut message = Message::assistant()
            .with_content(MessageContent::reasoning("Thinking through the problem..."))
            .with_text("The result is 42");

        // Add a tool call to test that reasoning_content works with tool calls
        message = message.with_tool_request(
            "tool1",
            Ok(rmcp::model::CallToolRequestParams {
                meta: None,
                task: None,
                name: "test_tool".into(),
                arguments: Some(rmcp::object!({"param": "value"})),
            }),
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");

        // Should have reasoning_content field
        assert!(spec[0].get("reasoning_content").is_some());
        assert_eq!(
            spec[0]["reasoning_content"],
            "Thinking through the problem..."
        );

        // Should have content
        assert_eq!(spec[0]["content"], "The result is 42");

        // Should have tool_calls
        assert!(spec[0]["tool_calls"].is_array());
        assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

        Ok(())
    }
}
