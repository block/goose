use super::*;

pub const THOUGHT_SIGNATURE_KEY: &str = "thoughtSignature";
pub(super) const SYNTHETIC_THOUGHT_SIGNATURE: &str = "skip_thought_signature_validator";

pub fn metadata_with_signature(signature: &str) -> ProviderMetadata {
    let mut map = ProviderMetadata::new();
    map.insert(THOUGHT_SIGNATURE_KEY.to_string(), json!(signature));
    map
}

pub fn get_thought_signature(metadata: &Option<ProviderMetadata>) -> Option<&str> {
    metadata
        .as_ref()
        .and_then(|m| m.get(THOUGHT_SIGNATURE_KEY))
        .and_then(|v| v.as_str())
}

pub(super) fn is_user_loop_boundary(message: &Message) -> bool {
    message.role == Role::User
        && message
            .content
            .iter()
            .any(|content| !matches!(content, MessageContent::ToolResponse(_)))
}

pub(super) fn insert_thought_signature(part: &mut Map<String, Value>, signature: &str) {
    part.insert(THOUGHT_SIGNATURE_KEY.to_string(), json!(signature));
}

pub(super) fn maybe_insert_signature_from_metadata(
    part: &mut Map<String, Value>,
    metadata: &Option<ProviderMetadata>,
) {
    if let Some(signature) = get_thought_signature(metadata) {
        insert_thought_signature(part, signature);
    }
}

pub(super) fn build_function_response_part(name: &str, text: String) -> Map<String, Value> {
    let mut part = Map::new();
    let mut function_response = Map::new();
    function_response.insert("name".to_string(), json!(name));
    function_response.insert("response".to_string(), json!({"content": {"text": text}}));
    part.insert("functionResponse".to_string(), json!(function_response));
    part
}

/// Convert internal Message format to Google's API message specification
pub fn format_messages(messages: &[Message]) -> Vec<Value> {
    let filtered: Vec<_> = messages
        .iter()
        .filter(|m| m.is_agent_visible())
        .filter(|message| {
            message.content.iter().any(|content| {
                !matches!(
                    content,
                    MessageContent::ToolConfirmationRequest(_) | MessageContent::ActionRequired(_)
                )
            })
        })
        .collect();

    let active_loop_start_idx = filtered
        .iter()
        .enumerate()
        .rev()
        .find(|(_, m)| is_user_loop_boundary(m))
        .map(|(i, _)| i);

    filtered
        .iter()
        .enumerate()
        .filter_map(|(idx, message)| {
            let role = if message.role == Role::User {
                "user"
            } else {
                "model"
            };
            let include_signature = active_loop_start_idx.is_none_or(|start_idx| idx >= start_idx);
            // Only the first model tool call in a turn is guaranteed to carry
            // a signature for loop continuity.
            let mut needs_synthetic_for_first_model_tool_call =
                include_signature && message.role != Role::User;
            let mut parts = Vec::new();
            for message_content in message.content.iter() {
                match message_content {
                    MessageContent::Text(text) => {
                        if !text.text.is_empty() {
                            parts.push(json!({"text": text.text}));
                        }
                    }
                    MessageContent::ToolRequest(request) => match &request.tool_call {
                        Ok(tool_call) => {
                            let mut function_call_part = Map::new();
                            function_call_part.insert(
                                "name".to_string(),
                                json!(sanitize_function_name(&tool_call.name)),
                            );

                            if let Some(args) = &tool_call.arguments {
                                if !args.is_empty() {
                                    function_call_part
                                        .insert("args".to_string(), args.clone().into());
                                }
                            }

                            let mut part = Map::new();
                            part.insert("functionCall".to_string(), json!(function_call_part));

                            if include_signature {
                                if let Some(signature) = get_thought_signature(&request.metadata) {
                                    insert_thought_signature(&mut part, signature);
                                } else if needs_synthetic_for_first_model_tool_call {
                                    insert_thought_signature(
                                        &mut part,
                                        SYNTHETIC_THOUGHT_SIGNATURE,
                                    );
                                }
                            }
                            needs_synthetic_for_first_model_tool_call = false;

                            parts.push(json!(part));
                        }
                        Err(e) => {
                            parts.push(json!({"text":format!("Error: {}", e)}));
                        }
                    },
                    MessageContent::ToolResponse(response) => match &response.tool_result {
                        Ok(result) => {
                            let mut tool_content = Vec::new();
                            for content in result.content.iter().map(|c| c.raw.clone()) {
                                match content {
                                    RawContent::Image(image) => {
                                        parts.push(json!({
                                            "inline_data": {
                                                "mime_type": image.mime_type,
                                                "data": image.data,
                                            }
                                        }));
                                    }
                                    _ => {
                                        tool_content.push(content.no_annotation());
                                    }
                                }
                            }
                            let mut text = tool_content
                                .iter()
                                .filter_map(|c| match c.deref() {
                                    RawContent::Text(t) => Some(t.text.clone()),
                                    RawContent::Resource(raw_embedded_resource) => Some(
                                        raw_embedded_resource.clone().no_annotation().get_text(),
                                    ),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");

                            if text.is_empty() {
                                text = "Tool call is done.".to_string();
                            }
                            let mut part = build_function_response_part(&response.id, text);
                            if include_signature {
                                maybe_insert_signature_from_metadata(&mut part, &response.metadata);
                            }
                            parts.push(json!(part));
                        }
                        Err(e) => {
                            let mut part =
                                build_function_response_part(&response.id, format!("Error: {}", e));
                            if include_signature {
                                maybe_insert_signature_from_metadata(&mut part, &response.metadata);
                            }
                            parts.push(json!(part));
                        }
                    },
                    MessageContent::Thinking(_) => {}
                    MessageContent::Image(image) => {
                        parts.push(json!({
                            "inline_data": {
                                "mime_type": image.mime_type,
                                "data": image.data,
                            }
                        }));
                    }

                    _ => {}
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(json!({"role": role, "parts": parts}))
            }
        })
        .collect()
}

pub fn format_tools(tools: &[Tool]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            let mut parameters = Map::new();
            parameters.insert("name".to_string(), json!(tool.name));
            parameters.insert("description".to_string(), json!(tool.description));

            // Use parametersJsonSchema which supports full JSON Schema including $ref/$defs
            if tool
                .input_schema
                .get("properties")
                .and_then(|v| v.as_object())
                .is_some_and(|p| !p.is_empty())
            {
                parameters.insert("parametersJsonSchema".to_string(), json!(tool.input_schema));
            }
            json!(parameters)
        })
        .collect()
}

pub(super) fn process_response_part_impl(
    part: &Value,
    last_signature: &mut Option<String>,
) -> Option<MessageContent> {
    let signature = part.get(THOUGHT_SIGNATURE_KEY).and_then(|v| v.as_str());
    let is_thought = part
        .get("thought")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    if let Some(sig) = signature {
        *last_signature = Some(sig.to_string());
    }

    let text_value = part.get("text");
    if let Some(text) = text_value.and_then(|v| v.as_str()) {
        if text.is_empty() {
            return None;
        }
        if is_thought {
            match signature {
                Some(sig) => Some(MessageContent::thinking(text.to_string(), sig.to_string())),
                None => Some(MessageContent::thinking(text.to_string(), "")),
            }
        } else {
            Some(MessageContent::text(text.to_string()))
        }
    } else if text_value.is_some() {
        tracing::warn!(
            "Google response part has 'text' field but it's not a string: {:?}",
            text_value
        );
        None
    } else if let Some(function_call) = part.get("functionCall") {
        let id = Uuid::new_v4().to_string();
        let name = function_call["name"].as_str().unwrap_or_default();

        if !is_valid_function_name(name) {
            let error = ErrorData {
                code: ErrorCode::INVALID_REQUEST,
                message: Cow::from(format!(
                    "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                    name
                )),
                data: None,
            };
            Some(MessageContent::tool_request(id, Err(error)))
        } else {
            let arguments = function_call
                .get("args")
                .map(|params| object(params.clone()));
            let effective_signature = signature.or(last_signature.as_deref());
            let metadata = effective_signature.map(metadata_with_signature);

            Some(MessageContent::tool_request_with_metadata(
                id,
                Ok({
                    let mut params = CallToolRequestParams::new(name.to_string());
                    if let Some(args) = arguments {
                        params = params.with_arguments(args);
                    }
                    params
                }),
                metadata.as_ref(),
            ))
        }
    } else {
        None
    }
}

pub fn response_to_message(response: Value) -> Result<Message> {
    let role = Role::Assistant;
    let created = chrono::Utc::now().timestamp();

    let parts = response
        .get("candidates")
        .and_then(|v| v.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array());

    let Some(parts) = parts else {
        return Ok(Message::new(role, created, Vec::new()));
    };

    let mut content = Vec::new();
    let mut last_signature: Option<String> = None;

    for part in parts {
        if let Some(msg_content) = process_response_part_impl(part, &mut last_signature) {
            content.push(msg_content);
        }
    }
    Ok(Message::new(role, created, content))
}

/// Extract usage information from Google's API response
pub fn get_usage(data: &Value) -> Result<Usage> {
    if let Some(usage_meta_data) = data.get("usageMetadata") {
        let input_tokens = usage_meta_data
            .get("promptTokenCount")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);
        let output_tokens = usage_meta_data
            .get("candidatesTokenCount")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);
        let total_tokens = usage_meta_data
            .get("totalTokenCount")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);
        Ok(Usage::new(input_tokens, output_tokens, total_tokens))
    } else {
        tracing::debug!(
            "Failed to get usage data: {}",
            ProviderError::UsageError("No usage data found in response".to_string())
        );
        // If no usage data, return None for all values
        Ok(Usage::new(None, None, None))
    }
}
