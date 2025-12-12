use super::sap_ai_core_payload::*;
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::{ProviderUsage, Usage};
use crate::providers::utils::{
    convert_image, detect_image_path, is_valid_function_name, load_image_file, safely_parse_json,
    sanitize_function_name, ImageFormat,
};
use anyhow::{anyhow, Result};
use async_stream::try_stream;
use futures::Stream;
use rmcp::model::{object, CallToolRequestParam, ErrorCode, ErrorData, Role, Tool};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;

/// Helper function to convert messages to ChatMessage format
#[allow(clippy::too_many_lines)]
fn convert_messages_to_chat_messages(
    messages: &[Message],
    image_format: &ImageFormat,
) -> Result<Vec<ChatMessage>> {
    let mut chat_messages = Vec::new();

    for message in messages {
        // Collect content from this message
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();
        let mut user_content_items = Vec::new();
        let mut has_image = false;
        let mut has_tool_requests = false;

        // Process all content items for this message
        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    if !text.text.is_empty() {
                        // Check for image paths in the text
                        if let Some(image_path) = detect_image_path(&text.text) {
                            // Try to load and convert the image
                            match load_image_file(image_path) {
                                Ok(image) => {
                                    tracing::info!(
                                        "Successfully loaded image from path: {}",
                                        image_path
                                    );
                                    // Add accumulated text content first if any
                                    if !text_content.is_empty() {
                                        user_content_items.push(UserChatMessageContentItem::Text {
                                            text: text_content.clone(),
                                        });
                                        text_content.clear();
                                    }

                                    // Add the text with image path
                                    user_content_items.push(UserChatMessageContentItem::Text {
                                        text: text.text.clone(),
                                    });

                                    // Add the converted image
                                    let converted_image = convert_image(&image, image_format);
                                    if let Some(image_url) = converted_image
                                        .get("image_url")
                                        .and_then(|v| v.get("url"))
                                        .and_then(|v| v.as_str())
                                    {
                                        user_content_items.push(
                                            UserChatMessageContentItem::ImageUrl {
                                                image_url: ImageContentUrl {
                                                    url: image_url.to_string(),
                                                    detail: "auto".to_string(),
                                                },
                                            },
                                        );
                                        has_image = true;
                                        tracing::info!("Successfully converted and added image to message content");
                                    } else {
                                        tracing::error!("Failed to extract image URL from converted image for path: {}", image_path);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to load image from path '{}': {}. Using text content only.", image_path, e);
                                    // If image loading fails, just use the text
                                    if !text_content.is_empty() && !text_content.ends_with('\n') {
                                        text_content.push('\n');
                                    }
                                    text_content.push_str(&text.text);
                                }
                            }
                        } else {
                            // No image path detected, add to text content
                            if !text_content.is_empty() && !text_content.ends_with('\n') {
                                text_content.push('\n');
                            }
                            text_content.push_str(&text.text);
                        }
                    }
                }
                MessageContent::Image(image) => {
                    has_image = true;
                    // Add accumulated text content first if any
                    if !text_content.is_empty() {
                        user_content_items.push(UserChatMessageContentItem::Text {
                            text: text_content.clone(),
                        });
                        text_content.clear();
                    }
                    // Add image - construct data URL from data and mime_type
                    let data_url = format!("data:{};base64,{}", image.mime_type, image.data);
                    user_content_items.push(UserChatMessageContentItem::ImageUrl {
                        image_url: ImageContentUrl {
                            url: data_url,
                            detail: "auto".to_string(),
                        },
                    });
                }
                MessageContent::ToolRequest(tool_request) => {
                    has_tool_requests = true;
                    if let Ok(tool_call) = &tool_request.tool_call {
                        let sanitized_name = sanitize_function_name(&tool_call.name);
                        let arguments_str = match &tool_call.arguments {
                            Some(args) => {
                                match serde_json::to_string(args) {
                                    Ok(json_str) => {
                                        tracing::info!("Successfully serialized tool call arguments for {}: {}", tool_call.name, json_str);
                                        json_str
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to serialize tool call arguments for '{}': {}. Using empty object.", tool_call.name, e);
                                        "{}".to_string()
                                    }
                                }
                            }
                            None => {
                                tracing::info!(
                                    "No arguments provided for tool call '{}'",
                                    tool_call.name
                                );
                                "{}".to_string()
                            }
                        };

                        tool_calls.push(MessageToolCall {
                            id: tool_request.id.clone(),
                            tool_type: "function".to_string(),
                            function: ToolCallFunction {
                                name: Some(sanitized_name),
                                arguments: Some(arguments_str),
                            },
                        });
                    }
                }
                MessageContent::ToolResponse(tool_response) => match &tool_response.tool_result {
                    // Convert tool responses to separate tool messages (regardless of original role)
                    Ok(result) => {
                        let result_text = result
                            .content
                            .iter()
                            .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                            .collect::<Vec<_>>()
                            .join("\n");

                        chat_messages.push(ChatMessage::Tool {
                            tool_call_id: tool_response.id.clone(),
                            content: ChatMessageContent::Text(result_text),
                        });
                    }

                    Err(error) => {
                        chat_messages.push(ChatMessage::Tool {
                            tool_call_id: tool_response.id.clone(),
                            content: ChatMessageContent::Text(error.to_string()),
                        });
                    }
                },
                // Skip all Claude-specific or unsupported content types
                MessageContent::Thinking(_)
                | MessageContent::RedactedThinking(_)
                | MessageContent::ToolConfirmationRequest(_)
                | MessageContent::FrontendToolRequest(_)
                | MessageContent::ActionRequired(_)
                | MessageContent::SystemNotification(_) => {
                    continue;
                }
            }
        }

        // Build the appropriate message type based on role and collected content
        match message.role {
            Role::User => {
                // Add any remaining text content for images
                if has_image && !text_content.is_empty() {
                    user_content_items.push(UserChatMessageContentItem::Text {
                        text: text_content.clone(),
                    });
                }

                // Create user message if we have content
                if has_image && !user_content_items.is_empty() {
                    chat_messages.push(ChatMessage::User {
                        content: UserChatMessageContent::Array(user_content_items),
                    });
                } else if !text_content.is_empty() {
                    chat_messages.push(ChatMessage::User {
                        content: UserChatMessageContent::Text(text_content),
                    });
                }
            }
            Role::Assistant => {
                // Only create assistant message if we have text content or tool requests
                if !text_content.is_empty() || has_tool_requests {
                    let content = if text_content.is_empty() && has_tool_requests {
                        Some(ChatMessageContent::Text(String::new())) // SAP AI Core requires content field, even if empty
                    } else if !text_content.is_empty() {
                        Some(ChatMessageContent::Text(text_content))
                    } else {
                        Some(ChatMessageContent::Text(String::new())) // Always provide content field
                    };

                    chat_messages.push(ChatMessage::Assistant {
                        content,
                        refusal: None,
                        tool_calls: if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        },
                    });
                }
            } // Tool messages are handled via ToolResponse content, not role
              // System messages are handled via system_prompt parameter, not role
        }
    }

    Ok(chat_messages)
}

/// Helper function to convert Goose tools to SAP AI Core ChatCompletionTool format
fn convert_tools_to_sap_format(tools: &[Tool]) -> Result<Vec<ChatCompletionTool>> {
    let mut sap_tools = Vec::new();

    for tool in tools {
        // Ensure schema has required properties for SAP AI Core validation
        let mut schema = (*tool.input_schema).clone();

        // SAP AI Core requires "type": "object" at minimum
        if !schema.contains_key("type") {
            schema.insert(
                "type".to_string(),
                serde_json::Value::String("object".to_string()),
            );
        }

        // If no properties defined, add empty properties object
        if !schema.contains_key("properties") {
            schema.insert(
                "properties".to_string(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }

        sap_tools.push(ChatCompletionTool {
            tool_type: "function".to_string(),
            function: FunctionObject {
                name: Some(tool.name.to_string()),
                description: tool.description.as_ref().map(|d| d.to_string()),
                parameters: Some(serde_json::Value::Object(schema)),
                strict: false, // Default to false unless specified
            },
        });
    }

    Ok(sap_tools)
}

/// Convert internal Message format to SAP AI Core's orchestration format
pub fn create_request(
    messages: &[Message],
    _tools: &[Tool],
    _model_config: &ModelConfig,
    system_prompt: &str,
    stream_supported: bool,
) -> Result<Value> {
    // Process all messages in template - no history separation to maintain proper sequence

    // Convert all messages to SAP AI Core template format
    let mut template_messages = Vec::new();

    // Add system message first if provided
    if !system_prompt.is_empty() {
        template_messages.push(ChatMessage::System {
            content: ChatMessageContent::Text(system_prompt.to_string()),
        });
    }

    // Process all messages and add to template
    let image_format = ImageFormat::OpenAi; // Default to OpenAI format for SAP AI Core
    let chat_messages = convert_messages_to_chat_messages(messages, &image_format)?;
    template_messages.extend(chat_messages);

    // Create the SAP AI Core completion request
    let request = CompletionPostReq {
        config: OrchestrationConfig {
            modules: ModuleConfigs {
                prompt_templating: PromptTemplatingModuleConfig {
                    prompt: TemplateOrRef::Template(Template {
                        template: template_messages,
                        defaults: None,
                        response_format: None,
                        tools: if _tools.is_empty() {
                            None
                        } else {
                            Some(convert_tools_to_sap_format(_tools)?)
                        },
                    }),
                    model: LlmModelDetails {
                        name: _model_config.model_name.to_string(),
                        version: "latest".to_string(),
                        params: None,
                    },
                },
                filtering: None,
                masking: None,
                grounding: None,
                translation: None,
            },
            stream: if stream_supported {
                Some(GlobalStreamOptions {
                    enabled: true,
                    chunk_size: 100,
                    delimiters: None,
                })
            } else {
                Some(GlobalStreamOptions {
                    enabled: false,
                    chunk_size: 100,
                    delimiters: None,
                })
            },
        },
        placeholder_values: None,
        messages_history: None, // Keep all messages in template to maintain proper sequence
    };

    Ok(serde_json::to_value(request)?)
}

/// Parse SAP AI Core response and convert to internal Message format
pub fn response_to_message(response_body: &str) -> Result<Message> {
    let response: CompletionPostRes = serde_json::from_str(response_body).map_err(|e| {
        tracing::error!(
            "Failed to parse SAP AI Core response JSON: {}. Response body (first 500 chars): {}",
            e,
            response_body.chars().take(500).collect::<String>()
        );
        anyhow!("Failed to parse SAP AI Core response: {}", e)
    })?;

    // Use final_result for the response
    if response.final_result.choices.is_empty() {
        tracing::error!("SAP AI Core response validation failed: no choices in final_result. Response structure: request_id={}, model={}",
            response.request_id, response.final_result.model);
        return Err(anyhow!("No choices in SAP AI Core final result"));
    }

    let choice = &response.final_result.choices[0];
    let message = &choice.message;
    let mut contents = Vec::new();

    // Handle text content
    if let Some(content) = &message.content {
        if !content.trim().is_empty() {
            contents.push(MessageContent::text(content));
        }
    }

    // Handle tool calls
    if let Some(tool_calls) = &message.tool_calls {
        for tool_call in tool_calls {
            let function_name = tool_call
                .function
                .name
                .as_ref()
                .cloned()
                .unwrap_or_default();
            let arguments_str = tool_call
                .function
                .arguments
                .as_ref()
                .cloned()
                .unwrap_or_default();

            // Validate function name
            if !is_valid_function_name(&function_name) {
                let error = ErrorData {
                    code: ErrorCode::INVALID_REQUEST,
                    message: Cow::from(format!(
                        "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                        function_name
                    )),
                    data: None,
                };
                contents.push(MessageContent::tool_request(
                    tool_call.id.clone(),
                    Err(error),
                ));
                continue;
            }

            // Use safely_parse_json for argument parsing
            let arguments_str = if arguments_str.trim().is_empty() {
                "{}".to_string()
            } else {
                arguments_str
            };

            match safely_parse_json(&arguments_str) {
                Ok(params) => {
                    let tool_call_param = CallToolRequestParam {
                        name: function_name.into(),
                        arguments: Some(object(params)),
                    };
                    let tool_request =
                        MessageContent::tool_request(tool_call.id.clone(), Ok(tool_call_param));
                    contents.push(tool_request);
                }
                Err(e) => {
                    let error = ErrorData {
                        code: ErrorCode::INVALID_PARAMS,
                        message: Cow::from(format!(
                            "Could not interpret tool use parameters for id {}: {}. Raw arguments: '{}'",
                            tool_call.id, e, arguments_str
                        )),
                        data: None,
                    };
                    contents.push(MessageContent::tool_request(
                        tool_call.id.clone(),
                        Err(error),
                    ));
                }
            }
        }
    }

    // Handle refusal
    if let Some(refusal) = &message.refusal {
        if !refusal.trim().is_empty() {
            contents.push(MessageContent::text(refusal));
        }
    }

    // If no content at all, return an error
    if contents.is_empty() {
        tracing::error!("SAP AI Core response validation failed: no content, tool calls, or refusal found. Message ID: {}, Model: {}",
            response.final_result.id, response.final_result.model);
        return Err(anyhow!(
            "No message content, tool calls, or refusal in SAP AI Core final result"
        ));
    }

    // Create the assistant message
    let mut msg = Message::new(Role::Assistant, chrono::Utc::now().timestamp(), contents);

    // Add ID
    msg = msg.with_id(response.final_result.id.clone());

    Ok(msg)
}

fn strip_data_prefix(line: &str) -> Option<&str> {
    line.strip_prefix("data: ").map(|s| s.trim())
}

/// Parse SAP AI Core streaming response
/// Note: This is a simplified implementation - SAP AI Core may have different streaming format
#[allow(clippy::too_many_lines)]
pub fn response_to_streaming_message<S>(
    mut _stream: S,
) -> impl Stream<Item = anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> + 'static
where
    S: Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    try_stream! {
        use futures::StreamExt;

        'outer: while let Some(response) = _stream.next().await {
            if response.as_ref().is_ok_and(|s| s == "data: [DONE]") {
                break 'outer;
            }

            let response_str = response?;
            let line = strip_data_prefix(&response_str);

            if line.is_none() || line.is_some_and(|l| l.is_empty()) {
                continue
            }

            let line_str = line.ok_or_else(|| {
                tracing::error!("SAP AI Core streaming: unexpected stream format, no data prefix found");
                anyhow!("unexpected stream format")
            })?;
            let chunk: CompletionPostStreamingRes = serde_json::from_str(line_str)
                .map_err(|e| {
                    tracing::error!("SAP AI Core streaming: failed to parse JSON chunk: {} - raw data: {}", e, line_str);
                    anyhow!("Failed to parse streaming chunk: {}: {:?}", e, &line_str)
                })?;

            if chunk.final_result.is_none() {
                match serde_json::from_str::<ErrorStreamingResponse>(line_str) {
                    Ok(error) => {
                        tracing::error!("SAP AI Core streaming: error: {:?}", error);
                        Err(anyhow!("The chunk error: {}, at: {}, code: {}", error.error.message, error.error.location, error.error.code))
                    },
                    Err(_) => {
                        tracing::info!("This chunk is not an error.");
                        Ok(())
                    }
                }?;
            }

            let usage = get_streaming_response_usage(&chunk).ok();

            if chunk.final_result.is_none() {
                tracing::error!("SAP AI Core streaming: chunk has no final_result");
                yield (None, usage);
                continue;
            }

            let final_result = chunk.final_result.as_ref().unwrap();
            if final_result.choices.is_empty() {
                tracing::error!("SAP AI Core streaming: final_result has no choices. Model: {}, ID: {}",
                    final_result.model, final_result.id);
                yield (None, usage);
                continue;
            }

            let first_result = &final_result.choices[0];
            if first_result.delta.content.is_empty()
                && (first_result.delta.tool_calls.is_none() || first_result.delta.tool_calls.as_ref().unwrap().is_empty()) {
                tracing::info!("SAP AI Core streaming: chunk has no content and no tool calls, yielding usage only");
                yield (None, usage);
                continue;
            }

            let choice = &final_result.choices[0];

            // Handle tool calls first
            if let Some(tool_calls) = &choice.delta.tool_calls {
                let mut tool_call_data: HashMap<i32, (String, String, String)> = HashMap::new();

                for tool_call in tool_calls {
                    if let Some(function) = &tool_call.function {
                        if let Some(id) = &tool_call.id {
                            let function_name = function.name.as_ref().cloned().unwrap_or_default();
                            let function_args = function.arguments.as_ref().cloned().unwrap_or_default();
                            tool_call_data.insert(tool_call.index as i32, (id.clone(), function_name, function_args));
                        } else {
                            tracing::error!("SAP AI Core streaming: tool call missing id");
                        }
                    } else {
                        tracing::error!("SAP AI Core streaming: tool call missing function");
                    }
                }

                // Check if this chunk already has finish_reason "tool_calls"
                let is_complete = final_result.choices[0].finish_reason == Some("tool_calls".to_string());

                if !is_complete {
                    let mut done = false;
                    while !done {
                        if let Some(response_chunk) = _stream.next().await {
                            if response_chunk.as_ref().is_ok_and(|s| s == "data: [DONE]") {
                                tracing::info!("SAP AI Core streaming: received [DONE] while collecting tool calls");
                                break 'outer;
                            }
                            let response_str = response_chunk?;
                            if let Some(line) = strip_data_prefix(&response_str) {
                                let tool_chunk: CompletionPostStreamingRes = serde_json::from_str(line)
                                    .map_err(|e| {
                                        tracing::error!("SAP AI Core streaming: failed to parse tool chunk: {} - raw: {}", e, line);
                                        anyhow!("Failed to parse streaming chunk: {}: {:?}", e, &line)
                                    })?;

                                if let Some(tool_final_result) = &tool_chunk.final_result {
                                    if let Some(delta_tool_calls) = &tool_final_result.choices[0].delta.tool_calls {
                                        for delta_call in delta_tool_calls {
                                            let index = delta_call.index as i32;
                                            if let Some(function) = &delta_call.function {
                                                let function_args = function.arguments.as_ref().cloned().unwrap_or_default();
                                                if let Some((_, _, ref mut args)) = tool_call_data.get_mut(&index) {
                                                    args.push_str(&function_args);
                                                } else if let Some(id) = &delta_call.id {
                                                    let function_name = function.name.as_ref().cloned().unwrap_or_default();
                                                    tool_call_data.insert(index, (id.clone(), function_name, function_args));
                                                }
                                            }
                                        }
                                    } else {
                                        match serde_json::from_str::<ErrorStreamingResponse>(line) {
                                            Ok(error) => {
                                                tracing::error!("SAP AI Core streaming: error: {:?}", error);
                                                Err(anyhow!("The chunk error: {}, at: {}, code: {}", error.error.message, error.error.location, error.error.code))
                                            },
                                            Err(_) => {
                                                tracing::info!("This chunk is not an error.");
                                                Ok(())
                                            }
                                        }?;
                                        tracing::info!("SAP AI Core streaming: no more delta tool calls, finishing");
                                        done = true;
                                    }

                                    if tool_final_result.choices[0].finish_reason == Some("tool_calls".to_string()) {
                                        tracing::info!("SAP AI Core streaming: tool calls finished with finish_reason");
                                        done = true;
                                    }
                                } else {
                                    tracing::error!("SAP AI Core streaming: tool chunk has no final_result");
                                }
                            }
                        } else {
                            tracing::error!("SAP AI Core streaming: stream ended unexpectedly while collecting tool calls. This may indicate a connection issue or incomplete response.");
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
                            safely_parse_json(arguments)
                        };

                        let content = match parsed {
                            Ok(params) => {
                                match convert_nested_json(params) {
                                    Ok(converted) => {
                                        let tool_call_param = CallToolRequestParam {
                                            name: function_name.clone().into(),
                                            arguments: Some(object(converted)),
                                        };
                                        MessageContent::tool_request(
                                            id.clone(),
                                            Ok(tool_call_param),
                                        )
                                    }
                                    Err(conversion_error) => {
                                        tracing::error!("SAP AI Core streaming: failed to convert nested JSON for tool call {}: {}. Raw arguments: {}",
                                            id, conversion_error, arguments);
                                        let error = ErrorData {
                                            code: ErrorCode::INVALID_PARAMS,
                                            message: Cow::from(format!(
                                                "Could not convert nested JSON for tool use parameters for id {}: {}",
                                                id, conversion_error
                                            )),
                                            data: None,
                                        };
                                        MessageContent::tool_request(id.clone(), Err(error))
                                    }
                                }
                            },
                            Err(e) => {
                                tracing::error!("SAP AI Core streaming: failed to parse tool arguments for {}: {} - raw: {}",
                                    id, e, arguments);
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

                // Add ID
                msg = msg.with_id(final_result.id.clone());

                yield (Some(msg), usage)
            } else {
                // Handle regular content (text/refusal)
                let delta = &choice.delta;
                let mut contents = Vec::new();

                // Handle content
                if !delta.content.is_empty() {
                    contents.push(MessageContent::text(&delta.content));
                }

                // Handle refusal
                if let Some(refusal) = &delta.refusal {
                    if !refusal.trim().is_empty() {
                        contents.push(MessageContent::text(refusal));
                    }
                }

                // Always yield content chunks, even if empty (for progress indication)
                if !contents.is_empty() {
                    let mut msg = Message::new(
                        Role::Assistant,
                        chrono::Utc::now().timestamp(),
                        contents,
                    );

                    // Add ID
                    msg = msg.with_id(final_result.id.clone());

                    yield (
                        Some(msg),
                        if choice.finish_reason.is_some() {
                            usage
                        } else {
                            None
                        },
                    )
                } else {
                    // Yield usage-only chunks for progress tracking
                    if usage.is_some() {
                        yield (None, usage)
                    }
                }
            }
        }
    }
}

/// Convert nested JSON strings to proper JSON values recursively
/// This function takes a JSON value and recursively parses any string values that contain valid JSON,
/// converting them into proper JSON structures (objects, arrays, etc.)
pub fn convert_nested_json(mut params: serde_json::Value) -> Result<serde_json::Value> {
    match &mut params {
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                *v = convert_nested_json(v.take())?;
            }
        }
        serde_json::Value::Array(array) => {
            for v in array.iter_mut() {
                *v = convert_nested_json(v.take())?;
            }
        }
        serde_json::Value::String(string) => {
            // Try to parse the string as JSON
            match serde_json::from_str::<serde_json::Value>(string) {
                Ok(parsed) => {
                    tracing::info!("Successfully parsed nested JSON string: {}", string);
                    // Recursively convert the parsed value in case it contains more nested JSON strings
                    return convert_nested_json(parsed);
                }
                Err(e) => {
                    tracing::info!(
                        "String '{}' is not valid JSON ({}), keeping as string",
                        string.chars().take(100).collect::<String>(),
                        e
                    );
                    // If parsing fails, keep the original string
                }
            }
        }
        _ => {
            // For other types (Number, Bool, Null), no conversion needed
        }
    }
    Ok(params)
}

/// Extract usage information from SAP AI Core response
pub fn get_usage(response_body: &str) -> Result<ProviderUsage> {
    let response: CompletionPostRes = serde_json::from_str(response_body).map_err(|e| {
        tracing::error!("Failed to parse SAP AI Core response for usage extraction: {}. Response body (first 200 chars): {}",
            e,
            response_body.chars().take(200).collect::<String>());
        anyhow!("Failed to parse SAP AI Core response for usage: {}", e)
    })?;

    get_response_usage(&response)
}

pub fn get_response_usage(response: &CompletionPostRes) -> Result<ProviderUsage> {
    // Use final_result usage for token counts
    let usage = &response.final_result.usage;
    Ok(ProviderUsage::new(
        response.final_result.model.clone(),
        Usage {
            input_tokens: Some(usage.prompt_tokens as i32),
            output_tokens: Some(usage.completion_tokens as i32),
            total_tokens: Some(usage.total_tokens as i32),
        },
    ))
}

pub fn get_streaming_response_usage(
    response: &CompletionPostStreamingRes,
) -> Result<ProviderUsage> {
    if let Some(final_result) = &response.final_result {
        if let Some(usage) = &final_result.usage {
            Ok(ProviderUsage::new(
                final_result.model.clone(),
                Usage {
                    input_tokens: Some(usage.prompt_tokens as i32),
                    output_tokens: Some(usage.completion_tokens as i32),
                    total_tokens: Some(usage.total_tokens as i32),
                },
            ))
        } else {
            Ok(ProviderUsage::new(
                final_result.model.clone(),
                Usage {
                    input_tokens: Some(0),
                    output_tokens: Some(0),
                    total_tokens: Some(0),
                },
            ))
        }
    } else {
        Err(anyhow!("No final result in streaming response"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::MessageContent;
    use rmcp::model::CallToolResult;

    #[test]
    fn test_create_request() {
        let messages = vec![
            Message::user().with_text("Hello, how are you?"),
            Message::assistant().with_text("I'm doing well, thank you!"),
        ];

        let model_config = ModelConfig::new_or_fail("anthropic--claude-4-sonnet");

        let tools = vec![];
        let result = create_request(
            &messages,
            &tools,
            &model_config,
            "You are a helpful assistant",
            false,
        );

        assert!(result.is_ok());
        let request_value = result.unwrap();

        // Verify the structure contains config
        assert!(request_value.get("config").is_some());
        let config = request_value.get("config").unwrap();

        // Verify modules exists
        assert!(config.get("modules").is_some());
        let modules = config.get("modules").unwrap();

        // Verify prompt_templating
        assert!(modules.get("prompt_templating").is_some());
        let templating_config = modules.get("prompt_templating").unwrap();
        assert!(templating_config.get("model").is_some());
        assert!(templating_config.get("prompt").is_some());
        let prompt = templating_config.get("prompt").unwrap();
        let template = prompt.get("template").unwrap().as_array().unwrap();
        assert_eq!(template.len(), 3); // system + user + assistant
    }

    #[test]
    fn test_response_to_message() {
        let response_json = json!({
            "request_id": "12eb1168-9316-9e1c-81ab-691780907486",
            "intermediate_results": {
                "templating": [
                    {
                        "role": "system",
                        "content": "You are a software senior engineer"
                    },
                    {
                        "content": "How to implement OAuth2 server with java",
                        "role": "user"
                    }
                ]
            },
            "final_result": {
                "id": "msg_bdrk_012eCWSd2UH16PFSBG12Vm7j",
                "object": "chat.completion",
                "created": 1757129945,
                "model": "claude-sonnet-4-20250514",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "Hello! How can I help you today?"
                        },
                        "finish_reason": "stop"
                    }
                ],
                "usage": {
                    "completion_tokens": 3575,
                    "prompt_tokens": 39,
                    "total_tokens": 3614
                }
            }
        });

        let result = response_to_message(&response_json.to_string());
        assert!(result.is_ok());

        let message = result.unwrap();
        assert_eq!(message.role, Role::Assistant);

        if let Some(MessageContent::Text(text)) = message.content.first() {
            assert_eq!(text.text, "Hello! How can I help you today?");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_create_request_with_tool_messages() {
        use rmcp::model::Content;

        // Create messages with tool interactions
        let messages = vec![
            Message::user().with_text("What's the weather like?"),
            Message::assistant().with_tool_request(
                "call_123",
                Ok(CallToolRequestParam {
                    name: "get_weather".into(),
                    arguments: Some(object(json!({"location": "New York"}))),
                }),
            ),
            Message::user().with_tool_response(
                "call_123",
                Ok(CallToolResult {
                    content: vec![Content::text("The weather in New York is sunny, 75°F")],
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }),
            ),
            Message::assistant().with_text("The weather in New York is sunny and 75°F today!"),
        ];

        let model_config = ModelConfig::new_or_fail("gpt-4");
        let tools = vec![];
        let result = create_request(
            &messages,
            &tools,
            &model_config,
            "You are a helpful assistant",
            false,
        );

        assert!(result.is_ok());
        let request_value = result.unwrap();

        // Verify the structure
        assert!(request_value.get("config").is_some());
        let config = request_value.get("config").unwrap();
        assert!(config.get("modules").is_some());
        let modules = config.get("modules").unwrap();
        assert!(modules.get("prompt_templating").is_some());
        let templating_config = modules.get("prompt_templating").unwrap();
        assert!(templating_config.get("prompt").is_some());
        let prompt = templating_config.get("prompt").unwrap();
        assert!(prompt.get("template").is_some());

        let template = prompt.get("template").unwrap().as_array().unwrap();

        // Should have: system + user + assistant_with_tool_call + tool_response + assistant_text
        assert_eq!(template.len(), 5);

        // Check the assistant message with tool call
        let assistant_msg = &template[2];
        assert_eq!(
            assistant_msg.get("role").unwrap().as_str().unwrap(),
            "assistant"
        );
        assert!(assistant_msg.get("tool_calls").is_some());
        let tool_calls = assistant_msg.get("tool_calls").unwrap().as_array().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(
            tool_calls[0].get("id").unwrap().as_str().unwrap(),
            "call_123"
        );
        assert_eq!(
            tool_calls[0]
                .get("function")
                .unwrap()
                .get("name")
                .unwrap()
                .as_str()
                .unwrap(),
            "get_weather"
        );

        // Check the tool response message
        let tool_msg = &template[3];
        assert_eq!(tool_msg.get("role").unwrap().as_str().unwrap(), "tool");
        assert_eq!(
            tool_msg.get("tool_call_id").unwrap().as_str().unwrap(),
            "call_123"
        );
        assert!(tool_msg
            .get("content")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("sunny"));

        // Check the final assistant message
        let final_msg = &template[4];
        assert_eq!(
            final_msg.get("role").unwrap().as_str().unwrap(),
            "assistant"
        );
        assert!(final_msg
            .get("content")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("75°F"));
    }

    #[test]
    fn test_response_to_message_with_tool_calls() {
        let response_json = json!({
            "request_id": "12eb1168-9316-9e1c-81ab-691780907486",
            "intermediate_results": {
                "templating": []
            },
            "final_result": {
                "id": "msg_tool_call_test",
                "object": "chat.completion",
                "created": 1757129945,
                "model": "claude-sonnet-4-20250514",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "I'll check the weather for you.",
                            "tool_calls": [
                                {
                                    "id": "call_123",
                                    "type": "function",
                                    "function": {
                                        "name": "get_weather",
                                        "arguments": "{\"location\": \"New York\"}"
                                    }
                                }
                            ]
                        },
                        "finish_reason": "tool_calls"
                    }
                ],
                "usage": {
                    "completion_tokens": 50,
                    "prompt_tokens": 20,
                    "total_tokens": 70
                }
            }
        });

        let result = response_to_message(&response_json.to_string());
        assert!(result.is_ok());

        let message = result.unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.id, Some("msg_tool_call_test".to_string()));

        // Should have both text content and tool request
        assert_eq!(message.content.len(), 2);

        // Check text content
        if let Some(MessageContent::Text(text)) = message.content.first() {
            assert_eq!(text.text, "I'll check the weather for you.");
        } else {
            panic!("Expected text content as first item");
        }

        // Check tool request
        if let Some(MessageContent::ToolRequest(tool_request)) = message.content.get(1) {
            assert_eq!(tool_request.id, "call_123");
            if let Ok(tool_call) = &tool_request.tool_call {
                assert_eq!(tool_call.name, "get_weather");
                assert_eq!(
                    tool_call
                        .arguments
                        .as_ref()
                        .unwrap()
                        .get("location")
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    "New York"
                );
            } else {
                panic!("Expected valid tool call");
            }
        } else {
            panic!("Expected tool request as second item");
        }
    }

    #[test]
    fn test_response_to_message_with_refusal() {
        let response_json = json!({
            "request_id": "12eb1168-9316-9e1c-81ab-691780907486",
            "intermediate_results": {
                "templating": []
            },
            "final_result": {
                "id": "msg_refusal_test",
                "object": "chat.completion",
                "created": 1757129945,
                "model": "claude-sonnet-4-20250514",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "refusal": "I cannot help with that request."
                        },
                        "finish_reason": "stop"
                    }
                ],
                "usage": {
                    "completion_tokens": 10,
                    "prompt_tokens": 15,
                    "total_tokens": 25
                }
            }
        });

        let result = response_to_message(&response_json.to_string());
        assert!(result.is_ok());

        let message = result.unwrap();
        assert_eq!(message.role, Role::Assistant);

        if let Some(MessageContent::Text(text)) = message.content.first() {
            assert_eq!(text.text, "I cannot help with that request.");
        } else {
            panic!("Expected text content from refusal");
        }
    }

    #[tokio::test]
    async fn test_streaming_text_response() {
        use futures::stream;

        let streaming_chunks = vec![
            Ok("data: {\"request_id\":\"test\",\"final_result\":{\"id\":\"msg_streaming\",\"object\":\"chat.completion\",\"created\":1234567890,\"model\":\"test-model\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\",\"role\":\"assistant\"},\"finish_reason\":null}],\"usage\":null}}".to_string()),
            Ok("data: {\"request_id\":\"test\",\"final_result\":{\"id\":\"msg_streaming\",\"object\":\"chat.completion\",\"created\":1234567890,\"model\":\"test-model\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world!\",\"role\":null},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":15,\"total_tokens\":25}}}".to_string()),
            Ok("data: [DONE]".to_string()),
        ];

        let mock_stream = stream::iter(streaming_chunks);
        let result_stream = response_to_streaming_message(mock_stream);

        use futures::StreamExt;

        // First chunk should have "Hello"
        let mut result_stream = Box::pin(result_stream);
        let first_result = result_stream.next().await.unwrap().unwrap();
        assert!(first_result.0.is_some());
        let message = first_result.0.unwrap();
        if let Some(MessageContent::Text(text)) = message.content.first() {
            assert_eq!(text.text, "Hello");
        } else {
            panic!("Expected text content");
        }

        // Second chunk should have " world!" and usage
        let second_result = result_stream.next().await.unwrap().unwrap();
        assert!(second_result.0.is_some());
        assert!(second_result.1.is_some()); // Should have usage because of finish_reason
        let message = second_result.0.unwrap();
        if let Some(MessageContent::Text(text)) = message.content.first() {
            assert_eq!(text.text, " world!");
        } else {
            panic!("Expected text content");
        }

        // Stream should be done
        assert!(result_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_streaming_tool_call_response() {
        use futures::stream;

        let streaming_chunks = vec![
            Ok("data: {\"request_id\":\"test\",\"final_result\":{\"id\":\"msg_tool_streaming\",\"object\":\"chat.completion\",\"created\":1234567890,\"model\":\"test-model\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\",\"tool_calls\":[{\"index\":0,\"id\":\"call_123\",\"type\":\"function\",\"function\":{\"name\":\"get_weather\",\"arguments\":\"{\\\"location\\\":\\\"New York\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":20,\"completion_tokens\":5,\"total_tokens\":25}}}".to_string()),
            Ok("data: [DONE]".to_string()),
        ];

        let mock_stream = stream::iter(streaming_chunks);
        let mut result_stream = Box::pin(response_to_streaming_message(mock_stream));

        use futures::StreamExt;

        // Should get tool call message
        let result = result_stream.next().await.unwrap().unwrap();
        assert!(result.0.is_some());
        assert!(result.1.is_some()); // Should have usage

        let message = result.0.unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content.len(), 1);

        if let Some(MessageContent::ToolRequest(tool_request)) = message.content.first() {
            assert_eq!(tool_request.id, "call_123");
            if let Ok(tool_call) = &tool_request.tool_call {
                assert_eq!(tool_call.name, "get_weather");
                // Arguments should be parsed as JSON - it contains partial JSON so may not parse fully
                // Just verify the tool call succeeded
            } else {
                panic!(
                    "Expected valid tool call, got: {:?}",
                    tool_request.tool_call
                );
            }
        } else {
            panic!("Expected tool request");
        }

        // Stream should be done
        assert!(result_stream.next().await.is_none());
    }

    #[test]
    fn test_payload_structure() {
        let messages = vec![Message::user().with_text("Hello, how are you?")];

        let model_config = ModelConfig::new_or_fail("test-model");
        let tools = vec![];
        let result = create_request(
            &messages,
            &tools,
            &model_config,
            "You are a helpful assistant",
            false,
        );

        assert!(result.is_ok());
        let request_value = result.unwrap();

        println!("Generated SAP AI Core payload structure:");
        println!("{}", serde_json::to_string_pretty(&request_value).unwrap());
    }

    // Tests for convert_nested_json function
    #[test]
    fn test_convert_nested_json_simple_object() {
        let input = json!({
            "name": "John",
            "age": 30,
            "config": "{\"theme\": \"dark\", \"notifications\": true}"
        });

        let result = convert_nested_json(input).unwrap();

        assert_eq!(result["name"], "John");
        assert_eq!(result["age"], 30);
        assert!(result["config"].is_object());
        assert_eq!(result["config"]["theme"], "dark");
        assert_eq!(result["config"]["notifications"], true);
    }

    #[test]
    fn test_convert_nested_json_nested_objects() {
        let input = json!({
            "user": {
                "name": "Alice",
                "preferences": "{\"language\": \"en\", \"settings\": \"{\\\"volume\\\": 80}\"}"
            },
            "metadata": "{\"version\": \"v1.0\", \"author\": \"test\"}"
        });

        let result = convert_nested_json(input).unwrap();

        assert_eq!(result["user"]["name"], "Alice");
        assert!(result["user"]["preferences"].is_object());
        assert_eq!(result["user"]["preferences"]["language"], "en");
        // The settings string contains escaped quotes, so it becomes a nested JSON object
        assert!(result["user"]["preferences"]["settings"].is_object());
        // Volume might be parsed as a number or string depending on JSON parsing
        let volume = &result["user"]["preferences"]["settings"]["volume"];
        assert!(volume == &json!(80) || volume == &json!("80"));
        assert!(result["metadata"].is_object());
        assert_eq!(result["metadata"]["version"], "v1.0");
        assert_eq!(result["metadata"]["author"], "test");
    }

    #[test]
    fn test_convert_nested_json_arrays() {
        let input = json!({
            "items": [
                "simple_string",
                "{\"type\": \"object\", \"value\": 42}",
                123,
                true,
                null
            ],
            "nested_array": "[\"a\", \"b\", \"{\\\"nested\\\": true}\"]"
        });

        let result = convert_nested_json(input).unwrap();

        assert_eq!(result["items"][0], "simple_string");
        assert!(result["items"][1].is_object());
        assert_eq!(result["items"][1]["type"], "object");
        assert_eq!(result["items"][1]["value"], 42);
        assert_eq!(result["items"][2], 123);
        assert_eq!(result["items"][3], true);
        assert!(result["items"][4].is_null());

        assert!(result["nested_array"].is_array());
        assert_eq!(result["nested_array"][0], "a");
        assert_eq!(result["nested_array"][1], "b");
        assert!(result["nested_array"][2].is_object());
        assert_eq!(result["nested_array"][2]["nested"], true);
    }

    #[test]
    fn test_convert_nested_json_mixed_complex() {
        let input = json!({
            "config": {
                "database": "{\"host\": \"localhost\", \"port\": 5432, \"credentials\": \"{\\\"user\\\": \\\"admin\\\", \\\"pass\\\": \\\"secret\\\"}\"}",
                "features": ["auth", "{\"name\": \"logging\", \"level\": \"info\"}", "cache"]
            },
            "users": "[{\"id\": 1, \"profile\": \"{\\\"name\\\": \\\"John\\\", \\\"age\\\": 25}\"}, {\"id\": 2, \"profile\": \"{\\\"name\\\": \\\"Jane\\\", \\\"age\\\": 30}\"}]"
        });

        let result = convert_nested_json(input).unwrap();

        // Check database config
        assert!(result["config"]["database"].is_object());
        assert_eq!(result["config"]["database"]["host"], "localhost");
        assert_eq!(result["config"]["database"]["port"], 5432);
        assert!(result["config"]["database"]["credentials"].is_object());
        assert_eq!(result["config"]["database"]["credentials"]["user"], "admin");
        assert_eq!(
            result["config"]["database"]["credentials"]["pass"],
            "secret"
        );

        // Check features array
        assert!(result["config"]["features"].is_array());
        assert_eq!(result["config"]["features"][0], "auth");
        assert!(result["config"]["features"][1].is_object());
        assert_eq!(result["config"]["features"][1]["name"], "logging");
        assert_eq!(result["config"]["features"][1]["level"], "info");
        assert_eq!(result["config"]["features"][2], "cache");

        // Check users array
        assert!(result["users"].is_array());
        assert_eq!(result["users"].as_array().unwrap().len(), 2);
        assert_eq!(result["users"][0]["id"], 1);
        assert!(result["users"][0]["profile"].is_object());
        assert_eq!(result["users"][0]["profile"]["name"], "John");
        assert_eq!(result["users"][0]["profile"]["age"], 25);
        assert_eq!(result["users"][1]["id"], 2);
        assert!(result["users"][1]["profile"].is_object());
        assert_eq!(result["users"][1]["profile"]["name"], "Jane");
        assert_eq!(result["users"][1]["profile"]["age"], 30);
    }

    #[test]
    fn test_convert_nested_json_invalid_json_strings() {
        let input = json!({
            "valid": "{\"key\": \"value\"}",
            "invalid": "{invalid json}",
            "empty": "",
            "not_json": "just a regular string",
            "partial": "{\"incomplete\": "
        });

        let result = convert_nested_json(input).unwrap();

        // Valid JSON should be parsed
        assert!(result["valid"].is_object());
        assert_eq!(result["valid"]["key"], "value");

        // Invalid JSON should remain as strings
        assert_eq!(result["invalid"], "{invalid json}");
        assert_eq!(result["empty"], "");
        assert_eq!(result["not_json"], "just a regular string");
        assert_eq!(result["partial"], "{\"incomplete\": ");
    }

    #[test]
    fn test_convert_nested_json_primitive_types() {
        let input = json!({
            "string": "hello",
            "number": 42,
            "float": 3.15,
            "boolean": true,
            "null_value": null,
            "json_string": "{\"parsed\": true}"
        });

        let result = convert_nested_json(input).unwrap();

        // Primitive types should remain unchanged
        assert_eq!(result["string"], "hello");
        assert_eq!(result["number"], 42);
        assert_eq!(result["float"], 3.15);
        assert_eq!(result["boolean"], true);
        assert!(result["null_value"].is_null());

        // JSON string should be parsed
        assert!(result["json_string"].is_object());
        assert_eq!(result["json_string"]["parsed"], true);
    }

    #[test]
    fn test_convert_nested_json_empty_structures() {
        let input = json!({
            "empty_object": "{}",
            "empty_array": "[]",
            "nested_empty": "{\"obj\": {}, \"arr\": []}"
        });

        let result = convert_nested_json(input).unwrap();

        assert!(result["empty_object"].is_object());
        assert_eq!(result["empty_object"].as_object().unwrap().len(), 0);

        assert!(result["empty_array"].is_array());
        assert_eq!(result["empty_array"].as_array().unwrap().len(), 0);

        assert!(result["nested_empty"].is_object());
        assert!(result["nested_empty"]["obj"].is_object());
        assert_eq!(result["nested_empty"]["obj"].as_object().unwrap().len(), 0);
        assert!(result["nested_empty"]["arr"].is_array());
        assert_eq!(result["nested_empty"]["arr"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_convert_nested_json_deeply_nested() {
        let input = json!({
            "level1": "{\"level2\": \"{\\\"level3\\\": \\\"{\\\\\\\"level4\\\\\\\": \\\\\\\"deep\\\\\\\"}\\\"}\"}"
        });

        let result = convert_nested_json(input).unwrap();

        assert!(result["level1"].is_object());
        assert!(result["level1"]["level2"].is_object());
        assert!(result["level1"]["level2"]["level3"].is_object());
        assert_eq!(result["level1"]["level2"]["level3"]["level4"], "deep");
    }

    #[test]
    fn test_convert_nested_json_special_characters() {
        let input = json!({
            "unicode": "{\"emoji\": \"🚀\", \"chinese\": \"你好\"}",
            "escaped": "{\"quote\": \"He said \\\"hello\\\"\", \"newline\": \"line1\\nline2\"}"
        });

        let result = convert_nested_json(input).unwrap();

        assert!(result["unicode"].is_object());
        assert_eq!(result["unicode"]["emoji"], "🚀");
        assert_eq!(result["unicode"]["chinese"], "你好");

        assert!(result["escaped"].is_object());
        assert_eq!(result["escaped"]["quote"], "He said \"hello\"");
        assert_eq!(result["escaped"]["newline"], "line1\nline2");
    }

    #[test]
    fn test_convert_nested_json_tool_call_scenario() {
        // Test a realistic scenario similar to what would be encountered in tool calls
        let input = json!({
            "function_name": "search_files",
            "arguments": "{\"path\": \"/home/user\", \"pattern\": \"*.rs\", \"options\": \"{\\\"recursive\\\": true, \\\"case_sensitive\\\": false}\"}"
        });

        let result = convert_nested_json(input).unwrap();

        assert_eq!(result["function_name"], "search_files");
        assert!(result["arguments"].is_object());
        assert_eq!(result["arguments"]["path"], "/home/user");
        assert_eq!(result["arguments"]["pattern"], "*.rs");
        assert!(result["arguments"]["options"].is_object());
        assert_eq!(result["arguments"]["options"]["recursive"], true);
        assert_eq!(result["arguments"]["options"]["case_sensitive"], false);
    }

    #[test]
    fn test_convert_nested_json_preserves_non_string_types() {
        let input = json!([
            42,
            true,
            null,
            3.15,
            "{\"converted\": true}",
            ["nested", "{\"also\": \"converted\"}"]
        ]);

        let result = convert_nested_json(input).unwrap();

        assert!(result.is_array());
        assert_eq!(result[0], 42);
        assert_eq!(result[1], true);
        assert!(result[2].is_null());
        assert_eq!(result[3], 3.15);
        assert!(result[4].is_object());
        assert_eq!(result[4]["converted"], true);
        assert!(result[5].is_array());
        assert_eq!(result[5][0], "nested");
        assert!(result[5][1].is_object());
        assert_eq!(result[5][1]["also"], "converted");
    }

    #[test]
    fn test_convert_tools_to_sap_format() {
        let tools = vec![
            Tool::new(
                "get_weather",
                "Get weather for a location",
                object(json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "City name"
                        }
                    },
                    "required": ["location"]
                })),
            ),
            Tool::new(
                "calculate",
                "",
                object(json!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string"
                        }
                    }
                })),
            ),
        ];

        let result = convert_tools_to_sap_format(&tools);
        assert!(result.is_ok());

        let sap_tools = result.unwrap();
        assert_eq!(sap_tools.len(), 2);

        // Check first tool
        assert_eq!(sap_tools[0].tool_type, "function");
        assert_eq!(sap_tools[0].function.name.as_ref().unwrap(), "get_weather");
        assert_eq!(
            sap_tools[0].function.description.as_ref().unwrap(),
            "Get weather for a location"
        );
        assert!(sap_tools[0].function.parameters.is_some());
        let params = sap_tools[0].function.parameters.as_ref().unwrap();
        assert_eq!(params["type"], "object");
        assert!(params["properties"].is_object());

        // Check second tool
        assert_eq!(sap_tools[1].function.name.as_ref().unwrap(), "calculate");
    }

    #[test]
    fn test_convert_tools_to_sap_format_empty() {
        let tools = vec![];
        let result = convert_tools_to_sap_format(&tools);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_convert_tools_to_sap_format_minimal_schema() {
        let tools = vec![Tool::new("simple_tool", "A simple tool", object(json!({})))];

        let result = convert_tools_to_sap_format(&tools);
        assert!(result.is_ok());

        let sap_tools = result.unwrap();
        assert_eq!(sap_tools.len(), 1);

        // Should have type and properties added
        let params = sap_tools[0].function.parameters.as_ref().unwrap();
        assert_eq!(params["type"], "object");
        assert!(params["properties"].is_object());
    }

    #[test]
    fn test_strip_data_prefix() {
        assert_eq!(strip_data_prefix("data: hello"), Some("hello"));
        assert_eq!(strip_data_prefix("data:    world  "), Some("world"));
        assert_eq!(strip_data_prefix("data: "), Some(""));
        assert_eq!(strip_data_prefix("no prefix"), None);
        assert_eq!(strip_data_prefix("data:"), None); // "data:" without space doesn't match "data: "
        assert_eq!(strip_data_prefix(""), None);
    }

    #[test]
    fn test_get_usage() {
        let response_json = json!({
            "request_id": "12eb1168-9316-9e1c-81ab-691780907486",
            "intermediate_results": {},
            "final_result": {
                "id": "msg_test",
                "object": "chat.completion",
                "created": 1757129945,
                "model": "claude-sonnet-4-20250514",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "Hello!"
                        },
                        "finish_reason": "stop"
                    }
                ],
                "usage": {
                    "completion_tokens": 100,
                    "prompt_tokens": 50,
                    "total_tokens": 150
                }
            }
        });

        let result = get_usage(&response_json.to_string());
        assert!(result.is_ok());

        let usage = result.unwrap();
        assert_eq!(usage.model, "claude-sonnet-4-20250514");
        assert_eq!(usage.usage.input_tokens, Some(50));
        assert_eq!(usage.usage.output_tokens, Some(100));
        assert_eq!(usage.usage.total_tokens, Some(150));
    }

    #[test]
    fn test_get_usage_invalid_json() {
        let result = get_usage("invalid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_request_empty_messages() {
        let messages = vec![];
        let model_config = ModelConfig::new_or_fail("test-model");
        let tools = vec![];
        let result = create_request(&messages, &tools, &model_config, "", false);

        assert!(result.is_ok());
        let request_value = result.unwrap();
        let template = request_value["config"]["modules"]["prompt_templating"]["prompt"]
            ["template"]
            .as_array()
            .unwrap();
        // Should have no messages
        assert_eq!(template.len(), 0);
    }

    #[test]
    fn test_create_request_with_tools() {
        let messages = vec![Message::user().with_text("Use tools")];

        let tools = vec![Tool::new(
            "test_tool",
            "Test tool",
            object(json!({
                "type": "object",
                "properties": {
                    "param": {"type": "string"}
                }
            })),
        )];

        let model_config = ModelConfig::new_or_fail("test-model");
        let result = create_request(&messages, &tools, &model_config, "System", true);

        assert!(result.is_ok());
        let request_value = result.unwrap();

        // Check streaming is enabled
        assert_eq!(request_value["config"]["stream"]["enabled"], true);

        // Check tools are included
        let prompt = &request_value["config"]["modules"]["prompt_templating"]["prompt"];
        assert!(prompt["tools"].is_array());
        let tools_array = prompt["tools"].as_array().unwrap();
        assert_eq!(tools_array.len(), 1);
        assert_eq!(tools_array[0]["function"]["name"], "test_tool");
    }

    #[test]
    fn test_response_to_message_empty_choices() {
        let response_json = json!({
            "request_id": "test",
            "final_result": {
                "id": "msg_test",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "test-model",
                "choices": [],
                "usage": {
                    "completion_tokens": 0,
                    "prompt_tokens": 0,
                    "total_tokens": 0
                }
            }
        });

        let result = response_to_message(&response_json.to_string());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("choices") || error_msg.contains("SAP AI Core"),
            "Error message was: {}",
            error_msg
        );
    }

    #[test]
    fn test_response_to_message_empty_content() {
        let response_json = json!({
            "request_id": "test",
            "final_result": {
                "id": "msg_test",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "test-model",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": ""
                        },
                        "finish_reason": "stop"
                    }
                ],
                "usage": {
                    "completion_tokens": 0,
                    "prompt_tokens": 10,
                    "total_tokens": 10
                }
            }
        });

        let result = response_to_message(&response_json.to_string());
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("content") || error_msg.contains("SAP AI Core"),
            "Error message was: {}",
            error_msg
        );
    }

    #[test]
    fn test_response_to_message_invalid_tool_name() {
        let response_json = json!({
            "request_id": "test",
            "final_result": {
                "id": "msg_test",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "test-model",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "Using tool",
                            "tool_calls": [
                                {
                                    "id": "call_123",
                                    "type": "function",
                                    "function": {
                                        "name": "invalid tool name",
                                        "arguments": "{}"
                                    }
                                }
                            ]
                        },
                        "finish_reason": "tool_calls"
                    }
                ],
                "usage": {
                    "completion_tokens": 10,
                    "prompt_tokens": 10,
                    "total_tokens": 20
                }
            }
        });

        let result = response_to_message(&response_json.to_string());

        // The function may fail due to invalid tool name at parsing level
        // or succeed and include an error in the tool request
        match result {
            Ok(message) => {
                // If it succeeds, should have tool request with error
                let has_error = message.content.iter().any(|c| {
                    if let MessageContent::ToolRequest(tr) = c {
                        tr.tool_call.is_err()
                    } else {
                        false
                    }
                });
                assert!(has_error, "Expected at least one tool request with error");
            }
            Err(_) => {
                // If it fails entirely, that's also acceptable for invalid tool name
            }
        }
    }

    #[test]
    fn test_response_to_message_invalid_tool_arguments() {
        let response_json = json!({
            "request_id": "test",
            "final_result": {
                "id": "msg_test",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "test-model",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "Using tool",
                            "tool_calls": [
                                {
                                    "id": "call_123",
                                    "type": "function",
                                    "function": {
                                        "name": "valid_tool",
                                        "arguments": "{invalid json"
                                    }
                                }
                            ]
                        },
                        "finish_reason": "tool_calls"
                    }
                ],
                "usage": {
                    "completion_tokens": 10,
                    "prompt_tokens": 10,
                    "total_tokens": 20
                }
            }
        });

        let result = response_to_message(&response_json.to_string());

        // The function may fail due to invalid JSON or succeed with error in tool request
        match result {
            Ok(message) => {
                // If it succeeds, should have tool request with error
                let has_error = message.content.iter().any(|c| {
                    if let MessageContent::ToolRequest(tr) = c {
                        tr.tool_call.is_err()
                    } else {
                        false
                    }
                });
                assert!(has_error, "Expected at least one tool request with error");
            }
            Err(_) => {
                // If it fails entirely due to invalid JSON, that's also acceptable
            }
        }
    }

    #[test]
    fn test_response_to_message_multiple_tool_calls() {
        let response_json = json!({
            "request_id": "test",
            "intermediate_results": {},
            "final_result": {
                "id": "msg_test",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "test-model",
                "choices": [
                    {
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "Using multiple tools",
                            "tool_calls": [
                                {
                                    "id": "call_1",
                                    "type": "function",
                                    "function": {
                                        "name": "tool_one",
                                        "arguments": "{\"param\": \"value1\"}"
                                    }
                                },
                                {
                                    "id": "call_2",
                                    "type": "function",
                                    "function": {
                                        "name": "tool_two",
                                        "arguments": "{\"param\": \"value2\"}"
                                    }
                                }
                            ]
                        },
                        "finish_reason": "tool_calls"
                    }
                ],
                "usage": {
                    "completion_tokens": 20,
                    "prompt_tokens": 15,
                    "total_tokens": 35
                }
            }
        });

        let result = response_to_message(&response_json.to_string());

        match result {
            Ok(message) => {
                // Should have text content + 2 tool requests
                assert!(
                    message.content.len() >= 2,
                    "Expected at least 2 content items"
                );

                let tool_requests: Vec<_> = message
                    .content
                    .iter()
                    .filter_map(|c| {
                        if let MessageContent::ToolRequest(tr) = c {
                            Some(tr)
                        } else {
                            None
                        }
                    })
                    .collect();

                assert_eq!(tool_requests.len(), 2, "Expected 2 tool requests");
                assert_eq!(tool_requests[0].id, "call_1");
                assert_eq!(tool_requests[1].id, "call_2");

                if let Ok(tool_call) = &tool_requests[0].tool_call {
                    assert_eq!(tool_call.name, "tool_one");
                }
                if let Ok(tool_call) = &tool_requests[1].tool_call {
                    assert_eq!(tool_call.name, "tool_two");
                }
            }
            Err(e) => {
                panic!("Expected successful parsing but got error: {}", e);
            }
        }
    }

    #[test]
    fn test_get_streaming_response_usage_no_final_result() {
        let chunk = CompletionPostStreamingRes {
            request_id: Some("test".to_string()),
            intermediate_results: None,
            final_result: None,
        };

        let result = get_streaming_response_usage(&chunk);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_streaming_response_usage_no_usage() {
        let chunk = CompletionPostStreamingRes {
            request_id: Some("test".to_string()),
            intermediate_results: None,
            final_result: Some(LlmModuleResultStreaming {
                id: "msg_test".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1234567890,
                model: "test-model".to_string(),
                system_fingerprint: None,
                choices: vec![],
                usage: None,
            }),
        };

        let result = get_streaming_response_usage(&chunk);
        assert!(result.is_ok());

        let usage = result.unwrap();
        assert_eq!(usage.usage.input_tokens, Some(0));
        assert_eq!(usage.usage.output_tokens, Some(0));
        assert_eq!(usage.usage.total_tokens, Some(0));
    }

    #[tokio::test]
    async fn test_streaming_empty_chunk() {
        use futures::stream;
        use futures::StreamExt;

        let streaming_chunks = vec![
            Ok("data: {\"request_id\":\"test\",\"final_result\":{\"id\":\"msg_test\",\"object\":\"chat.completion\",\"created\":1234567890,\"model\":\"test-model\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\",\"role\":\"assistant\"},\"finish_reason\":null}],\"usage\":null}}".to_string()),
            Ok("data: [DONE]".to_string()),
        ];

        let mock_stream = stream::iter(streaming_chunks);
        let mut result_stream = Box::pin(response_to_streaming_message(mock_stream));

        // Should yield None for empty content
        let result = result_stream.next().await.unwrap().unwrap();
        assert!(result.0.is_none());

        // Stream should be done
        assert!(result_stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_streaming_invalid_json() {
        use futures::stream;
        use futures::StreamExt;

        let streaming_chunks = vec![Ok("data: {invalid json}".to_string())];

        let mock_stream = stream::iter(streaming_chunks);
        let mut result_stream = Box::pin(response_to_streaming_message(mock_stream));

        // Should yield an error
        let result = result_stream.next().await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_streaming_no_data_prefix() {
        use futures::stream;
        use futures::StreamExt;

        let streaming_chunks = vec![
            Ok("no prefix here".to_string()),
            Ok("data: [DONE]".to_string()),
        ];

        let mock_stream = stream::iter(streaming_chunks);
        let mut result_stream = Box::pin(response_to_streaming_message(mock_stream));

        // Should skip the line without prefix and end
        assert!(result_stream.next().await.is_none());
    }
}
