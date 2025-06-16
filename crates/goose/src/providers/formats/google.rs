use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::Usage;
use crate::providers::errors::ProviderError;
use crate::providers::utils::{is_valid_function_name, sanitize_function_name};
use anyhow::Result;
use mcp_core::content::Content;
use mcp_core::role::Role;
use mcp_core::tool::{Tool, ToolCall};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{json, Map, Value};

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
                                    .map(|content| content.unannotated())
                                    .collect();

                                let mut tool_content = Vec::new();
                                for content in abridged {
                                    match content {
                                        Content::Image(image) => {
                                            parts.push(json!({
                                                "inline_data": {
                                                    "mime_type": image.mime_type,
                                                    "data": image.data,
                                                }
                                            }));
                                        }
                                        _ => {
                                            tool_content.push(content);
                                        }
                                    }
                                }
                                let mut text = tool_content
                                    .iter()
                                    .filter_map(|c| match c {
                                        Content::Text(t) => Some(t.text.clone()),
                                        Content::Resource(r) => Some(r.get_text()),
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
            let tool_input_schema = tool.input_schema.as_object().unwrap();
            let tool_input_schema_properties = tool_input_schema
                .get("properties")
                .unwrap_or(&json!({}))
                .as_object()
                .unwrap()
                .clone();
            if !tool_input_schema_properties.is_empty() {
                let accepted_tool_schema_attributes = vec![
                    "type".to_string(),
                    "format".to_string(),
                    "description".to_string(),
                    "nullable".to_string(),
                    "enum".to_string(),
                    "maxItems".to_string(),
                    "minItems".to_string(),
                    "properties".to_string(),
                    "required".to_string(),
                    "items".to_string(),
                ];
                parameters.insert(
                    "parameters".to_string(),
                    json!(process_map(
                        tool_input_schema,
                        &accepted_tool_schema_attributes,
                        None
                    )),
                );
            }
            json!(parameters)
        })
        .collect()
}

/// Process a JSON map to filter out unsupported attributes
fn process_map(
    map: &Map<String, Value>,
    accepted_keys: &[String],
    parent_key: Option<&str>,
) -> Value {
    let mut filtered_map: Map<String, serde_json::Value> = map
        .iter()
        .filter_map(|(key, value)| {
            let should_remove = !accepted_keys.contains(key) && parent_key != Some("properties");
            if should_remove {
                return None;
            }
            // Process nested maps recursively
            let filtered_value = match value {
                Value::Object(nested_map) => process_map(
                    &nested_map
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    accepted_keys,
                    Some(key),
                ),
                _ => value.clone(),
            };

            Some((key.clone(), filtered_value))
        })
        .collect();
    if parent_key != Some("properties") && !filtered_map.contains_key("type") {
        filtered_map.insert("type".to_string(), Value::String("string".to_string()));
    }

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
        return Ok(Message {
            role,
            created,
            content,
        });
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
    Ok(Message {
        role,
        created,
        content,
    })
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
        generation_config.insert("temperature".to_string(), json!(temp));
    }
    if let Some(tokens) = model_config.max_tokens {
        generation_config.insert("maxOutputTokens".to_string(), json!(tokens));
    }
    if !generation_config.is_empty() {
        payload.insert("generationConfig".to_string(), json!(generation_config));
    }

    Ok(Value::Object(payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn set_up_text_message(text: &str, role: Role) -> Message {
        Message {
            role,
            created: 0,
            content: vec![MessageContent::text(text.to_string())],
        }
    }

    fn set_up_tool_request_message(id: &str, tool_call: ToolCall) -> Message {
        Message {
            role: Role::User,
            created: 0,
            content: vec![MessageContent::tool_request(id.to_string(), Ok(tool_call))],
        }
    }

    fn set_up_tool_confirmation_message(id: &str, tool_call: ToolCall) -> Message {
        Message {
            role: Role::User,
            created: 0,
            content: vec![MessageContent::tool_confirmation_request(
                id.to_string(),
                tool_call.name.clone(),
                tool_call.arguments.clone(),
                Some("Goose would like to call the above tool. Allow? (y/n):".to_string()),
            )],
        }
    }

    fn set_up_tool_response_message(id: &str, tool_response: Vec<Content>) -> Message {
        Message {
            role: Role::Assistant,
            created: 0,
            content: vec![MessageContent::tool_response(
                id.to_string(),
                Ok(tool_response),
            )],
        }
    }

    fn set_up_tool(name: &str, description: &str, params: Value) -> Tool {
        Tool {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: json!({
                "properties": params
            }),
            annotations: None,
        }
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
            set_up_tool_request_message("id", ToolCall::new("tool_name", json!(arguments))),
            set_up_tool_confirmation_message("id2", ToolCall::new("tool_name_2", json!(arguments))),
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
        let params1 = json!({
            "param1": {
                "type": "string",
                "description": "A parameter",
                "field_does_not_accept": ["value1", "value2"]
            }
        });
        let params2 = json!({
            "param2": {
                "type": "string",
                "description": "B parameter",
            }
        });
        let tools = vec![
            set_up_tool("tool1", "description1", params1),
            set_up_tool("tool2", "description2", params2),
        ];
        let result = format_tools(&tools);
        assert_eq!(result.len(), 2);
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
    }

    #[test]
    fn test_tools_to_google_spec_with_empty_properties() {
        let tools = vec![Tool {
            name: "tool1".to_string(),
            description: "description1".to_string(),
            input_schema: json!({
                "properties": {}
            }),
            annotations: None,
        }];
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
