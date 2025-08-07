use crate::model::ModelConfig;
use crate::providers::base::Usage;
use crate::providers::errors::ProviderError;
use crate::providers::utils::{is_valid_function_name, sanitize_function_name};
use anyhow::Result;
use mcp_core::tool::ToolCall;
use rand::{distributions::Alphanumeric, Rng};
use rmcp::model::{AnnotateAble, RawContent, Role, Tool};

use crate::conversation::message::{Message, MessageContent};
use serde_json::{json, Map, Value};
use std::ops::Deref;

/// Convert internal Message format to Google's API message specification
pub fn format_messages(messages: &[Message]) -> Vec<Value> {
    messages
        .iter()
        .filter(|message| {
            message
                .content
                .iter()
                .any(|content| !matches!(content, MessageContent::ToolConfirmationRequest(_)))
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
                            if tool_call.arguments.is_object()
                                && !tool_call.arguments.as_object().unwrap().is_empty()
                            {
                                function_call_part
                                    .insert("args".to_string(), tool_call.arguments.clone());
                            }
                            parts.push(json!({
                                "functionCall": function_call_part
                            }));
                        }
                        Err(e) => {
                            parts.push(json!({"text":format!("Error: {}", e)}));
                        }
                    },
                    MessageContent::ToolResponse(response) => {
                        match &response.tool_result {
                            Ok(contents) => {
                                // Send only contents with no audience or with Assistant in the audience
                                let abridged: Vec<_> = contents
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
                                parts.push(json!({
                                    "functionResponse": {
                                        "name": response.id,
                                        "response": {"content": {"text": text}},
                                    }}
                                ));
                            }
                            Err(e) => {
                                parts.push(json!({"text":format!("Error: {}", e)}));
                            }
                        }
                    }

                    _ => {}
                }
            }
            json!({"role": role, "parts": parts})
        })
        .collect()
}

/// Convert internal Tool format to Google's API tool specification
pub fn format_tools(tools: &[Tool]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            let mut parameters = Map::new();
            parameters.insert("name".to_string(), json!(tool.name));
            parameters.insert("description".to_string(), json!(tool.description));
            let tool_input_schema = &tool.input_schema;
            // Only add the parameters key if the tool schema has non-empty properties.
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

/// Get the accepted keys for a given parent key in the JSON schema.
fn get_accepted_keys(parent_key: Option<&str>) -> Vec<&str> {
    match parent_key {
        Some("properties") => vec![
            "anyOf",
            "allOf",
            "type",
            // "format", // Google's APIs don't support this well
            "description",
            "nullable",
            "enum",
            "properties",
            "required",
            "items",
        ],
        Some("items") => vec!["type", "properties", "items", "required"],
        // This is the top-level schema.
        _ => vec!["type", "properties", "required", "anyOf", "allOf"],
    }
}

/// Process a JSON map to filter out unsupported attributes, mirroring the logic
/// from the official Google Gemini CLI.
/// See: https://github.com/google-gemini/gemini-cli/blob/8a6509ffeba271a8e7ccb83066a9a31a5d72a647/packages/core/src/tools/tool-registry.ts#L356
fn process_map(map: &Map<String, Value>, parent_key: Option<&str>) -> Value {
    let accepted_keys = get_accepted_keys(parent_key);
    let filtered_map: Map<String, Value> = map
        .iter()
        .filter_map(|(key, value)| {
            if !accepted_keys.contains(&key.as_str()) {
                return None; // Skip if key is not accepted
            }

            match key.as_str() {
                "properties" => {
                    // Process each property within the properties object
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
                        Some((key.clone(), Value::Object(processed_properties)))
                    } else {
                        None
                    }
                }
                "items" => {
                    // If it's a nested structure, recurse if it's an object.
                    value.as_object().map(|nested_map| {
                        (key.clone(), process_map(nested_map, Some(key.as_str())))
                    })
                }
                _ => {
                    // For other accepted keys, just clone the value.
                    Some((key.clone(), value.clone()))
                }
            }
        })
        .collect();

    Value::Object(filtered_map)
}

/// Convert Google's API response to internal Message format
pub fn response_to_message(response: Value) -> Result<Message> {
    let mut content = Vec::new();
    let binding = vec![];
    let candidates: &Vec<Value> = response
        .get("candidates")
        .and_then(|v| v.as_array())
        .unwrap_or(&binding);
    let candidate = candidates.first();
    let role = Role::Assistant;
    let created = chrono::Utc::now().timestamp();
    if candidate.is_none() {
        return Ok(Message::new(role, created, content));
    }
    let candidate = candidate.unwrap();
    let parts = candidate
        .get("content")
        .and_then(|content| content.get("parts"))
        .and_then(|parts| parts.as_array())
        .unwrap_or(&binding);

    for part in parts {
        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
            content.push(MessageContent::text(text.to_string()));
        } else if let Some(function_call) = part.get("functionCall") {
            let id: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(8)
                .map(char::from)
                .collect();
            let name = function_call["name"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            if !is_valid_function_name(&name) {
                let error = mcp_core::ToolError::NotFound(format!(
                    "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                    name
                ));
                content.push(MessageContent::tool_request(id, Err(error)));
            } else {
                let parameters = function_call.get("args");
                if let Some(params) = parameters {
                    content.push(MessageContent::tool_request(
                        id,
                        Ok(ToolCall::new(&name, params.clone())),
                    ));
                }
            }
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

/// Create a complete request payload for Google's API
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let mut payload = Map::new();
    payload.insert(
        "system_instruction".to_string(),
        json!({"parts": [{"text": system}]}),
    );
    payload.insert("contents".to_string(), json!(format_messages(messages)));
    if !tools.is_empty() {
        payload.insert(
            "tools".to_string(),
            json!({"functionDeclarations": format_tools(tools)}),
        );
    }
    let mut generation_config = Map::new();
    if let Some(temp) = model_config.temperature {
        generation_config.insert("temperature".to_string(), json!(temp as f64));
    }
    if let Some(tokens) = model_config.max_tokens {
        generation_config.insert("maxOutputTokens".to_string(), json!(tokens));
    }
    if !generation_config.is_empty() {
        payload.insert("generationConfig".to_string(), json!(generation_config));
    }

    Ok(json!(payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::{model::Content, object};
    use serde_json::json;

    fn set_up_text_message(text: &str, role: Role) -> Message {
        Message::new(role, 0, vec![MessageContent::text(text.to_string())])
    }

    fn set_up_tool_request_message(id: &str, tool_call: ToolCall) -> Message {
        Message::new(
            Role::User,
            0,
            vec![MessageContent::tool_request(id.to_string(), Ok(tool_call))],
        )
    }

    fn set_up_tool_confirmation_message(id: &str, tool_call: ToolCall) -> Message {
        Message::new(
            Role::User,
            0,
            vec![MessageContent::tool_confirmation_request(
                id.to_string(),
                tool_call.name.clone(),
                tool_call.arguments.clone(),
                Some("Goose would like to call the above tool. Allow? (y/n):".to_string()),
            )],
        )
    }

    fn set_up_tool_response_message(id: &str, tool_response: Vec<Content>) -> Message {
        Message::new(
            Role::Assistant,
            0,
            vec![MessageContent::tool_response(
                id.to_string(),
                Ok(tool_response),
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
            set_up_tool_request_message("id", ToolCall::new("tool_name", arguments.clone())),
            set_up_tool_confirmation_message(
                "id2",
                ToolCall::new("tool_name_2", arguments.clone()),
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
            assert!(matches!(error, mcp_core::ToolError::NotFound(_)));
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
            assert_eq!(tool_call.arguments["param"], "value");
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
}
