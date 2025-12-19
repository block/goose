use crate::model::ModelConfig;
use crate::providers::base::Usage;
use crate::providers::errors::ProviderError;
use crate::providers::utils::{is_valid_function_name, sanitize_function_name};
use anyhow::Result;
use rmcp::model::{
    object, AnnotateAble, CallToolRequestParam, ErrorCode, ErrorData, RawContent, Role, Tool,
};
use serde::Serialize;
use std::borrow::Cow;
use uuid::Uuid;

use crate::conversation::message::{Message, MessageContent, ProviderMetadata};
use serde_json::{json, Map, Value};
use std::ops::Deref;

pub const THOUGHT_SIGNATURE_KEY: &str = "thoughtSignature";

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

/// Convert internal Message format to Google's API message specification
pub fn format_messages(messages: &[Message]) -> Vec<Value> {
    messages
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
        .map(|message| {
            let role = if message.role == Role::User {
                "user"
            } else {
                "model"
            };
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

                            if let Some(signature) = get_thought_signature(&request.metadata) {
                                part.insert(THOUGHT_SIGNATURE_KEY.to_string(), json!(signature));
                            }

                            parts.push(json!(part));
                        }
                        Err(e) => {
                            parts.push(json!({"text":format!("Error: {}", e)}));
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
                                        content.audience().is_none_or(|audience| {
                                            audience.contains(&Role::Assistant)
                                        })
                                    })
                                    .map(|content| content.raw.clone())
                                    .collect();

                                let mut tool_content = Vec::new();
                                for content in abridged {
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
                                            raw_embedded_resource
                                                .clone()
                                                .no_annotation()
                                                .get_text(),
                                        ),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");

                                if text.is_empty() {
                                    text = "Tool call is done.".to_string();
                                }
                                let mut part = Map::new();
                                let mut function_response = Map::new();
                                function_response.insert("name".to_string(), json!(response.id));
                                function_response.insert(
                                    "response".to_string(),
                                    json!({"content": {"text": text}}),
                                );
                                part.insert(
                                    "functionResponse".to_string(),
                                    json!(function_response),
                                );
                                if let Some(signature) = get_thought_signature(&response.metadata) {
                                    part.insert(
                                        THOUGHT_SIGNATURE_KEY.to_string(),
                                        json!(signature),
                                    );
                                }
                                parts.push(json!(part));
                            }
                            Err(e) => {
                                let mut part = Map::new();
                                let mut function_response = Map::new();
                                function_response.insert("name".to_string(), json!(response.id));
                                function_response.insert(
                                    "response".to_string(),
                                    json!({"content": {"text": format!("Error: {}", e)}}),
                                );
                                part.insert(
                                    "functionResponse".to_string(),
                                    json!(function_response),
                                );
                                if let Some(signature) = get_thought_signature(&response.metadata) {
                                    part.insert(
                                        THOUGHT_SIGNATURE_KEY.to_string(),
                                        json!(signature),
                                    );
                                }
                                parts.push(json!(part));
                            }
                        }
                    }
                    MessageContent::Thinking(thinking) => {
                        let mut part = Map::new();
                        part.insert("text".to_string(), json!(thinking.thinking));
                        part.insert("thoughtSignature".to_string(), json!(thinking.signature));
                        parts.push(json!(part));
                    }

                    _ => {}
                }
            }
            json!({"role": role, "parts": parts})
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
            let tool_input_schema = &tool.input_schema;

            if tool_input_schema
                .get("properties")
                .and_then(|v| v.as_object())
                .is_some_and(|p| !p.is_empty())
            {
                parameters.insert(
                    "parameters".to_string(),
                    process_map(tool_input_schema, None),
                );
            }
            json!(parameters)
        })
        .collect()
}

pub fn get_accepted_keys(parent_key: Option<&str>) -> Vec<&str> {
    match parent_key {
        Some("properties") => vec![
            "anyOf",
            "allOf",
            "type",
            "description",
            "nullable",
            "enum",
            "properties",
            "required",
            "items",
        ],
        Some("items") => vec!["type", "properties", "items", "required"],
        _ => vec!["type", "properties", "required", "anyOf", "allOf"],
    }
}

pub fn process_value(value: &Value, parent_key: Option<&str>) -> Value {
    match value {
        Value::Object(map) => process_map(map, parent_key),
        Value::Array(arr) if parent_key == Some("type") => arr
            .iter()
            .find(|v| v.as_str() != Some("null"))
            .cloned()
            .unwrap_or_else(|| json!("string")),
        _ => value.clone(),
    }
}

/// Process a JSON map to filter out unsupported attributes, mirroring the logic
/// from the official Google Gemini CLI.
/// See: https://github.com/google-gemini/gemini-cli/blob/8a6509ffeba271a8e7ccb83066a9a31a5d72a647/packages/core/src/tools/tool-registry.ts#L356
pub fn process_map(map: &Map<String, Value>, parent_key: Option<&str>) -> Value {
    let accepted_keys = get_accepted_keys(parent_key);

    let filtered_map: Map<String, Value> = map
        .iter()
        .filter_map(|(key, value)| {
            if !accepted_keys.contains(&key.as_str()) {
                return None;
            }

            let processed_value = match key.as_str() {
                "properties" => {
                    if let Some(nested_map) = value.as_object() {
                        let processed_properties: Map<String, Value> = nested_map
                            .iter()
                            .map(|(prop_key, prop_value)| {
                                if let Some(prop_obj) = prop_value.as_object() {
                                    (prop_key.clone(), process_map(prop_obj, Some("properties")))
                                } else {
                                    (prop_key.clone(), prop_value.clone())
                                }
                            })
                            .collect();
                        Value::Object(processed_properties)
                    } else {
                        value.clone()
                    }
                }
                "items" => {
                    if let Some(items_map) = value.as_object() {
                        process_map(items_map, Some("items"))
                    } else {
                        value.clone()
                    }
                }
                "anyOf" | "allOf" => {
                    if let Some(arr) = value.as_array() {
                        let processed_arr: Vec<Value> = arr
                            .iter()
                            .map(|item| {
                                item.as_object().map_or_else(
                                    || item.clone(),
                                    |obj| process_map(obj, parent_key),
                                )
                            })
                            .collect();
                        Value::Array(processed_arr)
                    } else {
                        value.clone()
                    }
                }
                _ => process_value(value, Some(key.as_str())),
            };

            Some((key.clone(), processed_value))
        })
        .collect();

    Value::Object(filtered_map)
}

#[derive(Clone, Copy)]
enum SignedTextHandling {
    SkipSignedText,
    SignedTextAsThinking,
    SignedTextAsRegularText,
}

pub fn process_response_part(
    part: &Value,
    last_signature: &mut Option<String>,
) -> Option<MessageContent> {
    // For streaming: skip text with signatures (matches Anthropic/OpenAI behavior)
    process_response_part_impl(part, last_signature, SignedTextHandling::SkipSignedText)
}

fn process_response_part_non_streaming(
    part: &Value,
    last_signature: &mut Option<String>,
    has_function_calls: bool,
) -> Option<MessageContent> {
    // For non-streaming: signed text is thinking only if there are function calls
    let handling = if has_function_calls {
        SignedTextHandling::SignedTextAsThinking
    } else {
        SignedTextHandling::SignedTextAsRegularText
    };
    process_response_part_impl(part, last_signature, handling)
}

fn process_response_part_impl(
    part: &Value,
    last_signature: &mut Option<String>,
    signed_text_handling: SignedTextHandling,
) -> Option<MessageContent> {
    let signature = part.get(THOUGHT_SIGNATURE_KEY).and_then(|v| v.as_str());

    if let Some(sig) = signature {
        *last_signature = Some(sig.to_string());
    }

    let text_value = part.get("text");
    if let Some(text) = text_value.and_then(|v| v.as_str()) {
        if text.is_empty() {
            return None;
        }
        match (signature, signed_text_handling) {
            (Some(_), SignedTextHandling::SkipSignedText) => None,
            (Some(sig), SignedTextHandling::SignedTextAsThinking) => {
                Some(MessageContent::thinking(text.to_string(), sig.to_string()))
            }
            _ => Some(MessageContent::text(text.to_string())),
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
                Ok(CallToolRequestParam {
                    name: name.to_string().into(),
                    arguments,
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

    let has_function_calls = parts.iter().any(|p| p.get("functionCall").is_some());

    let mut content = Vec::new();
    let mut last_signature: Option<String> = None;

    for part in parts {
        if let Some(msg_content) =
            process_response_part_non_streaming(part, &mut last_signature, has_function_calls)
        {
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

pub fn response_to_streaming_message<S>(
    mut stream: S,
) -> impl futures::Stream<
    Item = anyhow::Result<(
        Option<Message>,
        Option<crate::providers::base::ProviderUsage>,
    )>,
> + 'static
where
    S: futures::Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    use async_stream::try_stream;
    use futures::StreamExt;

    try_stream! {
        let mut final_usage: Option<crate::providers::base::ProviderUsage> = None;
        let mut last_signature: Option<String> = None;
        let stream_id = Uuid::new_v4().to_string();
        let mut incomplete_data: Option<String> = None;

        while let Some(line_result) = stream.next().await {
            let line = line_result?;

            if line.trim().is_empty() {
                continue;
            }

            let data_part = if line.starts_with("data: ") {
                line.strip_prefix("data: ").unwrap()
            } else if line.starts_with("event:") || line.starts_with("id:") || line.starts_with("retry:") {
                continue;
            } else if incomplete_data.is_some() {
                &line
            } else {
                continue;
            };

            if data_part.trim() == "[DONE]" {
                break;
            }

            let chunk: Value = if let Some(ref mut incomplete) = incomplete_data {
                incomplete.push_str(data_part);
                match serde_json::from_str(incomplete) {
                    Ok(v) => {
                        incomplete_data = None;
                        v
                    }
                    Err(e) => {
                        if e.is_eof() {
                            continue;
                        }
                        tracing::warn!("Failed to parse streaming chunk: {}", e);
                        incomplete_data = None;
                        continue;
                    }
                }
            } else {
                match serde_json::from_str(data_part) {
                    Ok(v) => v,
                    Err(e) => {
                        if e.is_eof() {
                            incomplete_data = Some(data_part.to_string());
                            continue;
                        }
                        tracing::warn!("Failed to parse streaming chunk: {}", e);
                        continue;
                    }
                }
            };

            if let Some(error) = chunk.get("error") {
                let message = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                let status = error
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("UNKNOWN");
                Err(anyhow::anyhow!("Google API error ({}): {}", status, message))?;
            }

            if let Ok(usage) = get_usage(&chunk) {
                if usage.input_tokens.is_some() || usage.output_tokens.is_some() {
                    let model = chunk.get("modelVersion")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    final_usage = Some(crate::providers::base::ProviderUsage::new(model, usage));
                }
            }

            let parts = chunk
                .get("candidates")
                .and_then(|v| v.as_array())
                .and_then(|c| c.first())
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.as_array());

            if let Some(parts) = parts {
                for part in parts {
                    if let Some(content) = process_response_part(part, &mut last_signature) {
                        let message = Message::new(
                            Role::Assistant,
                            chrono::Utc::now().timestamp(),
                            vec![content],
                        ).with_id(stream_id.clone());
                        yield (Some(message), None);
                    }
                }
            }
        }

        if let Some(usage) = final_usage {
            yield (None, Some(usage));
        }
    }
}

#[derive(Serialize)]
struct TextPart<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct SystemInstruction<'a> {
    parts: [TextPart<'a>; 1],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolsWrapper {
    function_declarations: Vec<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GoogleRequest<'a> {
    system_instruction: SystemInstruction<'a>,
    contents: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<ToolsWrapper>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let tools_wrapper = if tools.is_empty() {
        None
    } else {
        Some(ToolsWrapper {
            function_declarations: format_tools(tools),
        })
    };

    let generation_config =
        if model_config.temperature.is_some() || model_config.max_tokens.is_some() {
            Some(GenerationConfig {
                temperature: model_config.temperature.map(|t| t as f64),
                max_output_tokens: model_config.max_tokens,
            })
        } else {
            None
        };

    let request = GoogleRequest {
        system_instruction: SystemInstruction {
            parts: [TextPart { text: system }],
        },
        contents: format_messages(messages),
        tools: tools_wrapper,
        generation_config,
    };

    Ok(serde_json::to_value(request)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::model::{CallToolRequestParam, CallToolResult};
    use rmcp::{model::Content, object};
    use serde_json::json;

    fn set_up_text_message(text: &str, role: Role) -> Message {
        Message::new(role, 0, vec![MessageContent::text(text.to_string())])
    }

    fn set_up_tool_request_message(id: &str, tool_call: CallToolRequestParam) -> Message {
        Message::new(
            Role::User,
            0,
            vec![MessageContent::tool_request(id.to_string(), Ok(tool_call))],
        )
    }

    fn set_up_action_required_message(id: &str, tool_call: CallToolRequestParam) -> Message {
        Message::new(
            Role::User,
            0,
            vec![MessageContent::action_required(
                id.to_string(),
                tool_call.name.to_string().clone(),
                tool_call.arguments.unwrap_or_default().clone(),
                Some("goose would like to call the above tool. Allow? (y/n):".to_string()),
            )],
        )
    }

    fn set_up_tool_response_message(id: &str, tool_response: Vec<Content>) -> Message {
        Message::new(
            Role::Assistant,
            0,
            vec![MessageContent::tool_response(
                id.to_string(),
                Ok(CallToolResult {
                    content: tool_response,
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }),
            )],
        )
    }

    #[test]
    fn test_get_usage() {
        let data = json!({
            "usageMetadata": {
                "promptTokenCount": 1,
                "candidatesTokenCount": 2,
                "totalTokenCount": 3
            }
        });
        let usage = get_usage(&data).unwrap();
        assert_eq!(usage.input_tokens, Some(1));
        assert_eq!(usage.output_tokens, Some(2));
        assert_eq!(usage.total_tokens, Some(3));
    }

    #[test]
    fn test_message_to_google_spec_text_message() {
        let messages = vec![
            set_up_text_message("Hello", Role::User),
            set_up_text_message("World", Role::Assistant),
        ];
        let payload = format_messages(&messages);
        assert_eq!(payload.len(), 2);
        assert_eq!(payload[0]["role"], "user");
        assert_eq!(payload[0]["parts"][0]["text"], "Hello");
        assert_eq!(payload[1]["role"], "model");
        assert_eq!(payload[1]["parts"][0]["text"], "World");
    }

    #[test]
    fn test_message_to_google_spec_tool_request_message() {
        let arguments = json!({
            "param1": "value1"
        });
        let messages = vec![
            set_up_tool_request_message(
                "id",
                CallToolRequestParam {
                    name: "tool_name".into(),
                    arguments: Some(object(arguments.clone())),
                },
            ),
            set_up_action_required_message(
                "id2",
                CallToolRequestParam {
                    name: "tool_name_2".into(),
                    arguments: Some(object(arguments.clone())),
                },
            ),
        ];
        let payload = format_messages(&messages);
        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0]["role"], "user");
        assert_eq!(payload[0]["parts"][0]["functionCall"]["args"], arguments);
    }

    #[test]
    fn test_message_to_google_spec_tool_result_message() {
        let tool_result: Vec<Content> = vec![Content::text("Hello")];
        let messages = vec![set_up_tool_response_message("response_id", tool_result)];
        let payload = format_messages(&messages);
        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0]["role"], "model");
        assert_eq!(
            payload[0]["parts"][0]["functionResponse"]["name"],
            "response_id"
        );
        assert_eq!(
            payload[0]["parts"][0]["functionResponse"]["response"]["content"]["text"],
            "Hello"
        );
    }

    #[test]
    fn test_message_to_google_spec_tool_result_multiple_texts() {
        let tool_result: Vec<Content> = vec![
            Content::text("Hello"),
            Content::text("World"),
            Content::embedded_text("test_uri", "This is a test."),
        ];

        let messages = vec![set_up_tool_response_message("response_id", tool_result)];
        let payload = format_messages(&messages);

        let expected_payload = vec![json!({
            "role": "model",
            "parts": [
                {
                    "functionResponse": {
                        "name": "response_id",
                        "response": {
                            "content": {
                                "text": "Hello\nWorld\nThis is a test."
                            }
                        }
                    }
                }
            ]
        })];

        assert_eq!(payload, expected_payload);
    }

    #[test]
    fn test_tools_to_google_spec_with_valid_tools() {
        let params1 = object!({
            "properties": {
                "param1": {
                    "type": "string",
                    "description": "A parameter",
                    "field_does_not_accept": ["value1", "value2"]
                }
            }
        });
        let params2 = object!({
            "properties": {
                "param2": {
                    "type": "string",
                    "description": "B parameter",
                }
            }
        });
        let params3 = object!({
            "properties": {
                "body": {
                    "description": "Review comment text",
                    "type": "string"
                },
                "comments": {
                    "description": "Line-specific comments array of objects to place comments on pull request changes. Requires path and body. For line comments use line or position. For multi-line comments use start_line and line with optional side parameters.",
                    "type": "array",
                    "items": {
                        "additionalProperties": false,
                        "properties": {
                            "body": {
                                "description": "comment body",
                                "type": "string"
                            },
                            "line": {
                                "anyOf": [
                                    { "type": "number" },
                                    { "type": "null" }
                                ],
                                "description": "line number in the file to comment on. For multi-line comments, the end of the line range"
                            },
                            "path": {
                                "description": "path to the file",
                                "type": "string"
                            },
                            "position": {
                                "anyOf": [
                                    { "type": "number" },
                                    { "type": "null" }
                                ],
                                "description": "position of the comment in the diff"
                            },
                            "side": {
                                "anyOf": [
                                    { "type": "string" },
                                    { "type": "null" }
                                ],
                                "description": "The side of the diff on which the line resides. For multi-line comments, this is the side for the end of the line range. (LEFT or RIGHT)"
                            },
                            "start_line": {
                                "anyOf": [
                                    { "type": "number" },
                                    { "type": "null" }
                                ],
                                "description": "The first line of the range to which the comment refers. Required for multi-line comments."
                            },
                            "start_side": {
                                "anyOf": [
                                    { "type": "string" },
                                    { "type": "null" }
                                ],
                                "description": "The side of the diff on which the start line resides for multi-line comments. (LEFT or RIGHT)"
                            }
                        },
                        "required": ["path", "body", "position", "line", "side", "start_line", "start_side"],
                        "type": "object"
                    }
                },
                "commitId": {
                    "description": "SHA of commit to review",
                    "type": "string"
                },
                "event": {
                    "description": "Review action to perform",
                    "enum": ["APPROVE", "REQUEST_CHANGES", "COMMENT"],
                    "type": "string"
                },
                "owner": {
                    "description": "Repository owner",
                    "type": "string"
                },
                "pullNumber": {
                    "description": "Pull request number",
                    "type": "number"
                }
            }
        });
        let tools = vec![
            Tool::new("tool1", "description1", params1),
            Tool::new("tool2", "description2", params2),
            Tool::new("tool3", "description3", params3),
        ];
        let result = format_tools(&tools);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0]["name"], "tool1");
        assert_eq!(result[0]["description"], "description1");
        assert_eq!(
            result[0]["parameters"]["properties"],
            json!({"param1": json!({
                "type": "string",
                "description": "A parameter"
            })})
        );
        assert_eq!(result[1]["name"], "tool2");
        assert_eq!(result[1]["description"], "description2");
        assert_eq!(
            result[1]["parameters"]["properties"],
            json!({"param2": json!({
                "type": "string",
                "description": "B parameter"
            })})
        );

        assert_eq!(result[2]["name"], "tool3");
        assert_eq!(
            result[2]["parameters"]["properties"],
            json!(

            {
                        "body": {
                            "description": "Review comment text",
                            "type": "string"
                        },
                        "comments": {
                            "description": "Line-specific comments array of objects to place comments on pull request changes. Requires path and body. For line comments use line or position. For multi-line comments use start_line and line with optional side parameters.",
                            "type": "array",
                            "items": {
                                "properties": {
                                    "body": {
                                        "description": "comment body",
                                        "type": "string"
                                    },
                                    "line": {
                                        "anyOf": [
                                            { "type": "number" },
                                            { "type": "null" }
                                        ],
                                        "description": "line number in the file to comment on. For multi-line comments, the end of the line range"
                                    },
                                    "path": {
                                        "description": "path to the file",
                                        "type": "string"
                                    },
                                    "position": {
                                        "anyOf": [
                                            { "type": "number" },
                                            { "type": "null" }
                                        ],
                                        "description": "position of the comment in the diff"
                                    },
                                    "side": {
                                        "anyOf": [
                                            { "type": "string" },
                                            { "type": "null" }
                                        ],
                                        "description": "The side of the diff on which the line resides. For multi-line comments, this is the side for the end of the line range. (LEFT or RIGHT)"
                                    },
                                    "start_line": {
                                        "anyOf": [
                                            { "type": "number" },
                                            { "type": "null" }
                                        ],
                                        "description": "The first line of the range to which the comment refers. Required for multi-line comments."
                                    },
                                    "start_side": {
                                        "anyOf": [
                                            { "type": "string" },
                                            { "type": "null" }
                                        ],
                                        "description": "The side of the diff on which the start line resides for multi-line comments. (LEFT or RIGHT)"
                                    }
                                },
                                "required": ["path", "body", "position", "line", "side", "start_line", "start_side"],
                                "type": "object"
                            }
                        },
                        "commitId": {
                            "description": "SHA of commit to review",
                            "type": "string"
                        },
                        "event": {
                            "description": "Review action to perform",
                            "enum": ["APPROVE", "REQUEST_CHANGES", "COMMENT"],
                            "type": "string"
                        },
                        "owner": {
                            "description": "Repository owner",
                            "type": "string"
                        },
                        "pullNumber": {
                            "description": "Pull request number",
                            "type": "number"
                        }
                    }
                    )
        );
    }

    #[test]
    fn test_tools_to_google_spec_with_empty_properties() {
        let tools = vec![Tool::new(
            "tool1".to_string(),
            "description1".to_string(),
            object!({
                "properties": {}
            }),
        )];
        let result = format_tools(&tools);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "tool1");
        assert_eq!(result[0]["description"], "description1");
        assert!(result[0]["parameters"].get("properties").is_none());
    }

    #[test]
    fn test_response_to_message_with_no_candidates() {
        let response = json!({});
        let message = response_to_message(response).unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert!(message.content.is_empty());
    }

    #[test]
    fn test_response_to_message_with_text_part() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "Hello, world!"
                    }]
                }
            }]
        });
        let message = response_to_message(response).unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content.len(), 1);
        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello, world!");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_response_to_message_with_invalid_function_name() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "invalid name!",
                            "args": {}
                        }
                    }]
                }
            }]
        });
        let message = response_to_message(response).unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content.len(), 1);
        if let Err(error) = &message.content[0].as_tool_request().unwrap().tool_call {
            assert!(matches!(
                error,
                ErrorData {
                    code: ErrorCode::INVALID_REQUEST,
                    message: _,
                    data: None,
                }
            ));
        } else {
            panic!("Expected tool request error");
        }
    }

    #[test]
    fn test_response_to_message_with_valid_function_call() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "valid_name",
                            "args": {
                                "param": "value"
                            }
                        }
                    }]
                }
            }]
        });
        let message = response_to_message(response).unwrap();
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.content.len(), 1);
        if let Ok(tool_call) = &message.content[0].as_tool_request().unwrap().tool_call {
            assert_eq!(tool_call.name, "valid_name");
            assert_eq!(
                tool_call
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("param"))
                    .and_then(|v| v.as_str()),
                Some("value")
            );
        } else {
            panic!("Expected valid tool request");
        }
    }

    #[test]
    fn test_response_to_message_with_empty_content() {
        let tool_result: Vec<Content> = Vec::new();

        let messages = vec![set_up_tool_response_message("response_id", tool_result)];
        let payload = format_messages(&messages);

        let expected_payload = vec![json!({
            "role": "model",
            "parts": [
                {
                    "functionResponse": {
                        "name": "response_id",
                        "response": {
                            "content": {
                                "text": "Tool call is done."
                            }
                        }
                    }
                }
            ]
        })];

        assert_eq!(payload, expected_payload);
    }

    #[test]
    fn test_tools_with_nullable_types_converted_to_single_type() {
        // Test that type arrays like ["string", "null"] are converted to single types
        let params = object!({
            "properties": {
                "nullable_field": {
                    "type": ["string", "null"],
                    "description": "A nullable string field"
                },
                "regular_field": {
                    "type": "number",
                    "description": "A regular number field"
                }
            }
        });
        let tools = vec![Tool::new("test_tool", "test description", params)];
        let result = format_tools(&tools);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "test_tool");

        // Verify that the type array was converted to a single string type
        let nullable_field = &result[0]["parameters"]["properties"]["nullable_field"];
        assert_eq!(nullable_field["type"], "string");
        assert_eq!(nullable_field["description"], "A nullable string field");

        // Verify that regular types are unchanged
        let regular_field = &result[0]["parameters"]["properties"]["regular_field"];
        assert_eq!(regular_field["type"], "number");
        assert_eq!(regular_field["description"], "A regular number field");
    }

    fn google_response(parts: Vec<Value>) -> Value {
        json!({"candidates": [{"content": {"role": "model", "parts": parts}}]})
    }

    fn tool_result(text: &str) -> CallToolResult {
        CallToolResult {
            content: vec![Content::text(text)],
            structured_content: None,
            is_error: Some(false),
            meta: None,
        }
    }

    #[test]
    fn test_thought_signature_roundtrip() {
        const SIG: &str = "thought_sig_abc";

        let response_with_tools = google_response(vec![
            json!({"text": "Let me think...", "thoughtSignature": SIG}),
            json!({"functionCall": {"name": "shell", "args": {"cmd": "ls"}}, "thoughtSignature": SIG}),
            json!({"functionCall": {"name": "read", "args": {}}}),
        ]);

        let native = response_to_message(response_with_tools).unwrap();
        assert_eq!(native.content.len(), 3, "Expected thinking + 2 tool calls");

        let thinking = native.content[0]
            .as_thinking()
            .expect("Text with function calls should be Thinking");
        assert_eq!(thinking.signature, SIG);

        let req1 = native.content[1]
            .as_tool_request()
            .expect("Second part should be ToolRequest");
        let req2 = native.content[2]
            .as_tool_request()
            .expect("Third part should be ToolRequest");
        assert_eq!(get_thought_signature(&req1.metadata), Some(SIG));
        assert_eq!(
            get_thought_signature(&req2.metadata),
            Some(SIG),
            "Should inherit"
        );

        let tool_response = Message::user().with_tool_response_with_metadata(
            req1.id.clone(),
            Ok(tool_result("output")),
            req1.metadata.as_ref(),
        );
        let google_out = format_messages(&[native, tool_response]);
        assert_eq!(google_out[0]["parts"][0]["thoughtSignature"], SIG);
        assert_eq!(google_out[1]["parts"][0]["thoughtSignature"], SIG);

        // Text-only response WITH signature but WITHOUT function calls should be regular text
        // (per original behavior: thinking is only when reasoning before tool calls)
        let final_response_with_sig =
            google_response(vec![json!({"text": "Done!", "thoughtSignature": SIG})]);
        let final_native_with_sig = response_to_message(final_response_with_sig).unwrap();
        assert!(
            final_native_with_sig.content[0].as_text().is_some(),
            "Text with signature but no function calls should be regular text (final response)"
        );

        let final_response_no_sig = google_response(vec![json!({"text": "Done!"})]);
        let final_native_no_sig = response_to_message(final_response_no_sig).unwrap();
        assert!(
            final_native_no_sig.content[0].as_text().is_some(),
            "Text without signature is regular text"
        );
    }

    const GOOGLE_TEXT_STREAM: &str = concat!(
        r#"data: {"candidates": [{"content": {"role": "model", "#,
        r#""parts": [{"text": "Hello"}]}}]}"#,
        "\n",
        r#"data: {"candidates": [{"content": {"role": "model", "#,
        r#""parts": [{"text": " world"}]}}]}"#,
        "\n",
        r#"data: {"candidates": [{"content": {"role": "model", "#,
        r#""parts": [{"text": "!"}]}}], "#,
        r#""usageMetadata": {"promptTokenCount": 10, "#,
        r#""candidatesTokenCount": 3, "totalTokenCount": 13}}"#
    );

    const GOOGLE_FUNCTION_STREAM: &str = concat!(
        r#"data: {"candidates": [{"content": {"role": "model", "#,
        r#""parts": [{"functionCall": {"name": "test_tool", "#,
        r#""args": {"param": "value"}}}]}}], "#,
        r#""usageMetadata": {"promptTokenCount": 5, "#,
        r#""candidatesTokenCount": 2, "totalTokenCount": 7}}"#
    );

    #[tokio::test]
    async fn test_streaming_text_response() {
        use futures::StreamExt;

        let lines: Vec<Result<String, anyhow::Error>> = GOOGLE_TEXT_STREAM
            .lines()
            .map(|l| Ok(l.to_string()))
            .collect();
        let stream = Box::pin(futures::stream::iter(lines));
        let mut message_stream = std::pin::pin!(response_to_streaming_message(stream));

        let mut text_parts = Vec::new();
        let mut message_ids: Vec<Option<String>> = Vec::new();
        let mut final_usage = None;

        while let Some(result) = message_stream.next().await {
            let (message, usage) = result.unwrap();
            if let Some(msg) = message {
                message_ids.push(msg.id.clone());
                if let Some(MessageContent::Text(text)) = msg.content.first() {
                    text_parts.push(text.text.clone());
                }
            }
            if usage.is_some() {
                final_usage = usage;
            }
        }

        assert_eq!(text_parts, vec!["Hello", " world", "!"]);
        let usage = final_usage.unwrap();
        assert_eq!(usage.usage.input_tokens, Some(10));
        assert_eq!(usage.usage.output_tokens, Some(3));

        // Verify all streaming messages have consistent IDs for UI aggregation
        assert!(
            message_ids.iter().all(|id| id.is_some()),
            "All streaming messages should have an ID"
        );
        let first_id = message_ids.first().unwrap();
        assert!(
            message_ids.iter().all(|id| id == first_id),
            "All streaming messages should have the same ID"
        );
    }

    #[tokio::test]
    async fn test_streaming_function_call() {
        use futures::StreamExt;

        let lines: Vec<Result<String, anyhow::Error>> = GOOGLE_FUNCTION_STREAM
            .lines()
            .map(|l| Ok(l.to_string()))
            .collect();
        let stream = Box::pin(futures::stream::iter(lines));
        let mut message_stream = std::pin::pin!(response_to_streaming_message(stream));

        let mut tool_calls = Vec::new();

        while let Some(result) = message_stream.next().await {
            let (message, _usage) = result.unwrap();
            if let Some(msg) = message {
                if let Some(MessageContent::ToolRequest(req)) = msg.content.first() {
                    if let Ok(tool_call) = &req.tool_call {
                        tool_calls.push(tool_call.name.to_string());
                    }
                }
            }
        }

        assert_eq!(tool_calls, vec!["test_tool"]);
    }

    #[tokio::test]
    async fn test_streaming_error_response() {
        use futures::StreamExt;

        let error_stream = concat!(
            r#"data: {"error": {"code": 400, "#,
            r#""message": "Invalid request", "status": "INVALID_ARGUMENT"}}"#
        );
        let lines: Vec<Result<String, anyhow::Error>> =
            error_stream.lines().map(|l| Ok(l.to_string())).collect();
        let stream = Box::pin(futures::stream::iter(lines));
        let mut message_stream = std::pin::pin!(response_to_streaming_message(stream));

        let result = message_stream.next().await;
        assert!(result.is_some());
        let err = result.unwrap();
        assert!(err.is_err());
        let error_msg = err.unwrap_err().to_string();
        assert!(error_msg.contains("INVALID_ARGUMENT"));
        assert!(error_msg.contains("Invalid request"));
    }

    #[tokio::test]
    async fn test_streaming_with_sse_event_lines() {
        use futures::StreamExt;

        // SSE format can include event: lines which should be skipped
        let sse_stream = r#"event: message
data: {"candidates": [{"content": {"role": "model", "parts": [{"text": "Hello"}]}}]}

event: message
data: {"candidates": [{"content": {"role": "model", "parts": [{"text": " world"}]}}]}

data: [DONE]"#;
        let lines: Vec<Result<String, anyhow::Error>> =
            sse_stream.lines().map(|l| Ok(l.to_string())).collect();
        let stream = Box::pin(futures::stream::iter(lines));
        let mut message_stream = std::pin::pin!(response_to_streaming_message(stream));

        let mut text_parts = Vec::new();

        while let Some(result) = message_stream.next().await {
            let (message, _usage) = result.unwrap();
            if let Some(msg) = message {
                if let Some(MessageContent::Text(text)) = msg.content.first() {
                    text_parts.push(text.text.clone());
                }
            }
        }

        assert_eq!(text_parts, vec!["Hello", " world"]);
    }

    #[tokio::test]
    async fn test_streaming_handles_done_signal() {
        use futures::StreamExt;

        let stream_with_done = concat!(
            r#"data: {"candidates": [{"content": {"role": "model", "#,
            r#""parts": [{"text": "Complete"}]}}]}"#,
            "\n",
            "data: [DONE]\n",
            r#"data: {"candidates": [{"content": {"role": "model", "#,
            r#""parts": [{"text": "Should not appear"}]}}]}"#
        );
        let lines: Vec<Result<String, anyhow::Error>> = stream_with_done
            .lines()
            .map(|l| Ok(l.to_string()))
            .collect();
        let stream = Box::pin(futures::stream::iter(lines));
        let mut message_stream = std::pin::pin!(response_to_streaming_message(stream));

        let mut text_parts = Vec::new();

        while let Some(result) = message_stream.next().await {
            let (message, _usage) = result.unwrap();
            if let Some(msg) = message {
                if let Some(MessageContent::Text(text)) = msg.content.first() {
                    text_parts.push(text.text.clone());
                }
            }
        }

        // Only "Complete" should be captured, stream should stop at [DONE]
        assert_eq!(text_parts, vec!["Complete"]);
    }
}
