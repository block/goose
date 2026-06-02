use super::*;

pub(super) fn build_input_items(messages: &[Message]) -> Result<Vec<Value>> {
    let mut items = Vec::new();

    for message in messages {
        let role = match message.role {
            Role::User => Some("user"),
            Role::Assistant => Some("assistant"),
        };
        let mut content_items: Vec<Value> = Vec::new();

        let flush_text = |items: &mut Vec<Value>, role: Option<&str>, content: &mut Vec<Value>| {
            if let Some(role) = role {
                if !content.is_empty() {
                    items.push(json!({ "role": role, "content": std::mem::take(content) }));
                }
            } else {
                content.clear();
            }
        };

        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    if !text.text.is_empty() {
                        let content_type = if message.role == Role::Assistant {
                            "output_text"
                        } else {
                            "input_text"
                        };
                        content_items.push(json!({ "type": content_type, "text": text.text }));
                    }
                }
                MessageContent::Image(img) => {
                    content_items.push(json!({
                        "type": "input_image",
                        "image_url": format!("data:{};base64,{}", img.mime_type, img.data),
                    }));
                }
                MessageContent::ToolRequest(request) => {
                    flush_text(&mut items, role, &mut content_items);
                    if let Ok(tool_call) = &request.tool_call {
                        let arguments_str = match tool_call.arguments.as_ref() {
                            Some(args) => serde_json::to_string(args)?,
                            None => "{}".to_string(),
                        };
                        items.push(json!({
                            "type": "function_call",
                            "call_id": request.id,
                            "name": tool_call.name,
                            "arguments": arguments_str
                        }));
                    }
                }
                MessageContent::ToolResponse(response) => {
                    flush_text(&mut items, role, &mut content_items);
                    match &response.tool_result {
                        Ok(contents) => {
                            let text_content: Vec<String> = contents
                                .content
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
                                items.push(json!({
                                    "type": "function_call_output",
                                    "call_id": response.id,
                                    "output": text_content.join("\n")
                                }));
                            }
                        }
                        Err(error_data) => {
                            items.push(json!({
                                "type": "function_call_output",
                                "call_id": response.id,
                                "output": format!("Error: {}", error_data.message)
                            }));
                        }
                    }
                }
                _ => {}
            }
        }

        flush_text(&mut items, role, &mut content_items);
    }

    Ok(items)
}

pub(super) fn create_codex_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let input_items = build_input_items(messages)?;
    let reasoning_effort = reasoning_effort_for_config(model_config);

    let instructions = match model_config.model_name.as_str() {
        "gpt-5.3-codex" => format!("{GPT_53_CODEX_TOOL_PREAMBLE}\n\n{system}"),
        _ => system.to_string(),
    };

    let mut payload = json!({
        "model": model_config.model_name,
        "input": input_items,
        "store": false,
        "instructions": instructions,
    });

    let payload_obj = payload
        .as_object_mut()
        .ok_or_else(|| anyhow!("Codex payload must be a JSON object"))?;

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

        payload_obj.insert("tools".to_string(), json!(tools_spec));
        payload_obj.insert("tool_choice".to_string(), json!("auto"));
        payload_obj.insert("parallel_tool_calls".to_string(), json!(true));
    }

    if let Some(temp) = model_config.temperature {
        payload_obj.insert("temperature".to_string(), json!(temp));
    }

    if let Some(reasoning_effort) = reasoning_effort {
        payload_obj.insert(
            "reasoning".to_string(),
            json!({ "effort": reasoning_effort }),
        );
    }

    Ok(payload)
}
