use crate::conversation::message::{Message, MessageContent};
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
    object, AnnotateAble, CallToolRequestParam, Content, ErrorCode, ErrorData, RawContent,
    ResourceContents, Role, Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::ops::Deref;

// ============================================================================
// Responses API Types (for gpt-5.1-codex and similar models)
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponsesApiResponse {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: String,
    pub model: String,
    pub output: Vec<ResponseOutputItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponseReasoningInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponseUsage>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ResponseOutputItem {
    Reasoning {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<Vec<String>>,
    },
    Message {
        id: String,
        status: String,
        role: String,
        content: Vec<ResponseContentBlock>,
    },
    FunctionCall {
        id: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
        name: String,
        arguments: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ResponseContentBlock {
    OutputText {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Vec<Value>>,
    },
    ToolCall {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseReasoningInfo {
    pub effort: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_tokens: i32,
}

// ============================================================================
// Responses API Streaming Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ResponsesStreamEvent {
    #[serde(rename = "response.created")]
    ResponseCreated {
        sequence_number: i32,
        response: ResponseMetadata,
    },
    #[serde(rename = "response.in_progress")]
    ResponseInProgress {
        sequence_number: i32,
        response: ResponseMetadata,
    },
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded {
        sequence_number: i32,
        output_index: i32,
        item: ResponseOutputItemInfo,
    },
    #[serde(rename = "response.content_part.added")]
    ContentPartAdded {
        sequence_number: i32,
        item_id: String,
        output_index: i32,
        content_index: i32,
        part: ContentPart,
    },
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta {
        sequence_number: i32,
        item_id: String,
        output_index: i32,
        content_index: i32,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        logprobs: Option<Vec<Value>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        obfuscation: Option<String>,
    },
    #[serde(rename = "response.output_item.done")]
    OutputItemDone {
        sequence_number: i32,
        output_index: i32,
        item: ResponseOutputItemInfo,
    },
    #[serde(rename = "response.content_part.done")]
    ContentPartDone {
        sequence_number: i32,
        item_id: String,
        output_index: i32,
        content_index: i32,
        part: ContentPart,
    },
    #[serde(rename = "response.completed")]
    ResponseCompleted {
        sequence_number: i32,
        response: ResponseMetadata,
    },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta {
        sequence_number: i32,
        item_id: String,
        output_index: i32,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        obfuscation: Option<String>,
    },
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone {
        sequence_number: i32,
        item_id: String,
        output_index: i32,
        arguments: String,
    },
    #[serde(rename = "error")]
    Error {
        error: Value,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: String,
    pub model: String,
    pub output: Vec<ResponseOutputItemInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ResponseUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponseReasoningInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ResponseOutputItemInfo {
    Reasoning {
        id: String,
        summary: Vec<String>,
    },
    Message {
        id: String,
        status: String,
        role: String,
        content: Vec<ContentPart>,
    },
    FunctionCall {
        id: String,
        status: String,
        call_id: String,
        name: String,
        arguments: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ContentPart {
    OutputText {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Vec<Value>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        logprobs: Option<Vec<Value>>,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct DeltaToolCallFunction {
    name: Option<String>,
    arguments: String, // chunk of encoded JSON,
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
                        Ok(contents) => {
                            // Send only contents with no audience or with Assistant in the audience
                            let abridged: Vec<_> = contents
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
                MessageContent::ToolConfirmationRequest(_) => {
                    // Skip tool confirmation requests
                }
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

                if !is_valid_function_name(&function_name) {
                    let error = ErrorData {
                        code: ErrorCode::INVALID_REQUEST,
                        message: Cow::from(format!(
                            "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                            function_name
                        )),
                        data: None,
                    };
                    content.push(MessageContent::tool_request(id, Err(error)));
                } else {
                    match safely_parse_json(&arguments_str) {
                        Ok(params) => {
                            content.push(MessageContent::tool_request(
                                id,
                                Ok(CallToolRequestParam {
                                    name: function_name.into(),
                                    arguments: Some(object(params)),
                                }),
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
                            content.push(MessageContent::tool_request(id, Err(error)));
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

            let usage = chunk.usage.as_ref().and_then(|u| {
                chunk.model.as_ref().map(|model| {
                    ProviderUsage {
                        usage: get_usage(u),
                        model: model.clone(),
                    }
                })
            });

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

                // Check if this chunk already has finish_reason "tool_calls"
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

                                if !tool_chunk.choices.is_empty() {
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
                                    } else {
                                        done = true;
                                    }

                                    if tool_chunk.choices[0].finish_reason == Some("tool_calls".to_string()) {
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
                                MessageContent::tool_request(
                                    id.clone(),
                                    Ok(CallToolRequestParam { name: function_name.clone().into(), arguments: Some(object(params)) }),
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
                                MessageContent::tool_request(id.clone(), Err(error))
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
                    if chunk.choices[0].finish_reason.is_some() {
                        usage
                    } else {
                        None
                    },
                )
            } else if usage.is_some() {
                yield (None, usage)
            }
        }
    }
}

// ============================================================================
// Responses API Helper Functions
// ============================================================================

pub fn create_responses_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> anyhow::Result<Value, Error> {
    let mut conversation_parts = Vec::new();

    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        for content in &message.content {
            match content {
                MessageContent::Text(text) if !text.text.is_empty() => {
                    let role_str = if message.role == Role::User {
                        "User"
                    } else {
                        "Assistant"
                    };
                    conversation_parts.push(format!("{}: {}", role_str, text.text));
                }
                MessageContent::ToolRequest(request) => {
                    if let Ok(tool_call) = &request.tool_call {
                        conversation_parts.push(format!(
                            "Tool Call: {} with arguments {:?}",
                            tool_call.name, tool_call.arguments
                        ));
                    }
                }
                MessageContent::ToolResponse(response) => {
                    if let Ok(contents) = &response.tool_result {
                        let text_content: Vec<String> = contents
                            .iter()
                            .filter_map(|c| {
                                if let RawContent::Text(t) = c.deref() {
                                    Some(t.text.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if !text_content.is_empty() {
                            conversation_parts
                                .push(format!("Tool Result: {}", text_content.join(" ")));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let input = if conversation_parts.is_empty() {
        "Hello".to_string()
    } else {
        conversation_parts.join("\n\n")
    };

    let mut payload = json!({
        "model": model_config.model_name,
        "input": input,
        "instructions": system,
    });

    if !tools.is_empty() {
        let tools_spec: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                })
            })
            .collect();

        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tools_spec));
    }

    if let Some(temp) = model_config.temperature {
        payload
            .as_object_mut()
            .unwrap()
            .insert("temperature".to_string(), json!(temp));
    }

    if let Some(tokens) = model_config.max_tokens {
        payload
            .as_object_mut()
            .unwrap()
            .insert("max_output_tokens".to_string(), json!(tokens));
    }

    Ok(payload)
}

pub fn responses_api_to_message(response: &ResponsesApiResponse) -> anyhow::Result<Message> {
    let mut content = Vec::new();

    for item in &response.output {
        match item {
            ResponseOutputItem::Reasoning { .. } => {
                continue;
            }
            ResponseOutputItem::Message {
                content: msg_content,
                ..
            } => {
                for block in msg_content {
                    match block {
                        ResponseContentBlock::OutputText { text, .. } => {
                            if !text.is_empty() {
                                content.push(MessageContent::text(text));
                            }
                        }
                        ResponseContentBlock::ToolCall { id, name, input } => {
                            content.push(MessageContent::tool_request(
                                id.clone(),
                                Ok(CallToolRequestParam {
                                    name: name.clone().into(),
                                    arguments: Some(object(input.clone())),
                                }),
                            ));
                        }
                    }
                }
            }
            ResponseOutputItem::FunctionCall {
                id,
                name,
                arguments,
                ..
            } => {
                let parsed_args = if arguments.is_empty() {
                    json!({})
                } else {
                    serde_json::from_str(arguments).unwrap_or_else(|_| json!({}))
                };

                content.push(MessageContent::tool_request(
                    id.clone(),
                    Ok(CallToolRequestParam {
                        name: name.clone().into(),
                        arguments: Some(object(parsed_args)),
                    }),
                ));
            }
        }
    }

    let mut message = Message::new(Role::Assistant, chrono::Utc::now().timestamp(), content);

    message = message.with_id(response.id.clone());

    Ok(message)
}

pub fn get_responses_usage(response: &ResponsesApiResponse) -> Usage {
    response.usage.as_ref().map_or_else(Usage::default, |u| {
        Usage::new(
            Some(u.input_tokens),
            Some(u.output_tokens),
            Some(u.total_tokens),
        )
    })
}

pub fn responses_api_to_streaming_message<S>(
    mut stream: S,
) -> impl Stream<Item = anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> + 'static
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    try_stream! {
        use futures::StreamExt;

        let mut accumulated_text = String::new();
        let mut response_id: Option<String> = None;
        let mut model_name: Option<String> = None;
        let mut final_usage: Option<ProviderUsage> = None;
        let mut output_items: Vec<ResponseOutputItemInfo> = Vec::new();
        let mut is_text_response = false;

        'outer: while let Some(response) = stream.next().await {
            let response_str = response?;

            // Skip empty lines
            if response_str.trim().is_empty() {
                continue;
            }

            // Parse SSE format: "event: <type>\ndata: <json>"
            // For now, we only care about the data line
            let data_line = if response_str.starts_with("data: ") {
                response_str.strip_prefix("data: ").unwrap()
            } else if response_str.starts_with("event: ") {
                // Skip event type lines
                continue;
            } else {
                // Try to parse as-is in case there's no prefix
                &response_str
            };

            // Skip [DONE] marker
            if data_line == "[DONE]" {
                break 'outer;
            }

            let event: ResponsesStreamEvent = serde_json::from_str(data_line)
                .map_err(|e| anyhow!("Failed to parse Responses stream event: {}: {:?}", e, data_line))?;

            match event {
                ResponsesStreamEvent::ResponseCreated { response, .. } |
                ResponsesStreamEvent::ResponseInProgress { response, .. } => {
                    response_id = Some(response.id);
                    model_name = Some(response.model);
                }

                ResponsesStreamEvent::OutputTextDelta { delta, .. } => {
                    is_text_response = true;
                    accumulated_text.push_str(&delta);

                    // Yield incremental text updates for true streaming
                    let mut content = Vec::new();
                    if !accumulated_text.is_empty() {
                        content.push(MessageContent::text(&accumulated_text));
                    }
                    let msg = Message::new(Role::Assistant, chrono::Utc::now().timestamp(), content);
                    yield (Some(msg), None);
                }

                ResponsesStreamEvent::OutputItemDone { item, .. } => {
                    output_items.push(item);
                }

                ResponsesStreamEvent::ResponseCompleted { response, .. } => {
                    // Collect final usage data
                    if let Some(usage) = response.usage {
                        if let Some(model) = &model_name {
                            final_usage = Some(ProviderUsage {
                                usage: Usage::new(
                                    Some(usage.input_tokens),
                                    Some(usage.output_tokens),
                                    Some(usage.total_tokens),
                                ),
                                model: model.clone(),
                            });
                        }
                    }

                    // For complete output, use the response output items
                    if !response.output.is_empty() {
                        output_items = response.output;
                    }

                    break 'outer;
                }

                ResponsesStreamEvent::FunctionCallArgumentsDelta { .. } => {
                    // Function call arguments are being streamed, but we'll get the complete
                    // arguments in the OutputItemDone event, so we can ignore deltas for now
                }

                ResponsesStreamEvent::FunctionCallArgumentsDone { .. } => {
                    // Arguments are complete, will be in the OutputItemDone event
                }

                ResponsesStreamEvent::Error { error } => {
                    Err(anyhow!("Responses API error: {:?}", error))?;
                }

                _ => {
                    // Ignore other event types (OutputItemAdded, ContentPartAdded, ContentPartDone)
                }
            }
        }

        // Yield final message if we have tool calls or non-text output
        if !is_text_response || !output_items.is_empty() {
            let mut content = Vec::new();

            // Process output items for tool calls
            for item in output_items {
                match item {
                    ResponseOutputItemInfo::Reasoning { .. } => {
                        // Skip reasoning items
                    }
                    ResponseOutputItemInfo::Message { content: parts, .. } => {
                        for part in parts {
                            match part {
                                ContentPart::OutputText { text, .. } => {
                                    if !text.is_empty() && !is_text_response {
                                        content.push(MessageContent::text(&text));
                                    }
                                }
                                ContentPart::ToolCall { id, name, arguments } => {
                                    let parsed_args = if arguments.is_empty() {
                                        json!({})
                                    } else {
                                        serde_json::from_str(&arguments).unwrap_or_else(|_| json!({}))
                                    };

                                    content.push(MessageContent::tool_request(
                                        id,
                                        Ok(CallToolRequestParam {
                                            name: name.into(),
                                            arguments: Some(object(parsed_args)),
                                        }),
                                    ));
                                }
                            }
                        }
                    }
                    ResponseOutputItemInfo::FunctionCall { call_id, name, arguments, .. } => {
                        let parsed_args = if arguments.is_empty() {
                            json!({})
                        } else {
                            serde_json::from_str(&arguments).unwrap_or_else(|_| json!({}))
                        };

                        content.push(MessageContent::tool_request(
                            call_id,
                            Ok(CallToolRequestParam {
                                name: name.into(),
                                arguments: Some(object(parsed_args)),
                            }),
                        ));
                    }
                }
            }

            if !content.is_empty() {
                let mut message = Message::new(Role::Assistant, chrono::Utc::now().timestamp(), content);
                if let Some(id) = response_id {
                    message = message.with_id(id);
                }
                yield (Some(message), final_usage);
            } else if let Some(usage) = final_usage {
                yield (None, Some(usage));
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
    let mut tools_spec = if !tools.is_empty() {
        format_tools(tools)?
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
    // o1, o3 models currently don't support temperature
    if !is_ox_model {
        if let Some(temp) = model_config.temperature {
            payload
                .as_object_mut()
                .unwrap()
                .insert("temperature".to_string(), json!(temp));
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
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
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
                Ok(CallToolRequestParam {
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

        messages
            .push(Message::user().with_tool_response(tool_id, Ok(vec![Content::text("Result")])));

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
            Ok(CallToolRequestParam {
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

        messages
            .push(Message::user().with_tool_response(tool_id, Ok(vec![Content::text("Result")])));

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
            Ok(CallToolRequestParam {
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
            Ok(CallToolRequestParam {
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
            Ok(CallToolRequestParam {
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
            Ok(CallToolRequestParam {
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
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
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
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
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
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
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

        let response_stream =
            tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
        let messages = response_to_streaming_message(response_stream);
        pin!(messages);

        while let Some(Ok((message, _usage))) = messages.next().await {
            if let Some(msg) = message {
                println!("{:?}", msg);
                if msg.content.len() == 2 {
                    if let (MessageContent::ToolRequest(req1), MessageContent::ToolRequest(req2)) =
                        (&msg.content[0], &msg.content[1])
                    {
                        if req1.tool_call.is_ok() && req2.tool_call.is_ok() {
                            // We expect two tool calls in the response
                            assert_eq!(req1.tool_call.as_ref().unwrap().name, "developer__shell");
                            assert_eq!(req2.tool_call.as_ref().unwrap().name, "developer__shell");
                            return Ok(());
                        }
                    }
                }
            }
        }

        panic!("Expected tool call message with two calls, but did not see it");
    }
}
