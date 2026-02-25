use crate::model::ModelConfig;
use crate::providers::base::Usage;
use crate::providers::errors::ProviderError;
use crate::providers::utils::{is_valid_function_name, sanitize_function_name};
use anyhow::Result;
use rmcp::model::{
    object, AnnotateAble, CallToolRequestParams, ErrorCode, ErrorData, RawContent, Role, Tool,
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

    let last_assistant_idx = filtered
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role != Role::User)
        .map(|(i, _)| i)
        .next_back();

    filtered
        .iter()
        .enumerate()
        .map(|(idx, message)| {
            let role = if message.role == Role::User {
                "user"
            } else {
                "model"
            };
            let include_signature = match last_assistant_idx {
                Some(last_idx) => idx >= last_idx,
                None => false,
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

                            if include_signature {
                                if let Some(signature) = get_thought_signature(&request.metadata) {
                                    part.insert(
                                        THOUGHT_SIGNATURE_KEY.to_string(),
                                        json!(signature),
                                    );
                                }
                            }

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
                            let mut part = Map::new();
                            let mut function_response = Map::new();
                            function_response.insert("name".to_string(), json!(response.id));
                            function_response
                                .insert("response".to_string(), json!({"content": {"text": text}}));
                            part.insert("functionResponse".to_string(), json!(function_response));
                            if include_signature {
                                if let Some(signature) = get_thought_signature(&response.metadata) {
                                    part.insert(
                                        THOUGHT_SIGNATURE_KEY.to_string(),
                                        json!(signature),
                                    );
                                }
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
                            part.insert("functionResponse".to_string(), json!(function_response));
                            if include_signature {
                                if let Some(signature) = get_thought_signature(&response.metadata) {
                                    part.insert(
                                        THOUGHT_SIGNATURE_KEY.to_string(),
                                        json!(signature),
                                    );
                                }
                            }
                            parts.push(json!(part));
                        }
                    },
                    MessageContent::Thinking(thinking) => {
                        let mut part = Map::new();
                        part.insert("text".to_string(), json!(thinking.thinking));
                        if include_signature {
                            part.insert("thoughtSignature".to_string(), json!(thinking.signature));
                        }
                        parts.push(json!(part));
                    }
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

#[derive(Clone, Copy)]
enum SignedTextHandling {
    SignedTextAsThinking,
    SignedTextAsRegularText,
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
                Ok(CallToolRequestParams {
                    meta: None,
                    task: None,
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
                    // Always emit text as regular text during streaming — we can't
                    // know yet whether function calls will follow.
                    if let Some(content) = process_response_part_impl(part, &mut last_signature, SignedTextHandling::SignedTextAsRegularText) {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_config: Option<ThinkingConfig>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum ThinkingLevel {
    Low,
    High,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ThinkingConfig {
    thinking_level: ThinkingLevel,
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

fn get_thinking_config(model_config: &ModelConfig) -> Option<ThinkingConfig> {
    if !model_config
        .model_name
        .to_lowercase()
        .starts_with("gemini-3")
    {
        return None;
    }

    let thinking_level_str = model_config
        .request_params
        .as_ref()
        .and_then(|params| params.get("thinking_level"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase())
        .or_else(|| {
            crate::config::Config::global()
                .get_param::<String>("gemini3_thinking_level")
                .ok()
                .map(|s| s.to_lowercase())
        })
        .unwrap_or_else(|| "low".to_string());

    let thinking_level = match thinking_level_str.as_str() {
        "high" => ThinkingLevel::High,
        "low" => ThinkingLevel::Low,
        invalid => {
            tracing::warn!(
                "Invalid thinking level '{}' for model '{}'. Valid levels: low, high. Using 'low'.",
                invalid,
                model_config.model_name,
            );
            ThinkingLevel::Low
        }
    };

    Some(ThinkingConfig { thinking_level })
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

    let thinking_config = get_thinking_config(model_config);

    let generation_config = Some(GenerationConfig {
        temperature: model_config.temperature.map(|t| t as f64),
        max_output_tokens: Some(model_config.max_output_tokens()),
        thinking_config,
    });

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
    use rmcp::model::{CallToolRequestParams, CallToolResult};
    use rmcp::{model::Content, object};
    use serde_json::json;

    fn set_up_text_message(text: &str, role: Role) -> Message {
        Message::new(role, 0, vec![MessageContent::text(text.to_string())])
    }

    fn set_up_tool_request_message(id: &str, tool_call: CallToolRequestParams) -> Message {
        Message::new(
            Role::User,
            0,
            vec![MessageContent::tool_request(id.to_string(), Ok(tool_call))],
        )
    }

    fn set_up_action_required_message(id: &str, tool_call: CallToolRequestParams) -> Message {
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
    fn test_message_to_google_spec_image_message() {
        use rmcp::model::{AnnotateAble, RawImageContent};

        let image = RawImageContent {
            mime_type: "image/png".to_string(),
            data: "base64encodeddata".to_string(),
            meta: None,
        };
        let messages = vec![Message::new(
            Role::User,
            0,
            vec![
                MessageContent::text("What is in this image?".to_string()),
                MessageContent::Image(image.no_annotation()),
            ],
        )];
        let payload = format_messages(&messages);

        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0]["role"], "user");
        assert_eq!(payload[0]["parts"][0]["text"], "What is in this image?");
        assert_eq!(
            payload[0]["parts"][1]["inline_data"]["mime_type"],
            "image/png"
        );
        assert_eq!(
            payload[0]["parts"][1]["inline_data"]["data"],
            "base64encodeddata"
        );
    }

    #[test]
    fn test_message_to_google_spec_tool_request_message() {
        let arguments = json!({
            "param1": "value1"
        });
        let messages = vec![
            set_up_tool_request_message(
                "id",
                CallToolRequestParams {
                    meta: None,
                    task: None,
                    name: "tool_name".into(),
                    arguments: Some(object(arguments.clone())),
                },
            ),
            set_up_action_required_message(
                "id2",
                CallToolRequestParams {
                    meta: None,
                    task: None,
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
        let params = object!({
            "properties": {
                "param1": {
                    "type": "string",
                    "description": "A parameter"
                }
            }
        });
        let tools = vec![Tool::new("tool1", "description1", params.clone())];
        let result = format_tools(&tools);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "tool1");
        assert_eq!(result[0]["description"], "description1");
        assert!(result[0].get("parametersJsonSchema").is_some());
        assert!(result[0].get("parameters").is_none());
        assert_eq!(result[0]["parametersJsonSchema"], json!(params));
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
        assert!(result[0].get("parametersJsonSchema").is_none());
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
    fn test_tools_uses_parameters_json_schema() {
        let params = object!({
            "properties": {
                "field": {
                    "type": ["string", "null"],
                    "description": "A field"
                }
            }
        });
        let tools = vec![Tool::new("test_tool", "test description", params.clone())];
        let result = format_tools(&tools);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "test_tool");
        assert!(result[0].get("parametersJsonSchema").is_some());
        assert_eq!(result[0]["parametersJsonSchema"], json!(params));
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
        let google_out = format_messages(&[native.clone(), tool_response.clone()]);
        assert_eq!(google_out[0]["parts"][0]["thoughtSignature"], SIG);
        assert_eq!(google_out[1]["parts"][0]["thoughtSignature"], SIG);

        let second_assistant =
            Message::assistant().with_thinking("More thinking".to_string(), "sig_456".to_string());
        let google_multi = format_messages(&[native, tool_response, second_assistant]);
        assert!(google_multi[0]["parts"][0]
            .get("thoughtSignature")
            .is_none());
        assert!(google_multi[1]["parts"][0]
            .get("thoughtSignature")
            .is_none());
        assert_eq!(google_multi[2]["parts"][0]["thoughtSignature"], "sig_456");

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
    async fn test_streaming_with_thought_signature() {
        use futures::StreamExt;

        async fn collect_streaming_text(raw: &str) -> (String, usize) {
            let lines: Vec<Result<String, anyhow::Error>> =
                raw.lines().map(|l| Ok(l.to_string())).collect();
            let stream = Box::pin(futures::stream::iter(lines));
            let mut msg_stream = std::pin::pin!(response_to_streaming_message(stream));
            let mut text = String::new();
            let mut thinking = 0usize;
            while let Some(Ok((message, _))) = msg_stream.next().await {
                if let Some(msg) = message {
                    for c in &msg.content {
                        match c {
                            MessageContent::Text(t) => text.push_str(&t.text),
                            MessageContent::Thinking(_) => thinking += 1,
                            _ => {}
                        }
                    }
                }
            }
            (text, thinking)
        }

        // First chunk signed
        let (text, thinking) = collect_streaming_text(concat!(
            r#"data: {"candidates": [{"content": {"role": "model", "#,
            r#""parts": [{"text": "Hello", "thoughtSignature": "sig1"}]}}], "#,
            r#""modelVersion": "gemini-3-flash-preview"}"#,
            "\n",
            r#"data: {"candidates": [{"content": {"role": "model", "#,
            r#""parts": [{"text": " world"}]}}], "modelVersion": "gemini-3-flash-preview"}"#
        ))
        .await;
        assert_eq!(thinking, 0);
        assert_eq!(text, "Hello world");

        // Last chunk signed (the reported truncation bug)
        let (text, thinking) = collect_streaming_text(concat!(
            r#"data: {"candidates": [{"content": {"role": "model", "#,
            r#""parts": [{"text": "SECURITY.md: Project"}]}}], "#,
            r#""modelVersion": "gemini-3-flash-preview"}"#,
            "\n",
            r#"data: {"candidates": [{"content": {"role": "model", "#,
            r#""parts": [{"text": " policies.\n\nRead it?", "thoughtSignature": "sig2"}]}}], "#,
            r#""modelVersion": "gemini-3-flash-preview"}"#
        ))
        .await;
        assert_eq!(thinking, 0);
        assert_eq!(text, "SECURITY.md: Project policies.\n\nRead it?");

        // Intermediate chunk signed
        let (text, thinking) = collect_streaming_text(concat!(
            r#"data: {"candidates": [{"content": {"role": "model", "#,
            r#""parts": [{"text": "one "}]}}], "modelVersion": "gemini-3-flash-preview"}"#,
            "\n",
            r#"data: {"candidates": [{"content": {"role": "model", "#,
            r#""parts": [{"text": "two ", "thoughtSignature": "sig3"}]}}], "modelVersion": "gemini-3-flash-preview"}"#,
            "\n",
            r#"data: {"candidates": [{"content": {"role": "model", "#,
            r#""parts": [{"text": "three"}]}}], "modelVersion": "gemini-3-flash-preview"}"#
        ))
        .await;
        assert_eq!(thinking, 0);
        assert_eq!(text, "one two three");
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

    #[test]
    fn test_format_tools_uses_parameters_json_schema() {
        let tool = Tool::new(
            "test_tool",
            "Test tool with $ref",
            object!({
                "type": "object",
                "$defs": {
                    "MyType": { "type": "string", "description": "A custom type" }
                },
                "properties": {
                    "field": { "$ref": "#/$defs/MyType" }
                }
            }),
        );

        let result = format_tools(&[tool]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "test_tool");
        assert!(result[0].get("parametersJsonSchema").is_some());
        assert!(result[0].get("parameters").is_none());

        let schema = &result[0]["parametersJsonSchema"];
        assert_eq!(schema["properties"]["field"]["$ref"], "#/$defs/MyType");
        assert!(schema.get("$defs").is_some());
    }

    #[test]
    fn test_get_thinking_config() {
        use crate::model::ModelConfig;

        // Test 1: Gemini 3 model defaults to low thinking level
        let config = ModelConfig::new("gemini-3-pro").unwrap();
        let result = get_thinking_config(&config);
        assert!(result.is_some());
        let thinking_config = result.unwrap();
        assert!(matches!(thinking_config.thinking_level, ThinkingLevel::Low));

        // Test 2: Case-insensitive model detection
        let config = ModelConfig::new("Gemini-3-Flash").unwrap();
        let result = get_thinking_config(&config);
        assert!(result.is_some());

        // Test 3: Non-Gemini 3 model returns None
        let config = ModelConfig::new("gpt-4o").unwrap();
        let result = get_thinking_config(&config);
        assert!(result.is_none());
    }
}
