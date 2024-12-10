use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::errors::AgentError;
use crate::models::content::{Content, ImageContent};
use crate::models::message::{Message, MessageContent};
use crate::models::role::Role;
use crate::models::tool::{Tool, ToolCall};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    OpenAi,
    Anthropic,
}

/// Convert internal Message format to OpenAI's API message specification
///   some openai compatible endpoints use the anthropic image spec at the content level
///   even though the message structure is otherwise following openai, the enum switches this
pub fn messages_to_openai_spec(messages: &[Message], image_format: &ImageFormat) -> Vec<Value> {
    let mut messages_spec = Vec::new();

    for message in messages {
        let mut converted = json!({
            "role": message.role
        });

        let mut output = Vec::new();

        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    if !text.text.is_empty() {
                        converted["content"] = json!(text.text);
                    }
                }
                MessageContent::ToolRequest(request) => match &request.tool_call {
                    Ok(tool_call) => {
                        let sanitized_name = sanitize_function_name(&tool_call.name);
                        let tool_calls = converted
                            .as_object_mut()
                            .unwrap()
                            .entry("tool_calls")
                            .or_insert(json!([]));

                        tool_calls.as_array_mut().unwrap().push(json!({
                            "id": request.id,
                            "type": "function",
                            "function": {
                                "name": sanitized_name,
                                "arguments": tool_call.arguments.to_string(),
                            }
                        }));
                    }
                    Err(e) => {
                        output.push(json!({
                            "role": "tool",
                            "content": format!("Error: {}", e),
                            "tool_call_id": request.id
                        }));
                    }
                },
                MessageContent::ToolResponse(response) => {
                    match &response.tool_result {
                        Ok(contents) => {
                            // Send only contents with no audience or with Assistant in the audience
                            let abridged: Vec<_> = contents
                                .iter()
                                .filter(|content| {
                                    content
                                        .audience()
                                        .is_none_or(|audience| audience.contains(&Role::Assistant))
                                })
                                .map(|content| content.unannotated())
                                .collect();

                            // Process all content, replacing images with placeholder text
                            let mut tool_content = Vec::new();
                            let mut image_messages = Vec::new();

                            for content in abridged {
                                match content {
                                    Content::Image(image) => {
                                        // Add placeholder text in the tool response
                                        tool_content.push(Content::text("This tool result included an image that is uploaded in the next message."));

                                        // Create a separate image message
                                        image_messages.push(json!({
                                            "role": "user",
                                            "content": [convert_image(&image, image_format)]
                                        }));
                                    }
                                    _ => {
                                        tool_content.push(content);
                                    }
                                }
                            }

                            // First add the tool response with all content
                            output.push(json!({
                                "role": "tool",
                                "content": tool_content,
                                "tool_call_id": response.id
                            }));

                            // Then add any image messages that need to follow
                            output.extend(image_messages);
                        }
                        Err(e) => {
                            // A tool result error is shown as output so the model can interpret the error message
                            output.push(json!({
                                "role": "tool",
                                "content": format!("The tool call returned the following error:\n{}", e),
                                "tool_call_id": response.id
                            }));
                        }
                    }
                }
                MessageContent::Image(image) => {
                    // Handle direct image content
                    converted["content"] = json!([convert_image(image, image_format)]);
                }
            }
        }

        if converted.get("content").is_some() || converted.get("tool_calls").is_some() {
            output.insert(0, converted);
        }
        messages_spec.extend(output);
    }

    messages_spec
}

/// Convert an image content into an image json based on format
pub fn convert_image(image: &ImageContent, image_format: &ImageFormat) -> Value {
    match image_format {
        ImageFormat::OpenAi => json!({
            "type": "image_url",
            "image_url": {
                "url": format!("data:{};base64,{}", image.mime_type, image.data)
            }
        }),
        ImageFormat::Anthropic => json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": image.mime_type,
                "data": image.data,
            }
        }),
    }
}

/// Convert internal Tool format to OpenAI's API tool specification
pub fn tools_to_openai_spec(tools: &[Tool]) -> Result<Vec<Value>> {
    let mut tool_names = std::collections::HashSet::new();
    let mut result = Vec::new();

    for tool in tools {
        if !tool_names.insert(&tool.name) {
            return Err(anyhow!("Duplicate tool name: {}", tool.name));
        }

        result.push(json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema,
            }
        }));
    }

    Ok(result)
}

/// Convert OpenAI's API response to internal Message format
pub fn openai_response_to_message(response: Value) -> Result<Message> {
    let original = response["choices"][0]["message"].clone();
    let mut content = Vec::new();

    if let Some(text) = original.get("content") {
        if let Some(text_str) = text.as_str() {
            content.push(MessageContent::text(text_str));
        }
    }

    if let Some(tool_calls) = original.get("tool_calls") {
        if let Some(tool_calls_array) = tool_calls.as_array() {
            for tool_call in tool_calls_array {
                let id = tool_call["id"].as_str().unwrap_or_default().to_string();
                let function_name = tool_call["function"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let arguments = tool_call["function"]["arguments"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                if !is_valid_function_name(&function_name) {
                    let error = AgentError::ToolNotFound(format!(
                        "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                        function_name
                    ));
                    content.push(MessageContent::tool_request(id, Err(error)));
                } else {
                    match serde_json::from_str::<Value>(&arguments) {
                        Ok(params) => {
                            content.push(MessageContent::tool_request(
                                id,
                                Ok(ToolCall::new(&function_name, params)),
                            ));
                        }
                        Err(e) => {
                            let error = AgentError::InvalidParameters(format!(
                                "Could not interpret tool use parameters for id {}: {}",
                                id, e
                            ));
                            content.push(MessageContent::tool_request(id, Err(error)));
                        }
                    }
                }
            }
        }
    }

    Ok(Message {
        role: Role::Assistant,
        created: chrono::Utc::now().timestamp(),
        content,
    })
}

fn sanitize_function_name(name: &str) -> String {
    let re = Regex::new(r"[^a-zA-Z0-9_-]").unwrap();
    re.replace_all(name, "_").to_string()
}

fn is_valid_function_name(name: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    re.is_match(name)
}

#[derive(Debug, thiserror::Error)]
#[error("Context length exceeded. Message: {0}")]
pub struct ContextLengthExceededError(String);

pub fn check_openai_context_length_error(error: &Value) -> Option<ContextLengthExceededError> {
    let code = error.get("code")?.as_str()?;
    if code == "context_length_exceeded" || code == "string_above_max_length" {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error")
            .to_string();
        Some(ContextLengthExceededError(message))
    } else {
        None
    }
}

pub fn check_bedrock_context_length_error(error: &Value) -> Option<ContextLengthExceededError> {
    let external_message = error.get("external_model_message")?.get("message")?.as_str()?;
    if external_message.to_lowercase().contains("too long") {
        Some(ContextLengthExceededError(external_message.to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::content::Content;
    use serde_json::json;

    const OPENAI_TOOL_USE_RESPONSE: &str = r#"{
        "choices": [{
            "role": "assistant",
            "message": {
                "tool_calls": [{
                    "id": "1",
                    "function": {
                        "name": "example_fn",
                        "arguments": "{\"param\": \"value\"}"
                    }
                }]
            }
        }],
        "usage": {
            "input_tokens": 10,
            "output_tokens": 25,
            "total_tokens": 35
        }
    }"#;

    #[test]
    fn test_messages_to_openai_spec() -> Result<()> {
        let message = Message::user().with_text("Hello");
        let spec = messages_to_openai_spec(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(spec[0]["content"], "Hello");
        Ok(())
    }

    #[test]
    fn test_tools_to_openai_spec() -> Result<()> {
        let tool = Tool::new(
            "test_tool",
            "A test tool",
            json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let spec = tools_to_openai_spec(&[tool])?;

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["type"], "function");
        assert_eq!(spec[0]["function"]["name"], "test_tool");
        Ok(())
    }

    #[test]
    fn test_sanitize_function_name() {
        assert_eq!(sanitize_function_name("hello-world"), "hello-world");
        assert_eq!(sanitize_function_name("hello world"), "hello_world");
        assert_eq!(sanitize_function_name("hello@world"), "hello_world");
    }

    #[test]
    fn test_is_valid_function_name() {
        assert!(is_valid_function_name("hello-world"));
        assert!(is_valid_function_name("hello_world"));
        assert!(!is_valid_function_name("hello world"));
        assert!(!is_valid_function_name("hello@world"));
    }

    #[test]
    fn test_messages_to_openai_spec_complex() -> Result<()> {
        let mut messages = vec![
            Message::assistant().with_text("Hello!"),
            Message::user().with_text("How are you?"),
            Message::assistant().with_tool_request(
                "tool1",
                Ok(ToolCall::new("example", json!({"param1": "value1"}))),
            ),
        ];

        // Get the ID from the tool request to use in the response
        let tool_id = if let MessageContent::ToolRequest(request) = &messages[2].content[0] {
            request.id.clone()
        } else {
            panic!("should be tool request");
        };

        messages
            .push(Message::user().with_tool_response(tool_id, Ok(vec![Content::text("Result")])));

        let spec = messages_to_openai_spec(&messages, &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 4);
        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"], "Hello!");
        assert_eq!(spec[1]["role"], "user");
        assert_eq!(spec[1]["content"], "How are you?");
        assert_eq!(spec[2]["role"], "assistant");
        assert!(spec[2]["tool_calls"].is_array());
        assert_eq!(spec[3]["role"], "tool");
        assert_eq!(
            spec[3]["content"],
            json!([{"text": "Result", "type": "text"}])
        );
        assert_eq!(spec[3]["tool_call_id"], spec[2]["tool_calls"][0]["id"]);

        Ok(())
    }

    #[test]
    fn test_tools_to_openai_spec_duplicate() -> Result<()> {
        let tool1 = Tool::new(
            "test_tool",
            "Test tool",
            json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let tool2 = Tool::new(
            "test_tool",
            "Test tool",
            json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let result = tools_to_openai_spec(&[tool1, tool2]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate tool name"));

        Ok(())
    }

    #[test]
    fn test_tools_to_openai_spec_empty() -> Result<()> {
        let spec = tools_to_openai_spec(&[])?;
        assert!(spec.is_empty());
        Ok(())
    }

    #[test]
    fn test_openai_response_to_message_text() -> Result<()> {
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "content": "Hello from John Cena!"
                }
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 25,
                "total_tokens": 35
            }
        });

        let message = openai_response_to_message(response)?;
        assert_eq!(message.content.len(), 1);
        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello from John Cena!");
        } else {
            panic!("Expected Text content");
        }
        assert!(matches!(message.role, Role::Assistant));

        Ok(())
    }

    #[test]
    fn test_openai_response_to_message_valid_toolrequest() -> Result<()> {
        let response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        let message = openai_response_to_message(response)?;

        assert_eq!(message.content.len(), 1);
        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "example_fn");
            assert_eq!(tool_call.arguments, json!({"param": "value"}));
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_openai_response_to_message_invalid_func_name() -> Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["name"] =
            json!("invalid fn");

        let message = openai_response_to_message(response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(AgentError::ToolNotFound(msg)) => {
                    assert!(msg.starts_with("The provided function name"));
                }
                _ => panic!("Expected ToolNotFound error"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_openai_response_to_message_json_decode_error() -> Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            json!("invalid json {");

        let message = openai_response_to_message(response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(AgentError::InvalidParameters(msg)) => {
                    assert!(msg.starts_with("Could not interpret tool use parameters"));
                }
                _ => panic!("Expected InvalidParameters error"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_check_openai_context_length_error() {
        let error = json!({
            "code": "context_length_exceeded",
            "message": "This message is too long"
        });

        let result = check_openai_context_length_error(&error);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().to_string(),
            "Context length exceeded. Message: This message is too long"
        );

        let error = json!({
            "code": "other_error",
            "message": "Some other error"
        });

        let result = check_openai_context_length_error(&error);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_bedrock_context_length_error() {
        let error = json!({
            "error": "Received error from amazon-bedrock",
            "external_model_message": {
                "message": "Input is too long for requested model."
            }
        });

        let result = check_bedrock_context_length_error(&error);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().to_string(),
            "Context length exceeded. Message: Input is too long for requested model."
        );

        let error = json!({
            "error": "Some other error",
            "external_model_message": {
                "message": "Some other message"
            }
        });

        let result = check_bedrock_context_length_error(&error);
        assert!(result.is_none());
    }
}
