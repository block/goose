use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::{ProviderUsage, Usage};
use anyhow::{anyhow, Error};
use async_stream::try_stream;
use chrono;
use futures::Stream;
use rmcp::model::{object, CallToolRequestParam, RawContent, Role, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::ops::Deref;

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
    #[serde(rename = "response.output_text.done")]
    OutputTextDone {
        sequence_number: i32,
        item_id: String,
        output_index: i32,
        content_index: i32,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        logprobs: Option<Vec<Value>>,
    },
    #[serde(rename = "response.completed")]
    ResponseCompleted {
        sequence_number: i32,
        response: ResponseMetadata,
    },
    #[serde(rename = "response.failed")]
    ResponseFailed { sequence_number: i32, error: Value },
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
    Error { error: Value },
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

fn has_fresh_tool_responses(messages: &[Message]) -> bool {
    if let Some(last_idx) = messages.iter().rposition(|m| m.role == Role::Assistant) {
        let last_assistant = &messages[last_idx];
        let has_tool_requests = last_assistant
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolRequest(_)));

        if has_tool_requests {
            let after_assistant = &messages[last_idx + 1..];
            let has_tool_responses = after_assistant.iter().any(|m| {
                m.content
                    .iter()
                    .any(|c| matches!(c, MessageContent::ToolResponse(_)))
            });
            let has_text_after = after_assistant.iter().any(|m| {
                m.role == Role::Assistant
                    && m.content
                        .iter()
                        .any(|c| matches!(c, MessageContent::Text(_)))
            });
            has_tool_responses && !has_text_after
        } else {
            false
        }
    } else {
        false
    }
}

fn add_conversation_history(input_items: &mut Vec<Value>, messages: &[Message]) {
    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        let has_only_tool_content = message.content.iter().all(|c| {
            matches!(
                c,
                MessageContent::ToolRequest(_) | MessageContent::ToolResponse(_)
            )
        });

        if has_only_tool_content {
            continue;
        }

        if message.role != Role::User && message.role != Role::Assistant {
            continue;
        }

        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let mut content_items = Vec::new();
        for content in &message.content {
            if let MessageContent::Text(text) = content {
                if !text.text.is_empty() {
                    let content_type = if message.role == Role::Assistant {
                        "output_text"
                    } else {
                        "input_text"
                    };
                    content_items.push(json!({
                        "type": content_type,
                        "text": text.text
                    }));
                }
            }
        }

        if !content_items.is_empty() {
            input_items.push(json!({
                "role": role,
                "content": content_items
            }));
        }
    }
}

fn add_function_calls(input_items: &mut Vec<Value>, messages: &[Message]) {
    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        if message.role == Role::Assistant {
            for content in &message.content {
                if let MessageContent::ToolRequest(request) = content {
                    if let Ok(tool_call) = &request.tool_call {
                        let arguments_str = tool_call
                            .arguments
                            .as_ref()
                            .map(|args| {
                                serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
                            })
                            .unwrap_or_else(|| "{}".to_string());

                        tracing::debug!(
                            "Replaying function_call with call_id: {}, name: {}",
                            request.id,
                            tool_call.name
                        );
                        input_items.push(json!({
                            "type": "function_call",
                            "call_id": request.id,
                            "name": tool_call.name,
                            "arguments": arguments_str
                        }));
                    }
                }
            }
        }
    }
}

fn add_function_call_outputs(input_items: &mut Vec<Value>, messages: &[Message]) {
    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        for content in &message.content {
            if let MessageContent::ToolResponse(response) = content {
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
                        tracing::debug!(
                            "Sending function_call_output with call_id: {}",
                            response.id
                        );
                        input_items.push(json!({
                            "type": "function_call_output",
                            "call_id": response.id,
                            "output": text_content.join("\n")
                        }));
                    }
                }
            }
        }
    }
}

fn add_full_conversation(input_items: &mut Vec<Value>, messages: &[Message]) {
    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        // Only User and Assistant messages
        if message.role != Role::User && message.role != Role::Assistant {
            continue;
        }

        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let mut content_items = Vec::new();

        for content in &message.content {
            match content {
                MessageContent::Text(text) if !text.text.is_empty() => {
                    let content_type = if message.role == Role::Assistant {
                        "output_text"
                    } else {
                        "input_text"
                    };
                    content_items.push(json!({
                        "type": content_type,
                        "text": text.text
                    }));
                }
                MessageContent::ToolRequest(_) | MessageContent::ToolResponse(_) => {
                    // Skip tool content in full conversation mode
                    continue;
                }
                _ => {}
            }
        }

        if !content_items.is_empty() {
            input_items.push(json!({
                "role": role,
                "content": content_items
            }));
        }
    }
}

pub fn create_responses_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> anyhow::Result<Value, Error> {
    let mut input_items = Vec::new();

    if !system.is_empty() {
        input_items.push(json!({
            "role": "system",
            "content": [{
                "type": "input_text",
                "text": system
            }]
        }));
    }

    if has_fresh_tool_responses(messages) {
        add_conversation_history(&mut input_items, messages);
        add_function_calls(&mut input_items, messages);
        add_function_call_outputs(&mut input_items, messages);
    } else {
        add_full_conversation(&mut input_items, messages);
    }

    // If no messages, provide a minimal input
    if input_items.is_empty() {
        input_items.push(json!({
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "Hello"
            }]
        }));
    }

    let mut payload = json!({
        "model": model_config.model_name,
        "input": input_items,
        "store": false,  // Don't store responses on server (we replay history ourselves)
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
                tracing::debug!("Received FunctionCall with id: {}, name: {}", id, name);
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

/// Process output items and create message content from tool calls
fn process_streaming_output_items(
    output_items: Vec<ResponseOutputItemInfo>,
    is_text_response: bool,
) -> Vec<MessageContent> {
    let mut content = Vec::new();

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
                        ContentPart::ToolCall {
                            id,
                            name,
                            arguments,
                        } => {
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
            ResponseOutputItemInfo::FunctionCall {
                call_id,
                name,
                arguments,
                ..
            } => {
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

    content
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
                    if !delta.is_empty() {
                        content.push(MessageContent::text(&delta));
                    }
                    let mut msg = Message::new(Role::Assistant, chrono::Utc::now().timestamp(), content);

                    // Add ID so desktop client knows these deltas are part of the same message
                    if let Some(id) = &response_id {
                        msg = msg.with_id(id.clone());
                    }

                    yield (Some(msg), None);
                }

                ResponsesStreamEvent::OutputItemDone { item, .. } => {
                    output_items.push(item);
                }

                ResponsesStreamEvent::OutputTextDone { .. } => {
                    // Text is already complete from deltas, this is just a summary event
                }

                ResponsesStreamEvent::ResponseCompleted { response, .. } => {
                    // Always set final usage (use default if not provided)
                    let model = model_name.as_ref().unwrap_or(&response.model);
                    let usage = response.usage.as_ref().map_or_else(
                        Usage::default,
                        |u| Usage::new(
                            Some(u.input_tokens),
                            Some(u.output_tokens),
                            Some(u.total_tokens),
                        ),
                    );
                    final_usage = Some(ProviderUsage {
                        usage,
                        model: model.clone(),
                    });

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

                ResponsesStreamEvent::ResponseFailed { error, .. } => {
                    Err(anyhow!("Responses API failed: {:?}", error))?;
                }

                ResponsesStreamEvent::Error { error } => {
                    Err(anyhow!("Responses API error: {:?}", error))?;
                }

                _ => {
                    // Ignore other event types (OutputItemAdded, ContentPartAdded, ContentPartDone)
                }
            }
        }

        // Process final output items and yield usage data
        let content = process_streaming_output_items(output_items, is_text_response);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::object;

    #[test]
    fn test_create_responses_request_basic() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "gpt-5.1-codex".to_string(),
            context_limit: Some(400_000),
            temperature: Some(0.7),
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model: None,
        };

        let messages = vec![
            Message::user().with_text("Hello, how are you?"),
            Message::assistant().with_text("I'm doing well, thank you!"),
            Message::user().with_text("What's the weather?"),
        ];

        let request =
            create_responses_request(&model_config, "You are a helpful assistant", &messages, &[])?;
        let obj = request.as_object().unwrap();

        // Verify basic structure
        assert_eq!(obj.get("model").unwrap().as_str().unwrap(), "gpt-5.1-codex");
        assert!(!obj.get("store").unwrap().as_bool().unwrap());
        assert!((obj.get("temperature").unwrap().as_f64().unwrap() - 0.7).abs() < 0.01); // Float comparison
        assert_eq!(
            obj.get("max_output_tokens").unwrap().as_i64().unwrap(),
            1024
        );

        // Verify input is an array
        let input = obj.get("input").unwrap().as_array().unwrap();
        assert_eq!(input.len(), 4); // system + 3 messages

        // Verify system message
        assert_eq!(input[0]["role"].as_str().unwrap(), "system");
        assert_eq!(
            input[0]["content"][0]["type"].as_str().unwrap(),
            "input_text"
        );
        assert_eq!(
            input[0]["content"][0]["text"].as_str().unwrap(),
            "You are a helpful assistant"
        );

        // Verify user message
        assert_eq!(input[1]["role"].as_str().unwrap(), "user");
        assert_eq!(
            input[1]["content"][0]["type"].as_str().unwrap(),
            "input_text"
        );
        assert_eq!(
            input[1]["content"][0]["text"].as_str().unwrap(),
            "Hello, how are you?"
        );

        // Verify assistant message
        assert_eq!(input[2]["role"].as_str().unwrap(), "assistant");
        assert_eq!(
            input[2]["content"][0]["type"].as_str().unwrap(),
            "output_text"
        );
        assert_eq!(
            input[2]["content"][0]["text"].as_str().unwrap(),
            "I'm doing well, thank you!"
        );

        Ok(())
    }

    #[test]
    fn test_create_responses_request_with_tools() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "gpt-5.1-codex".to_string(),
            context_limit: Some(400_000),
            temperature: None,
            max_tokens: Some(2048),
            toolshim: false,
            toolshim_model: None,
            fast_model: None,
        };

        let tool = Tool::new(
            "get_weather",
            "Get the current weather",
            object!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name"
                    }
                },
                "required": ["location"]
            }),
        );

        let mut messages = vec![Message::user().with_text("What's the weather in SF?")];

        // Add a tool call from the assistant (will be skipped in input, represented by tool result)
        let msg_with_tool = Message::assistant().with_tool_request(
            "call_1",
            Ok(CallToolRequestParam {
                name: "get_weather".into(),
                arguments: Some(object!({"location": "San Francisco"})),
            }),
        );
        messages.push(msg_with_tool);

        // Get the tool request ID
        let tool_id = if let MessageContent::ToolRequest(req) = &messages[1].content[0] {
            req.id.clone()
        } else {
            panic!("Expected tool request");
        };

        // Add tool response
        messages.push(
            Message::user()
                .with_tool_response(tool_id.clone(), Ok(vec![Content::text("72°F and sunny")])),
        );

        // Add final assistant response
        messages.push(
            Message::assistant().with_text("The weather in San Francisco is 72°F and sunny!"),
        );

        let request = create_responses_request(
            &model_config,
            "You are a weather assistant",
            &messages,
            &[tool],
        )?;
        let obj = request.as_object().unwrap();

        // Verify tools are included
        let tools = obj.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"].as_str().unwrap(), "function");
        assert_eq!(tools[0]["name"].as_str().unwrap(), "get_weather");

        // Verify input structure
        let input = obj.get("input").unwrap().as_array().unwrap();

        // Should have: system, user question, assistant response
        // Tool calls and tool responses are NOT included in history (they're internal to previous turns)
        // Only the assistant's final text response is included

        // Verify no function_call_output in history (they're only for immediate responses)
        let has_tool_output = input
            .iter()
            .any(|item| item["type"].as_str() == Some("function_call_output"));
        assert!(
            !has_tool_output,
            "Tool responses should not be in conversation history"
        );

        // Verify final assistant response is present
        let assistant_msg = input
            .iter()
            .find(|item| item["role"].as_str() == Some("assistant"))
            .unwrap();

        assert_eq!(
            assistant_msg["content"][0]["type"].as_str().unwrap(),
            "output_text"
        );
        assert!(assistant_msg["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("72°F"));

        Ok(())
    }
}
