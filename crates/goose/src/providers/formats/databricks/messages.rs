use super::*;

#[derive(Serialize)]
pub(super) struct DatabricksMessage {
    pub content: Value,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

pub(super) fn format_text_content(text: &str, image_format: &ImageFormat) -> (Vec<Value>, bool) {
    let mut items = vec![json!({"type": "text", "text": text})];
    let has_image = if let Some(path) = detect_image_path(text) {
        if let Ok(image) = load_image_file(path) {
            items.push(convert_image(&image, image_format));
        }
        true
    } else {
        false
    };
    (items, has_image)
}

pub(super) fn format_tool_response(
    response: &crate::conversation::message::ToolResponse,
    image_format: &ImageFormat,
) -> Vec<DatabricksMessage> {
    let mut result = Vec::new();

    match &response.tool_result {
        Ok(call_result) => {
            let abridged: Vec<_> = call_result.content.iter().map(|c| c.raw.clone()).collect();

            let mut tool_content = Vec::new();
            let mut image_messages = Vec::new();

            for content in abridged {
                match content {
                    RawContent::Image(image) => {
                        tool_content.push(Content::text(
                            "This tool result included an image that is uploaded in the next message.",
                        ));
                        image_messages.push(DatabricksMessage {
                            role: "user".to_string(),
                            content: [convert_image(&image.no_annotation(), image_format)].into(),
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }
                    RawContent::Resource(resource) => {
                        let text = match &resource.resource {
                            ResourceContents::TextResourceContents { text, .. } => text.clone(),
                            _ => String::new(),
                        };
                        tool_content.push(Content::text(text));
                    }
                    _ => tool_content.push(content.no_annotation()),
                }
            }

            let tool_response_content: Value = json!(tool_content
                .iter()
                .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                .collect::<Vec<String>>()
                .join(" "));

            result.push(DatabricksMessage {
                content: tool_response_content,
                role: "tool".to_string(),
                tool_call_id: Some(response.id.clone()),
                tool_calls: None,
            });
            result.extend(image_messages);
        }
        Err(e) => {
            result.push(DatabricksMessage {
                role: "tool".to_string(),
                content: format!("The tool call returned the following error:\n{}", e).into(),
                tool_call_id: Some(response.id.clone()),
                tool_calls: None,
            });
        }
    }

    result
}

/// Convert internal Message format to Databricks' API message specification
///   Databricks is mostly OpenAI compatible, but has some differences (reasoning type, etc)
///   some openai compatible endpoints use the anthropic image spec at the content level
///   even though the message structure is otherwise following openai, the enum switches this
pub(super) fn format_messages(
    messages: &[Message],
    image_format: &ImageFormat,
) -> Vec<DatabricksMessage> {
    let mut result = Vec::new();
    for message in messages {
        let mut converted = DatabricksMessage {
            content: Value::Null,
            role: match message.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            tool_calls: None,
            tool_call_id: None,
        };

        let mut content_array = Vec::new();
        let mut has_tool_calls = false;
        let mut has_multiple_content = false;
        // Deferred so all tool-role messages stay consecutive (required by Claude via Databricks).
        let mut pending_image_messages: Vec<DatabricksMessage> = Vec::new();

        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    if !text.text.is_empty() {
                        let (items, multi) = format_text_content(&text.text, image_format);
                        content_array.extend(items);
                        has_multiple_content |= multi;
                    }
                }
                MessageContent::Thinking(content) => {
                    has_multiple_content = true;
                    content_array.push(json!({
                        "type": "reasoning",
                        "summary": [{
                            "type": "summary_text",
                            "text": content.thinking,
                            "signature": content.signature
                        }]
                    }));
                }
                MessageContent::RedactedThinking(content) => {
                    has_multiple_content = true;
                    content_array.push(json!({
                        "type": "reasoning",
                        "summary": [{"type": "summary_encrypted_text", "data": content.data}]
                    }));
                }
                MessageContent::ToolRequest(request) => {
                    has_tool_calls = true;
                    match &request.tool_call {
                        Ok(tool_call) => {
                            let sanitized_name = sanitize_function_name(&tool_call.name);
                            let arguments_str = tool_call
                                .arguments
                                .as_ref()
                                .map(|args| {
                                    serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
                                })
                                .unwrap_or_else(|| "{}".to_string());

                            let tool_calls = converted.tool_calls.get_or_insert_default();
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

                            tool_calls.push(tool_call_json);
                        }
                        Err(e) => {
                            content_array
                                .push(json!({"type": "text", "text": format!("Error: {}", e)}));
                        }
                    }
                }
                MessageContent::ToolResponse(response) => {
                    for msg in format_tool_response(response, image_format) {
                        if msg.role == "user" {
                            pending_image_messages.push(msg);
                        } else {
                            result.push(msg);
                        }
                    }
                }
                MessageContent::Image(image) => {
                    content_array.push(convert_image(image, image_format));
                }
                MessageContent::FrontendToolRequest(req) => {
                    let text = match &req.tool_call {
                        Ok(tool_call) => format!(
                            "Frontend tool request: {} ({})",
                            tool_call.name,
                            serde_json::to_string_pretty(&tool_call.arguments).unwrap()
                        ),
                        Err(e) => format!("Frontend tool request error: {}", e),
                    };
                    content_array.push(json!({"type": "text", "text": text}));
                }
                MessageContent::SystemNotification(_)
                | MessageContent::ToolConfirmationRequest(_)
                | MessageContent::ActionRequired(_) => {}
            }
        }

        result.extend(pending_image_messages);

        if !content_array.is_empty() {
            converted.content = if content_array.len() == 1
                && !has_multiple_content
                && content_array[0]["type"] == "text"
            {
                json!(content_array[0]["text"])
            } else {
                json!(content_array)
            };
        }

        if !content_array.is_empty() || has_tool_calls {
            result.push(converted);
        }
    }

    result
}

/// Convert Databricks' API response to internal Message format
#[allow(clippy::too_many_lines)]
pub fn response_to_message(response: &Value) -> anyhow::Result<Message> {
    let original = &response["choices"][0]["message"];
    let mut content = Vec::new();

    // Handle array-based content
    if let Some(content_array) = original.get("content").and_then(|c| c.as_array()) {
        for content_item in content_array {
            match content_item.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(text) = content_item.get("text").and_then(|t| t.as_str()) {
                        content.push(MessageContent::text(text));
                    }
                }
                Some("reasoning") => {
                    if let Some(summary_array) =
                        content_item.get("summary").and_then(|s| s.as_array())
                    {
                        for summary in summary_array {
                            match summary.get("type").and_then(|t| t.as_str()) {
                                Some("summary_text") => {
                                    let text = summary
                                        .get("text")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or_default();
                                    let signature = summary
                                        .get("signature")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or_default();
                                    content.push(MessageContent::thinking(text, signature));
                                }
                                Some("summary_encrypted_text") => {
                                    if let Some(data) = summary.get("data").and_then(|d| d.as_str())
                                    {
                                        content.push(MessageContent::redacted_thinking(data));
                                    }
                                }
                                _ => continue,
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
    } else if let Some(text) = original.get("content").and_then(|t| t.as_str()) {
        // Handle legacy single string content
        content.push(MessageContent::text(text));
    }

    // Handle tool calls
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
                                Ok(CallToolRequestParams::new(function_name)
                                    .with_arguments(object(params))),
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

/// Check if the model name indicates a Claude/Anthropic model that supports cache control.
pub(super) fn is_claude_model(model_name: &str) -> bool {
    model_name.contains("claude")
}
