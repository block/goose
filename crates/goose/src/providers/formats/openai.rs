use crate::conversation::message::{Message, MessageContent, ProviderMetadata};
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
use regex;
use rmcp::model::{
    object, AnnotateAble, CallToolRequestParams, Content, ErrorCode, ErrorData, RawContent,
    ResourceContents, Role, Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::ops::Deref;

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
}

#[derive(Serialize, Deserialize, Debug)]
struct Delta {
    content: Option<String>,
    role: Option<String>,
    tool_calls: Option<Vec<DeltaToolCall>>,
    reasoning_details: Option<Vec<Value>>,
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
    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        let mut converted = json!({
            "role": message.role
        });

        let mut output = Vec::new();
        let mut content_array = Vec::new();
        let mut text_array = Vec::new();

        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    if !text.text.is_empty() {
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
                MessageContent::ToolResponse(response) => {
                    match &response.tool_result {
                        Ok(result) => {
                            // Send only contents with no audience or with Assistant in the audience
                            let abridged: Vec<_> = result
                                .content
                                .iter()
                                .filter(|content| {
                                    content
                                        .audience()
                                        .is_none_or(|audience| audience.contains(&Role::Assistant))
                                })
                                .cloned()
                                .collect();

                            // Process all content, replacing images with placeholder text
                            let mut tool_content = Vec::new();
                            let mut image_messages = Vec::new();

                            for content in abridged {
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
                                        let text = match &resource.resource {
                                            ResourceContents::TextResourceContents {
                                                text, ..
                                            } => text.clone(),
                                            _ => String::new(),
                                        };
                                        tool_content.push(Content::text(text));
                                    }
                                    _ => {
                                        tool_content.push(content);
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
                    content_array.push(convert_image(image, image_format));
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

/// Process a single tool call and return MessageContent
fn process_tool_call(id: String, function_name: String, arguments_str: String) -> MessageContent {
    let arguments_str = if arguments_str.is_empty() {
        "{}".to_string()
    } else {
        arguments_str
    };

    if !is_valid_function_name(&function_name) {
        let error = ErrorData {
            code: ErrorCode::INVALID_REQUEST,
            message: Cow::from(format!(
                "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                function_name
            )),
            data: None,
        };
        MessageContent::tool_request(id, Err(error))
    } else {
        match safely_parse_json(&arguments_str) {
            Ok(params) => MessageContent::tool_request(
                id,
                Ok(CallToolRequestParams {
                    meta: None,
                    task: None,
                    name: function_name.into(),
                    arguments: Some(object(params)),
                }),
            ),
            Err(e) => {
                let error = ErrorData {
                    code: ErrorCode::INVALID_PARAMS,
                    message: Cow::from(format!(
                        "Could not interpret tool use parameters for id {}: {}. Raw arguments: '{}'",
                        id, e, arguments_str
                    )),
                    data: None,
                };
                MessageContent::tool_request(id, Err(error))
            }
        }
    }
}

/// Process Qwen-format tool calls from text content
fn process_qwen_tool_calls(text_str: &str) -> Vec<MessageContent> {
    let mut content = Vec::new();

    if let Some(qwen_tool_calls) = parse_qwen_tool_calls(text_str) {
        for tool_call in qwen_tool_calls {
            let id = tool_call["id"].as_str().unwrap_or_default().to_string();
            let function_name = tool_call["function"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let arguments_str = tool_call["function"]["arguments"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            content.push(process_tool_call(id, function_name, arguments_str));
        }

        let remaining_text = strip_qwen_tool_calls(text_str);
        if !remaining_text.is_empty() {
            content.push(MessageContent::text(&remaining_text));
        }
    } else {
        content.push(MessageContent::text(text_str));
    }

    content
}

/// Process standard OpenAI-format tool calls
fn process_standard_tool_calls(tool_calls: &Value) -> Vec<MessageContent> {
    let mut content = Vec::new();

    if let Some(tool_calls_array) = tool_calls.as_array() {
        for tool_call in tool_calls_array {
            let id = tool_call["id"].as_str().unwrap_or_default().to_string();
            let function_name = tool_call["function"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let arguments_str = tool_call["function"]["arguments"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            content.push(process_tool_call(id, function_name, arguments_str));
        }
    }

    content
}

/// Convert OpenAI's API response to internal Message format
pub fn response_to_message(response: &Value) -> anyhow::Result<Message> {
    let Some(original) = response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|m| m.get("message"))
    else {
        return Ok(Message::new(
            Role::Assistant,
            chrono::Utc::now().timestamp(),
            Vec::new(),
        ));
    };

    let mut content = Vec::new();

    // Check for Qwen's XML-wrapped tool calls first
    if let Some(text) = original.get("content") {
        if let Some(text_str) = text.as_str() {
            content.extend(process_qwen_tool_calls(text_str));
        }
    }

    // Process standard OpenAI tool_calls format
    if let Some(tool_calls) = original.get("tool_calls") {
        content.extend(process_standard_tool_calls(tool_calls));
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

/// Parse Qwen's <tool_call> XML format and convert to standard tool_calls format.
///
/// Qwen models output tool calls wrapped in XML tags like:
/// <tool_call>
/// {"name": "tool_name", "arguments": {"param": "value"}}
/// </tool_call>
///
/// This function extracts these and converts them to the standard OpenAI format.
fn parse_qwen_tool_calls(content: &str) -> Option<Vec<Value>> {
    if !content.contains("<tool_call>") {
        return None;
    }

    let mut tool_calls = Vec::new();

    // Pattern to match <tool_call>...</tool_call> blocks
    // Using lazy matching (.*?) to handle multiple tool calls
    let re = match regex::Regex::new(
        r"(?is)<tool_call>s*
?(.*?)
?</tool_call>",
    ) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to compile Qwen tool call regex: {}", e);
            return None;
        }
    };

    for (idx, capture) in re.captures_iter(content).enumerate() {
        let json_str = match capture.get(1) {
            Some(m) => m.as_str().trim(),
            None => continue,
        };

        match serde_json::from_str::<Value>(json_str) {
            Ok(tool_data) => {
                let name = tool_data
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();

                let arguments = tool_data.get("arguments").cloned().unwrap_or(json!({}));

                // Generate a unique ID for this tool call
                let id = format!("qwen_tool_{}", idx);

                let tool_call = json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": serde_json::to_string(&arguments).unwrap_or_else(|_| "{}".to_string())
                    }
                });

                tracing::debug!("Parsed Qwen tool call: {:?}", &tool_call);
                tool_calls.push(tool_call);
            }
            Err(e) => {
                let preview: String = json_str.chars().take(100).collect();
                tracing::warn!("Failed to parse Qwen tool call JSON '{}': {}", preview, e);
                continue;
            }
        }
    }

    if tool_calls.is_empty() {
        None
    } else {
        tracing::info!("Extracted {} tool calls from Qwen format", tool_calls.len());
        Some(tool_calls)
    }
}

/// Strip Qwen XML tool call tags from text content
fn strip_qwen_tool_calls(content: &str) -> String {
    if !content.contains("<tool_call>") {
        return content.to_string();
    }

    let re = regex::Regex::new(r"(?is)<tool_call>.*?</tool_call>").unwrap();
    re.replace_all(content, "").trim().to_string()
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
            }

            let mut usage = extract_usage_with_output_tokens(&chunk);

            if chunk.choices.is_empty() {
                yield (None, usage)
            } else if chunk.choices[0].delta.tool_calls.as_ref().is_some_and(|tc| !tc.is_empty()) {
                let mut tool_call_data: std::collections::HashMap<i32, (String, String, String)> = std::collections::HashMap::new();

                if let Some(tool_calls) = &chunk.choices[0].delta.tool_calls {
                    for tool_call in tool_calls {
                        if let (Some(index), Some(id), Some(name)) = (tool_call.index, &tool_call.id, &tool_call.function.name) {
                            tool_call_data.insert(index, (id.clone(), name.clone(), tool_call.function.arguments.clone()));
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
                                    if let Some(delta_tool_calls) = &tool_chunk.choices[0].delta.tool_calls {
                                        for delta_call in delta_tool_calls {
                                            if let Some(index) = delta_call.index {
                                                if let Some((_, _, ref mut args)) = tool_call_data.get_mut(&index) {
                                                    args.push_str(&delta_call.function.arguments);
                                                } else if let (Some(id), Some(name)) = (&delta_call.id, &delta_call.function.name) {
                                                    tool_call_data.insert(index, (id.clone(), name.clone(), delta_call.function.arguments.clone()));
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

                let metadata: Option<ProviderMetadata> = if !accumulated_reasoning.is_empty() {
                    let mut map = ProviderMetadata::new();
                    map.insert("reasoning_details".to_string(), json!(accumulated_reasoning));
                    Some(map)
                } else {
                    None
                };

                let mut contents = Vec::new();
                let mut sorted_indices: Vec<_> = tool_call_data.keys().cloned().collect();
                sorted_indices.sort();

                for index in sorted_indices {
                    if let Some((id, function_name, arguments)) = tool_call_data.get(&index) {
                        let parsed = if arguments.is_empty() {
                            Ok(json!({}))
                        } else {
                            serde_json::from_str::<Value>(arguments)
                        };

                        let content = match parsed {
                            Ok(params) => {
                                MessageContent::tool_request_with_metadata(
                                    id.clone(),
                                    Ok(CallToolRequestParams {
                                        meta: None, task: None,
                                        name: function_name.clone().into(),
                                        arguments: Some(object(params))
                                    }),
                                    metadata.as_ref(),
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
                                MessageContent::tool_request_with_metadata(id.clone(), Err(error), metadata.as_ref())
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
            } else if chunk.choices[0].delta.content.is_some() {
                let text = chunk.choices[0].delta.content.as_ref().unwrap();
                let mut msg = Message::new(
                    Role::Assistant,
                    chrono::Utc::now().timestamp(),
                    vec![MessageContent::text(text)],
                );

                // Add ID if present
                if let Some(id) = chunk.id {
                    msg = msg.with_id(id);
                }

                yield (
                    Some(msg),
                    usage,
                )
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

        // Create message with image path
        let message = Message::user().with_text(format!("Here is an image: {}", png_path_str));
        let spec = format_messages(&[message], &ImageFormat::OpenAi);

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
            fast_model: None,
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
            fast_model: None,
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
            fast_model: None,
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
    fn test_parse_qwen_tool_calls_single() -> anyhow::Result<()> {
        let content = r#"Let me check the file for you.
<tool_call>
{"name": "developer__text_editor", "arguments": {"path": "test.rs", "command": "view"}}
</tool_call>"#;

        let tool_calls = parse_qwen_tool_calls(content).expect("Should parse Qwen tool call");

        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["id"], "qwen_tool_0");
        assert_eq!(tool_calls[0]["type"], "function");
        assert_eq!(tool_calls[0]["function"]["name"], "developer__text_editor");

        let args_str = tool_calls[0]["function"]["arguments"].as_str().unwrap();
        let args: Value = serde_json::from_str(args_str)?;
        assert_eq!(args["path"], "test.rs");
        assert_eq!(args["command"], "view");

        Ok(())
    }

    #[test]
    fn test_parse_qwen_tool_calls_multiple() -> anyhow::Result<()> {
        let content = r#"I'll run two commands for you.
<tool_call>
{"name": "developer__shell", "arguments": {"command": "ls"}}
</tool_call>
Let me also check the status.
<tool_call>
{"name": "developer__shell", "arguments": {"command": "git status"}}
</tool_call>"#;

        let tool_calls =
            parse_qwen_tool_calls(content).expect("Should parse multiple Qwen tool calls");

        assert_eq!(tool_calls.len(), 2);
        assert_eq!(tool_calls[0]["id"], "qwen_tool_0");
        assert_eq!(tool_calls[0]["function"]["name"], "developer__shell");
        assert_eq!(tool_calls[1]["id"], "qwen_tool_1");
        assert_eq!(tool_calls[1]["function"]["name"], "developer__shell");

        Ok(())
    }

    #[test]
    fn test_parse_qwen_tool_calls_no_tags() {
        let content = "This is just regular text without any tool calls.";
        let result = parse_qwen_tool_calls(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_qwen_tool_calls_malformed_json() {
        let content = r#"<tool_call>
{"name": "test", "arguments": {invalid json}
</tool_call>"#;

        let result = parse_qwen_tool_calls(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_qwen_tool_calls_empty_arguments() -> anyhow::Result<()> {
        let content = r#"<tool_call>
{"name": "test_tool"}
</tool_call>"#;

        let tool_calls =
            parse_qwen_tool_calls(content).expect("Should parse with missing arguments");
        assert_eq!(tool_calls.len(), 1);

        let args_str = tool_calls[0]["function"]["arguments"].as_str().unwrap();
        let args: Value = serde_json::from_str(args_str)?;
        assert_eq!(args, json!({}));

        Ok(())
    }

    #[test]
    fn test_strip_qwen_tool_calls() {
        let content = r#"Let me check the file.
<tool_call>
{"name": "test", "arguments": {"x": 1}}
</tool_call>
I've executed the command."#;

        let result = strip_qwen_tool_calls(content);
        assert_eq!(
            result,
            "Let me check the file.

I've executed the command."
        );
    }

    #[test]
    fn test_strip_qwen_tool_calls_no_tags() {
        let content = "Just regular text.";
        let result = strip_qwen_tool_calls(content);
        assert_eq!(result, content);
    }

    #[test]
    fn test_response_to_message_qwen_format() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "content": r#"Let me check that file.
<tool_call>
{"name": "developer__text_editor", "arguments": {"path": "test.rs", "command": "view"}}
</tool_call>"#
                }
            }]
        });

        let message = response_to_message(&response)?;

        assert_eq!(message.content.len(), 2);

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "developer__text_editor");
            assert_eq!(request.id, "qwen_tool_0");
            if let Some(args) = &tool_call.arguments {
                assert_eq!(args.get("path").and_then(|v| v.as_str()), Some("test.rs"));
                assert_eq!(args.get("command").and_then(|v| v.as_str()), Some("view"));
            } else {
                panic!("Expected arguments");
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        if let MessageContent::Text(text) = &message.content[1] {
            assert_eq!(text.text, "Let me check that file.");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_qwen_format_multiple_tools() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "content": r#"Running two commands:
<tool_call>
{"name": "developer__shell", "arguments": {"command": "ls"}}
</tool_call>
<tool_call>
{"name": "developer__shell", "arguments": {"command": "pwd"}}
</tool_call>
Done!"#
                }
            }]
        });

        let message = response_to_message(&response)?;

        assert_eq!(message.content.len(), 3);

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            assert_eq!(request.tool_call.as_ref().unwrap().name, "developer__shell");
            assert_eq!(request.id, "qwen_tool_0");
        } else {
            panic!("Expected ToolRequest for first tool");
        }

        if let MessageContent::ToolRequest(request) = &message.content[1] {
            assert_eq!(request.tool_call.as_ref().unwrap().name, "developer__shell");
            assert_eq!(request.id, "qwen_tool_1");
        } else {
            panic!("Expected ToolRequest for second tool");
        }

        if let MessageContent::Text(text) = &message.content[2] {
            assert!(text.text.contains("Running two commands"));
            assert!(text.text.contains("Done!"));
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_qwen_format_invalid_function_name() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "content": r#"<tool_call>
{"name": "invalid fn name", "arguments": {"x": 1}}
</tool_call>"#
                }
            }]
        });

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ErrorData {
                    code: ErrorCode::INVALID_REQUEST,
                    message: msg,
                    data: None,
                }) => {
                    assert!(msg.contains("invalid characters"));
                }
                _ => panic!("Expected INVALID_REQUEST error"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_qwen_and_standard_formats() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "content": "Some text",
                    "tool_calls": [{
                        "id": "standard_1",
                        "function": {
                            "name": "standard_tool",
                            "arguments": "{\"param\": \"value\"}"
                        }
                    }]
                }
            }]
        });

        let message = response_to_message(&response)?;

        assert_eq!(message.content.len(), 2);

        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Some text");
        } else {
            panic!("Expected Text content first");
        }

        if let MessageContent::ToolRequest(request) = &message.content[1] {
            assert_eq!(request.id, "standard_1");
            assert_eq!(request.tool_call.as_ref().unwrap().name, "standard_tool");
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }
}
