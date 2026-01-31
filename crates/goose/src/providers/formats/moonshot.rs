use crate::conversation::message::{Message, MessageContent, ProviderMetadata};
use crate::providers::base::ProviderUsage;
use crate::providers::formats::openai;
use anyhow::anyhow;
use async_stream::try_stream;
use futures::Stream;
use rmcp::model::{object, CallToolRequestParams, ErrorCode, ErrorData, Role};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;

pub const REASONING_CONTENT_KEY: &str = "reasoning_content";

fn has_assistant_content(message: &Message) -> bool {
    message.content.iter().any(|c| match c {
        MessageContent::Text(t) => !t.text.is_empty(),
        MessageContent::Image(_) => true,
        MessageContent::ToolRequest(req) => req.tool_call.is_ok(),
        MessageContent::FrontendToolRequest(req) => req.tool_call.is_ok(),
        _ => false,
    })
}

pub fn extract_reasoning_content(response: &Value) -> Option<String> {
    response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|m| m.get("message"))
        .and_then(|msg| msg.get("reasoning_content"))
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
}

pub fn get_reasoning_content(metadata: &Option<ProviderMetadata>) -> Option<String> {
    metadata
        .as_ref()
        .and_then(|m| m.get(REASONING_CONTENT_KEY))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub fn response_to_message(response: &Value) -> anyhow::Result<Message> {
    let mut message = openai::response_to_message(response)?;

    if let Some(reasoning) = extract_reasoning_content(response) {
        for content in &mut message.content {
            if let MessageContent::ToolRequest(req) = content {
                let mut meta = req.metadata.clone().unwrap_or_default();
                meta.insert(REASONING_CONTENT_KEY.to_string(), json!(reasoning));
                req.metadata = Some(meta);
            }
        }
    }

    Ok(message)
}

pub fn add_reasoning_content_to_request(payload: &mut Value, messages: &[Message]) {
    let mut assistant_reasoning: Vec<Option<String>> = messages
        .iter()
        .filter(|m| m.is_agent_visible())
        .filter(|m| m.role == Role::Assistant)
        .filter(|m| has_assistant_content(m))
        .map(|message| {
            message.content.iter().find_map(|c| match c {
                MessageContent::ToolRequest(req) => get_reasoning_content(&req.metadata),
                _ => None,
            })
        })
        .collect();

    if let Some(payload_messages) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("messages"))
        .and_then(|m| m.as_array_mut())
    {
        let mut assistant_idx = 0;
        for payload_msg in payload_messages.iter_mut() {
            if payload_msg.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                if assistant_idx < assistant_reasoning.len() {
                    if let Some(reasoning) = assistant_reasoning
                        .get_mut(assistant_idx)
                        .and_then(|r| r.take())
                    {
                        if let Some(obj) = payload_msg.as_object_mut() {
                            obj.insert("reasoning_content".to_string(), json!(reasoning));
                        }
                    }
                }
                assistant_idx += 1;
            }
        }
    }
}

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

        let mut accumulated_reasoning: Option<String> = None;

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
                if let Some(reasoning) = &chunk.choices[0].delta.reasoning_content {
                    match &mut accumulated_reasoning {
                        Some(acc) => acc.push_str(reasoning),
                        None => accumulated_reasoning = Some(reasoning.clone()),
                    }
                }
            }

            let usage = chunk.usage.as_ref().and_then(|u| {
                chunk.model.as_ref().map(|model| {
                    ProviderUsage {
                        usage: openai::get_usage(u),
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
                                    if let Some(reasoning) = &tool_chunk.choices[0].delta.reasoning_content {
                                        match &mut accumulated_reasoning {
                                            Some(acc) => acc.push_str(reasoning),
                                            None => accumulated_reasoning = Some(reasoning.clone()),
                                        }
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

                let metadata: Option<ProviderMetadata> = accumulated_reasoning.as_ref().map(|reasoning| {
                    let mut map = ProviderMetadata::new();
                    map.insert(REASONING_CONTENT_KEY.to_string(), json!(reasoning));
                    map
                });

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
                                        meta: None,
                                        task: None,
                                        name: function_name.clone().into(),
                                        arguments: Some(object(params)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_reasoning_content() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": "Hello",
                    "reasoning_content": "Let me think about this..."
                }
            }]
        });

        let reasoning = extract_reasoning_content(&response).unwrap();
        assert_eq!(reasoning, "Let me think about this...");
    }

    #[test]
    fn test_response_to_message_with_tool_calls() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"NYC\"}"
                        }
                    }],
                    "reasoning_content": "I need to check the weather"
                }
            }]
        });

        let message = response_to_message(&response).unwrap();
        assert!(!message.content.is_empty());

        let tool_request = message
            .content
            .iter()
            .find_map(|c| {
                if let MessageContent::ToolRequest(req) = c {
                    Some(req)
                } else {
                    None
                }
            })
            .unwrap();

        assert!(tool_request.metadata.is_some());
        let reasoning = get_reasoning_content(&tool_request.metadata).unwrap();
        assert_eq!(reasoning, "I need to check the weather");
    }
}
