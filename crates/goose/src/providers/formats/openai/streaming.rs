use super::*;

fn strip_data_prefix(line: &str) -> Option<&str> {
    // SSE spec allows both "data: value" and "data:value" (space after colon is optional)
    line.strip_prefix("data: ")
        .or_else(|| line.strip_prefix("data:"))
        .map(|s| s.trim())
}

fn parse_streaming_chunk(line: &str) -> Result<StreamingChunk, ProviderError> {
    let value: Value = serde_json::from_str(line).map_err(|e| {
        ProviderError::RequestFailed(format!("Failed to parse streaming chunk: {e}: {line:?}"))
    })?;

    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown server error");
        return Err(ProviderError::ServerError(message.to_string()));
    }

    if value.get("object").and_then(|o| o.as_str()) == Some("error") {
        let message = value
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown server error");
        return Err(ProviderError::ServerError(message.to_string()));
    }

    serde_json::from_value(value).map_err(|e| {
        ProviderError::RequestFailed(format!("Failed to parse streaming chunk: {e}: {line:?}"))
    })
}

pub(super) fn extract_usage_with_output_tokens(
    chunk: &StreamingChunk,
    fallback_model: Option<&str>,
) -> Option<ProviderUsage> {
    chunk
        .usage
        .as_ref()
        .and_then(|u| {
            chunk
                .model
                .as_deref()
                .or(fallback_model)
                .map(|model| ProviderUsage {
                    usage: get_usage(u),
                    model: model.to_string(),
                })
        })
        .filter(|u| u.usage.output_tokens.is_some())
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
        let mut think_filter = ThinkFilter::new();
        let mut saw_structured_reasoning = false;
        let mut yielded_reasoning_content_len = 0usize;
        let mut last_signature: Option<String> = None;
        // Buffer inline <think>...</think> content until we know whether structured
        // reasoning will arrive. Emitting it immediately and then receiving
        // reasoning_content in a later chunk would produce duplicated reasoning.
        let mut pending_inline_thinking = String::new();
        let mut last_seen_model: Option<String> = None;

        'outer: while let Some(response) = stream.next().await {
            let response_str = response?;
            let line = strip_data_prefix(&response_str);

            if line.is_some_and(|l| l == "[DONE]") {
                break 'outer;
            }

            if line.is_none() || line.is_some_and(|l| l.is_empty()) {
                continue
            }

            let chunk: StreamingChunk = parse_streaming_chunk(
                line.ok_or_else(|| anyhow!("unexpected stream format"))?
            )?;
            if let Some(model) = &chunk.model {
                last_seen_model = Some(model.clone());
            }

            if !chunk.choices.is_empty() {
                if let Some(details) = &chunk.choices[0].delta.reasoning_details {
                    accumulated_reasoning.extend(details.iter().cloned());
                }
                if let Some(rc) = chunk.choices[0].delta.reasoning_text() {
                    accumulated_reasoning_content.push_str(rc);
                    if !rc.is_empty() {
                        saw_structured_reasoning = true;
                        pending_inline_thinking.clear();
                    }
                }
            }

            let mut usage = extract_usage_with_output_tokens(&chunk, last_seen_model.as_deref());

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
                            let response_str = response_chunk?;
                            if let Some(line) = strip_data_prefix(&response_str) {
                                if line == "[DONE]" {
                                    break 'outer;
                                }

                                let tool_chunk: StreamingChunk = parse_streaming_chunk(line)?;
                                if let Some(model) = &tool_chunk.model {
                                    last_seen_model = Some(model.clone());
                                }

                                if let Some(chunk_usage) = extract_usage_with_output_tokens(&tool_chunk, last_seen_model.as_deref()) {
                                    usage = Some(chunk_usage);
                                }

                                if !tool_chunk.choices.is_empty() {
                                    if let Some(details) = &tool_chunk.choices[0].delta.reasoning_details {
                                        accumulated_reasoning.extend(details.iter().cloned());
                                    }
                                    if let Some(rc) = tool_chunk.choices[0].delta.reasoning_text() {
                                        accumulated_reasoning_content.push_str(rc);
                                        if !rc.is_empty() {
                                            saw_structured_reasoning = true;
                                            pending_inline_thinking.clear();
                                        }
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

                let filtered = think_filter.push("");
                let mut flush_thinking = String::new();
                if !saw_structured_reasoning {
                    flush_thinking.push_str(&pending_inline_thinking);
                    flush_thinking.push_str(&filtered.thinking);
                }
                pending_inline_thinking.clear();
                if !filtered.content.is_empty() || !flush_thinking.is_empty() {
                    let mut filtered_contents = Vec::new();
                    if !filtered.content.is_empty() {
                        filtered_contents.push(MessageContent::text(filtered.content));
                    }
                    if !flush_thinking.is_empty() {
                        filtered_contents.push(MessageContent::thinking(flush_thinking, ""));
                    }

                    if !filtered_contents.is_empty() {
                        let mut msg = Message::new(
                            Role::Assistant,
                            chrono::Utc::now().timestamp(),
                            filtered_contents,
                        );

                        if let Some(id) = chunk.id.clone() {
                            msg = msg.with_id(id);
                        }

                        yield (Some(msg), None);
                    }
                }

                let mut contents = Vec::new();
                if yielded_reasoning_content_len < accumulated_reasoning_content.len() {
                    if let Some(unyielded_reasoning) =
                        accumulated_reasoning_content.get(yielded_reasoning_content_len..)
                    {
                        if !unyielded_reasoning.is_empty() {
                            contents.push(MessageContent::thinking(unyielded_reasoning, ""));
                        }
                    }
                }
                accumulated_reasoning_content.clear();
                yielded_reasoning_content_len = 0;
                let mut sorted_indices: Vec<_> = tool_call_data.keys().cloned().collect();
                sorted_indices.sort();

                for index in sorted_indices {
                    if let Some((id, function_name, arguments, extra_fields)) = tool_call_data.get(&index) {
                        let parsed = if arguments.is_empty() {
                            Ok(json!({}))
                        } else {
                            safely_parse_json(arguments)
                        };

                        let metadata = if let Some(sig) = &last_signature {
                            let mut combined = extra_fields.clone().unwrap_or_default();
                            combined.insert(
                                crate::providers::formats::google::THOUGHT_SIGNATURE_KEY.to_string(),
                                json!(sig)
                            );
                            Some(combined)
                        } else {
                            extra_fields.as_ref().filter(|m| !m.is_empty()).cloned()
                        };

                        let content = match parsed {
                            Ok(params) => {
                                MessageContent::tool_request_with_metadata(
                                    id.clone(),
                                    Ok(CallToolRequestParams::new(function_name.clone()).with_arguments(object(params))),
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
            } else if chunk.choices[0].delta.content.is_some() || chunk.choices[0].delta.reasoning_text().is_some() {
                let mut content = Vec::new();

                if let Some(reasoning) = chunk.choices[0].delta.reasoning_text() {
                    let signature = last_signature.as_deref().unwrap_or("");
                    content.push(MessageContent::thinking(reasoning, signature));
                    yielded_reasoning_content_len = accumulated_reasoning_content.len();
                }

                let (text_content, thought_signature) = extract_content_and_signature(chunk.choices[0].delta.content.as_ref());

                if let Some(sig) = thought_signature {
                    last_signature = Some(sig);
                }

                if let Some(text) = text_content {
                    let filtered = think_filter.push(&text);

                    if !saw_structured_reasoning && !filtered.thinking.is_empty() {
                        pending_inline_thinking.push_str(&filtered.thinking);
                    }

                    if !filtered.content.is_empty() {
                        content.push(MessageContent::text(filtered.content));
                    }
                }

                if !content.is_empty() {
                    let mut msg = Message::new(
                        Role::Assistant,
                        chrono::Utc::now().timestamp(),
                        content,
                    );

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

        let filtered = think_filter.finish();
        let mut trailing_thinking = String::new();
        if !saw_structured_reasoning {
            trailing_thinking.push_str(&pending_inline_thinking);
            trailing_thinking.push_str(&filtered.thinking);
        }
        pending_inline_thinking.clear();

        if !filtered.content.is_empty() || !trailing_thinking.is_empty() {
            let mut content = Vec::new();

            if !filtered.content.is_empty() {
                content.push(MessageContent::text(filtered.content));
            }

            if !trailing_thinking.is_empty() {
                content.push(MessageContent::thinking(trailing_thinking, ""));
            }

            yield (
                Some(Message::new(
                    Role::Assistant,
                    chrono::Utc::now().timestamp(),
                    content,
                )),
                None,
            )
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
    create_request_with_options(
        model_config,
        system,
        messages,
        tools,
        image_format,
        for_streaming,
        OpenAiFormatOptions {
            preserve_thinking_context: true,
        },
    )
}

pub fn create_request_with_options(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
    for_streaming: bool,
    format_options: OpenAiFormatOptions,
) -> anyhow::Result<Value, Error> {
    if model_config.model_name.starts_with("o1-mini") {
        return Err(anyhow!(
            "o1-mini model is not currently supported since goose uses tool calling and o1-mini does not support it. Please use o1 or o3 models instead."
        ));
    }

    let (model_name, legacy_reasoning_effort) = extract_reasoning_effort(&model_config.model_name);
    let is_reasoning_model = is_openai_responses_model(&model_name);
    let reasoning_effort = if is_reasoning_model {
        model_config
            .thinking_effort()
            .map_or(legacy_reasoning_effort, |effort| {
                openai_reasoning_effort_for_thinking(&model_name, effort)
            })
    } else {
        None
    };

    let system_message = json!({
        "role": if is_reasoning_model { "developer" } else { "system" },
        "content": system
    });

    let messages_spec = format_messages_with_options(messages, image_format, format_options);
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

    if !is_reasoning_model {
        if let Some(temp) = model_config.temperature {
            payload["temperature"] = json!(temp);
        }
    }

    // Only emit max_tokens / max_completion_tokens when the user (via
    // GOOSE_MAX_TOKENS) or a canonical model record has supplied a value.
    // For unknown models on OpenAI-compatible endpoints (e.g. llama_swap,
    // lmstudio) sending the historic 4096 default truncates non-trivial
    // responses; omitting the field lets the server use its own max.
    if let Some(max_tokens) = model_config.max_tokens {
        let key = if is_reasoning_model {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        payload
            .as_object_mut()
            .unwrap()
            .insert(key.to_string(), json!(max_tokens));
    }

    if for_streaming {
        payload["stream"] = json!(true);
        payload["stream_options"] = json!({"include_usage": true});
    }

    if let Some(params) = &model_config.request_params {
        if let Some(obj) = payload.as_object_mut() {
            for (key, value) in params {
                if key != "thinking_effort" && !is_reserved_request_param_key(key) {
                    obj.insert(key.clone(), value.clone());
                }
            }
        }
    }

    Ok(payload)
}
