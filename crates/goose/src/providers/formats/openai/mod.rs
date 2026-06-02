use crate::conversation::message::{Message, MessageContent, ProviderMetadata};
use crate::mcp_utils::extract_text_from_resource;
use crate::model::ModelConfig;
use crate::providers::base::{split_think_blocks, ProviderUsage, ThinkFilter, Usage};
use crate::providers::errors::ProviderError;
use crate::providers::utils::{
    convert_image, detect_image_path, extract_reasoning_effort, is_openai_responses_model,
    is_valid_function_name, load_image_file, openai_reasoning_effort_for_thinking,
    safely_parse_json, sanitize_function_name, ImageFormat,
};
use anyhow::{anyhow, Error};
use async_stream::try_stream;
use chrono;
use futures::Stream;
use rmcp::model::{
    object, AnnotateAble, CallToolRequestParams, Content, ErrorCode, ErrorData, RawContent, Role,
    Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;

mod messages;
mod responses;
mod streaming;
mod types;

pub use messages::*;
pub use responses::*;
pub use streaming::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::model::CallToolResult;
    use rmcp::object;
    use serde_json::json;
    use test_case::test_case;
    use tokio::pin;
    use tokio_stream::{self, StreamExt};

    #[test]
    fn test_validate_tool_schemas() {
        // Test case 1: Empty parameters object
        // Input JSON with an incomplete parameters object
        let mut actual = vec![json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "type": "object"
                }
            }
        })];

        // Run the function to validate and update schemas
        validate_tool_schemas(&mut actual);

        // Expected JSON after validation
        let expected = vec![json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        })];

        // Compare entire JSON structures instead of individual fields
        assert_eq!(actual, expected);

        // Test case 2: Missing type field
        let mut tools = vec![json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "properties": {}
                }
            }
        })];

        validate_tool_schemas(&mut tools);

        let params = tools[0]["function"]["parameters"].as_object().unwrap();
        assert_eq!(params["type"], "object");

        // Test case 3: Complete valid schema should remain unchanged
        let original_schema = json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "City and country"
                        }
                    },
                    "required": ["location"]
                }
            }
        });

        let mut tools = vec![original_schema.clone()];
        validate_tool_schemas(&mut tools);
        assert_eq!(tools[0], original_schema);

        // Test case 4: anyOf nullable is unwrapped, preserving sibling metadata
        let mut tools = vec![json!({
            "type": "function",
            "function": {
                "name": "shell",
                "description": "run shell",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "timeout_secs": {
                            "description": "timeout in seconds",
                            "anyOf": [
                                { "type": "integer", "format": "uint64", "minimum": 0 },
                                { "type": "null" }
                            ]
                        }
                    },
                    "required": ["command"]
                }
            }
        })];
        validate_tool_schemas(&mut tools);
        let timeout_schema = &tools[0]["function"]["parameters"]["properties"]["timeout_secs"];
        assert_eq!(timeout_schema["type"], "integer");
        assert_eq!(timeout_schema["format"], "uint64");
        assert_eq!(timeout_schema["description"], "timeout in seconds");
        assert!(timeout_schema.get("anyOf").is_none());

        // Test case 4b: type array form (schemars 1.x style for nullable primitives)
        let mut tools = vec![json!({
            "type": "function",
            "function": {
                "name": "shell",
                "description": "run shell",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "timeout_secs": {
                            "type": ["integer", "null"],
                            "format": "uint64",
                            "minimum": 0
                        }
                    },
                    "required": ["command"]
                }
            }
        })];
        validate_tool_schemas(&mut tools);
        let timeout_schema = &tools[0]["function"]["parameters"]["properties"]["timeout_secs"];
        assert_eq!(timeout_schema["type"], "integer");
        assert!(!timeout_schema["type"].is_array());

        // Test case 5: Verify the actual ShellParams schema is compatible (no anyOf for timeout_secs)
        use crate::agents::platform_extensions::developer::shell::ShellParams;
        use schemars::schema_for;
        let schema_value = serde_json::to_value(schema_for!(ShellParams)).unwrap();
        let schema_obj = schema_value.as_object().unwrap().clone();
        let tool = rmcp::model::Tool::new("shell", "run shell", schema_obj);
        let mut tools = vec![json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema,
            }
        })];
        validate_tool_schemas(&mut tools);
        let timeout = &tools[0]["function"]["parameters"]["properties"]["timeout_secs"];
        assert!(
            timeout.get("anyOf").is_none(),
            "timeout_secs should not have anyOf after validation, got: {timeout}"
        );
        assert_eq!(
            timeout["type"], "integer",
            "timeout_secs should have type=integer"
        );
    }

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
    fn test_format_messages() -> anyhow::Result<()> {
        let message = Message::user().with_text("Hello");
        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(spec[0]["content"], "Hello");
        Ok(())
    }

    #[test]
    fn test_format_tools() -> anyhow::Result<()> {
        let tool = Tool::new(
            "test_tool",
            "A test tool",
            object!({
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

        let spec = format_tools(&[tool])?;

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["type"], "function");
        assert_eq!(spec[0]["function"]["name"], "test_tool");
        Ok(())
    }

    #[test]
    fn test_format_messages_complex() -> anyhow::Result<()> {
        let mut messages = vec![
            Message::assistant().with_text("Hello!"),
            Message::user().with_text("How are you?"),
            Message::assistant().with_tool_request(
                "tool1",
                Ok(CallToolRequestParams::new("example")
                    .with_arguments(object!({"param1": "value1"}))),
            ),
        ];

        // Get the ID from the tool request to use in the response
        let tool_id = if let MessageContent::ToolRequest(request) = &messages[2].content[0] {
            request.id.clone()
        } else {
            panic!("should be tool request");
        };

        messages.push(Message::user().with_tool_response(
            tool_id,
            Ok(CallToolResult::success(vec![Content::text("Result")])),
        ));

        let spec = format_messages(&messages, &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 4);
        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"], "Hello!");
        assert_eq!(spec[1]["role"], "user");
        assert_eq!(spec[1]["content"], "How are you?");
        assert_eq!(spec[2]["role"], "assistant");
        assert!(spec[2]["tool_calls"].is_array());
        assert_eq!(spec[3]["role"], "tool");
        assert_eq!(spec[3]["content"], "Result");
        assert_eq!(spec[3]["tool_call_id"], spec[2]["tool_calls"][0]["id"]);

        Ok(())
    }

    #[test]
    fn test_format_messages_multiple_content() -> anyhow::Result<()> {
        let mut messages = vec![Message::assistant().with_tool_request(
            "tool1",
            Ok(CallToolRequestParams::new("example").with_arguments(object!({"param1": "value1"}))),
        )];

        // Get the ID from the tool request to use in the response
        let tool_id = if let MessageContent::ToolRequest(request) = &messages[0].content[0] {
            request.id.clone()
        } else {
            panic!("should be tool request");
        };

        messages.push(Message::user().with_tool_response(
            tool_id,
            Ok(CallToolResult::success(vec![Content::text("Result")])),
        ));

        let spec = format_messages(&messages, &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());
        assert_eq!(spec[1]["role"], "tool");
        assert_eq!(spec[1]["content"], "Result");
        assert_eq!(spec[1]["tool_call_id"], spec[0]["tool_calls"][0]["id"]);

        Ok(())
    }

    #[test]
    fn test_format_tools_duplicate() -> anyhow::Result<()> {
        let tool1 = Tool::new(
            "test_tool",
            "Test tool",
            object!({
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
            object!({
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

        let result = format_tools(&[tool1, tool2]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate tool name"));

        Ok(())
    }

    #[test]
    fn test_format_tools_empty() -> anyhow::Result<()> {
        let spec = format_tools(&[])?;
        assert!(spec.is_empty());
        Ok(())
    }

    #[test]
    fn test_format_messages_with_image_path() -> anyhow::Result<()> {
        // Create a temporary PNG file with valid PNG magic numbers
        let temp_dir = tempfile::tempdir()?;
        let png_path = temp_dir.path().join("test.png");
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, // PNG magic number
            0x0D, 0x0A, 0x1A, 0x0A, // PNG header
            0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
        ];
        std::fs::write(&png_path, png_data)?;
        let png_path_str = png_path.to_str().unwrap();

        // Create user message with image path - should load the image
        let user_message = Message::user().with_text(format!("Here is an image: {}", png_path_str));
        let spec = format_messages(&[user_message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");

        // Content should be an array with text and image
        let content = spec[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert!(content[0]["text"].as_str().unwrap().contains(png_path_str));
        assert_eq!(content[1]["type"], "image_url");
        assert!(content[1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));

        // Create assistant message with same text - should NOT load the image
        let assistant_message =
            Message::assistant().with_text(format!("I saved the output to {}", png_path_str));
        let spec = format_messages(&[assistant_message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");

        // Content should be plain text, NOT an array with image
        let content = spec[0]["content"].as_str();
        assert!(
            content.is_some(),
            "Assistant message content should be a string, not an array with image"
        );
        assert!(content.unwrap().contains(png_path_str));

        Ok(())
    }

    #[test]
    fn test_format_messages_with_text_and_image_preserves_order() {
        // Text before image: order should be [text, image]
        let msg_text_first = Message::user()
            .with_text("Describe this image")
            .with_image("aW1hZ2VkYXRh", "image/png");

        let spec = format_messages(&[msg_text_first], &ImageFormat::OpenAi);
        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");

        let content = spec[0]["content"]
            .as_array()
            .expect("content should be an array");
        assert_eq!(content.len(), 2, "expected text + image entries");
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Describe this image");
        assert_eq!(content[1]["type"], "image_url");

        // Image before text: order should be [image, text]
        let msg_image_first = Message::user()
            .with_image("aW1hZ2VkYXRh", "image/png")
            .with_text("What do you see?");

        let spec2 = format_messages(&[msg_image_first], &ImageFormat::OpenAi);
        let content2 = spec2[0]["content"]
            .as_array()
            .expect("content should be an array");
        assert_eq!(content2.len(), 2, "expected image + text entries");
        assert_eq!(content2[0]["type"], "image_url");
        assert_eq!(content2[1]["type"], "text");
        assert_eq!(content2[1]["text"], "What do you see?");
    }

    #[test]
    fn test_response_to_message_text() -> anyhow::Result<()> {
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

        let message = response_to_message(&response)?;
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
    fn test_response_to_message_valid_toolrequest() -> anyhow::Result<()> {
        let response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        let message = response_to_message(&response)?;

        assert_eq!(message.content.len(), 1);
        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "example_fn");
            assert_eq!(tool_call.arguments, Some(object!({"param": "value"})));
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_invalid_func_name() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["name"] =
            json!("invalid fn");

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ErrorData {
                    code: ErrorCode::INVALID_REQUEST,
                    message: msg,
                    data: None,
                }) => {
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
    fn test_response_to_message_json_decode_error() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            json!("invalid json {");

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ErrorData {
                    code: ErrorCode::INVALID_PARAMS,
                    message: msg,
                    data: None,
                }) => {
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
    fn test_response_to_message_empty_argument() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            serde_json::Value::String("".to_string());

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "example_fn");
            assert_eq!(tool_call.arguments, Some(object!({})));
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_api_error() -> anyhow::Result<()> {
        // Test that API responses with an "error" field return the error message
        let response = json!({
            "error": {
                "message": "You have exceeded your quota",
                "type": "insufficient_quota",
                "code": "quota_exceeded"
            }
        });

        let result = response_to_message(&response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("API error:"));
        assert!(err.to_string().contains("You have exceeded your quota"));

        Ok(())
    }

    #[test]
    fn test_response_to_message_api_error_unknown() -> anyhow::Result<()> {
        // Test that API responses with an "error" field but no message return "Unknown error"
        let response = json!({
            "error": {
                "type": "some_error"
            }
        });

        let result = response_to_message(&response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("API error:"));
        assert!(err.to_string().contains("Unknown error"));

        Ok(())
    }

    #[test]
    fn test_response_to_message_no_choices() -> anyhow::Result<()> {
        // Test that responses without "choices" return an error
        let response = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1234567890
        });

        let result = response_to_message(&response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No message in API response"));

        Ok(())
    }

    #[test]
    fn test_format_messages_tool_request_with_none_arguments() -> anyhow::Result<()> {
        // Test that tool calls with None arguments are formatted as "{}" string
        let message = Message::assistant()
            .with_tool_request("tool1", Ok(CallToolRequestParams::new("test_tool")));

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());

        let tool_call = &spec[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "tool1");
        assert_eq!(tool_call["type"], "function");
        assert_eq!(tool_call["function"]["name"], "test_tool");
        // This should be the string "{}", not null
        assert_eq!(tool_call["function"]["arguments"], "{}");

        Ok(())
    }

    #[test]
    fn test_format_messages_tool_request_with_some_arguments() -> anyhow::Result<()> {
        // Test that tool calls with Some arguments are properly JSON-serialized
        let message = Message::assistant().with_tool_request(
            "tool1",
            Ok(CallToolRequestParams::new("test_tool")
                .with_arguments(object!({"param": "value", "number": 42}))),
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());

        let tool_call = &spec[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "tool1");
        assert_eq!(tool_call["type"], "function");
        assert_eq!(tool_call["function"]["name"], "test_tool");
        // This should be a JSON string representation
        let args_str = tool_call["function"]["arguments"].as_str().unwrap();
        let parsed_args: Value = serde_json::from_str(args_str)?;
        assert_eq!(parsed_args["param"], "value");
        assert_eq!(parsed_args["number"], 42);

        Ok(())
    }

    #[test]
    fn test_format_messages_frontend_tool_request_with_none_arguments() -> anyhow::Result<()> {
        // Test that FrontendToolRequest with None arguments are formatted as "{}" string
        let message = Message::assistant().with_frontend_tool_request(
            "frontend_tool1",
            Ok(CallToolRequestParams::new("frontend_test_tool")),
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());

        let tool_call = &spec[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "frontend_tool1");
        assert_eq!(tool_call["type"], "function");
        assert_eq!(tool_call["function"]["name"], "frontend_test_tool");
        // This should be the string "{}", not null
        assert_eq!(tool_call["function"]["arguments"], "{}");

        Ok(())
    }

    #[test]
    fn test_format_messages_frontend_tool_request_with_some_arguments() -> anyhow::Result<()> {
        // Test that FrontendToolRequest with Some arguments are properly JSON-serialized
        let message = Message::assistant().with_frontend_tool_request(
            "frontend_tool1",
            Ok(CallToolRequestParams::new("frontend_test_tool")
                .with_arguments(object!({"action": "click", "element": "button"}))),
        );

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());

        let tool_call = &spec[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "frontend_tool1");
        assert_eq!(tool_call["type"], "function");
        assert_eq!(tool_call["function"]["name"], "frontend_test_tool");
        // This should be a JSON string representation
        let args_str = tool_call["function"]["arguments"].as_str().unwrap();
        let parsed_args: Value = serde_json::from_str(args_str)?;
        assert_eq!(parsed_args["action"], "click");
        assert_eq!(parsed_args["element"], "button");

        Ok(())
    }

    #[test]
    fn test_format_messages_multiple_text_blocks() -> anyhow::Result<()> {
        let message = Message::user()
            .with_text("--- Resource: file:///test.md ---\n# Test\n\n---\n")
            .with_text(" What is in the file?");

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(
            spec[0]["content"],
            "--- Resource: file:///test.md ---\n# Test\n\n---\n\n What is in the file?"
        );
        Ok(())
    }

    #[test]
    fn test_create_request_gpt_4o() -> anyhow::Result<()> {
        // Test default medium reasoning effort for O3 model
        let model_config = ModelConfig {
            model_name: "gpt-4o".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "gpt-4o",
            "messages": [
                {
                    "role": "system",
                    "content": "system"
                }
            ],
            "max_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }

        Ok(())
    }

    #[test]
    fn test_create_request_omits_max_tokens_when_unset() -> anyhow::Result<()> {
        // Unknown models on OpenAI-compatible local providers (llama_swap,
        // lmstudio) have no canonical record and no GOOSE_MAX_TOKENS, so the
        // request must not pin the legacy 4096 default. See issue #9007.
        let model_config = ModelConfig {
            model_name: "some-unknown-local-model".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();
        assert!(
            !obj.contains_key("max_tokens"),
            "max_tokens should be omitted when model_config.max_tokens is None"
        );
        assert!(
            !obj.contains_key("max_completion_tokens"),
            "max_completion_tokens should be omitted when model_config.max_tokens is None"
        );
        Ok(())
    }

    #[test]
    fn test_request_params_preserve_reserved_fields() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "glm-4.7".to_string(),
            context_limit: Some(204800),
            temperature: None,
            max_tokens: Some(4096),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: Some(std::collections::HashMap::from([
                (
                    "thinking".to_string(),
                    json!({
                        "type": "enabled",
                        "clear_thinking": false
                    }),
                ),
                ("stream".to_string(), json!(false)),
                (
                    "stream_options".to_string(),
                    json!({"include_usage": false}),
                ),
                ("model".to_string(), json!("wrong-model")),
                ("messages".to_string(), json!([])),
                ("max_tokens".to_string(), json!(1)),
                ("temperature".to_string(), json!(2.0)),
                ("provider_custom".to_string(), json!("allowed")),
            ])),
            reasoning: None,
        };

        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            true,
        )?;

        assert_eq!(
            request["thinking"],
            json!({
                "type": "enabled",
                "clear_thinking": false
            })
        );
        assert_eq!(request["stream"], true);
        assert_eq!(request["stream_options"], json!({"include_usage": true}));
        assert_eq!(request["model"], "glm-4.7");
        assert_eq!(request["messages"][0]["role"], "system");
        assert_eq!(request["max_tokens"], 1);
        assert_eq!(request["temperature"], 2.0);
        assert_eq!(request["provider_custom"], "allowed");

        Ok(())
    }

    #[test]
    fn test_create_request_o1_default() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "o1".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "o1",
            "messages": [
                {
                    "role": "developer",
                    "content": "system"
                }
            ],
            "max_completion_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }
        assert!(
            obj.get("reasoning_effort").is_none(),
            "reasoning_effort should be omitted when no explicit suffix is provided"
        );

        Ok(())
    }

    #[test]
    fn test_create_request_o1_medium_effort() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!("medium"));
        let model_config = ModelConfig {
            model_name: "o1".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: Some(params),
            reasoning: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();

        assert_eq!(obj.get("reasoning_effort"), Some(&json!("medium")));
        assert!(obj.get("thinking_effort").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_o3_off_effort_preserves_none() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!("off"));
        let model_config = ModelConfig {
            model_name: "o3".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: Some(params),
            reasoning: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();

        assert_eq!(obj.get("reasoning_effort"), Some(&json!("none")));
        assert!(obj.get("thinking_effort").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_gpt5_pro_max_effort_uses_supported_level() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!("max"));
        let model_config = ModelConfig {
            model_name: "gpt-5.2-pro-2025-12-11".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: Some(params),
            reasoning: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();

        assert_eq!(obj.get("reasoning_effort"), Some(&json!("high")));
        assert!(obj.get("thinking_effort").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_o3_custom_reasoning_effort() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!("high"));
        let model_config = ModelConfig {
            model_name: "o3-mini".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: Some(params),
            reasoning: None,
        };
        let request = create_request(
            &model_config,
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
            false,
        )?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "o3-mini",
            "messages": [
                {
                    "role": "developer",
                    "content": "system"
                }
            ],
            "reasoning_effort": "high",
            "max_completion_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }
        assert!(obj.get("thinking_effort").is_none());

        Ok(())
    }

    struct StreamingUsageTestResult {
        usage_count: usize,
        usage: Option<ProviderUsage>,
        tool_calls: Vec<String>,
        has_text_content: bool,
    }

    async fn run_streaming_test(response_lines: &str) -> anyhow::Result<StreamingUsageTestResult> {
        let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let messages = response_to_streaming_message(response_stream);
        pin!(messages);

        let mut result = StreamingUsageTestResult {
            usage_count: 0,
            usage: None,
            tool_calls: Vec::new(),
            has_text_content: false,
        };

        while let Some(Ok((message, usage))) = messages.next().await {
            if let Some(u) = usage {
                result.usage_count += 1;
                result.usage = Some(u);
            }
            if let Some(msg) = message {
                for content in &msg.content {
                    match content {
                        MessageContent::ToolRequest(req) => {
                            if let Ok(tool_call) = &req.tool_call {
                                result.tool_calls.push(tool_call.name.to_string());
                            }
                        }
                        MessageContent::Text(text) if !text.text.is_empty() => {
                            result.has_text_content = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(result)
    }

    fn assert_usage_yielded_once(
        result: &StreamingUsageTestResult,
        expected_input: i32,
        expected_output: i32,
        expected_total: i32,
    ) {
        assert_eq!(
            result.usage_count, 1,
            "Usage should be yielded exactly once, but was yielded {} times",
            result.usage_count
        );

        let usage = result.usage.as_ref().expect("Expected usage to be present");
        assert_eq!(usage.usage.input_tokens, Some(expected_input));
        assert_eq!(usage.usage.output_tokens, Some(expected_output));
        assert_eq!(usage.usage.total_tokens, Some(expected_total));
    }

    #[test]
    fn test_get_usage_preserves_provider_totals_with_cache_fields() {
        let usage = get_usage(&json!({
            "prompt_tokens": 120,
            "completion_tokens": 30,
            "total_tokens": 150,
            "cache_read_input_tokens": 80,
            "cache_creation_input_tokens": 20
        }));

        assert_eq!(usage.input_tokens, Some(120));
        assert_eq!(usage.output_tokens, Some(30));
        assert_eq!(usage.total_tokens, Some(150));
        assert_eq!(usage.cache_read_input_tokens, Some(80));
        assert_eq!(usage.cache_write_input_tokens, Some(20));
    }

    #[test]
    fn test_get_usage_reads_nested_usage_object() {
        let usage = get_usage(&json!({
            "id": "chatcmpl_test",
            "usage": {
                "prompt_tokens": 84,
                "completion_tokens": 21,
                "total_tokens": 105,
                "cache_read_input_tokens": 60,
                "cache_creation_input_tokens": 10
            }
        }));

        assert_eq!(usage.input_tokens, Some(84));
        assert_eq!(usage.output_tokens, Some(21));
        assert_eq!(usage.total_tokens, Some(105));
        assert_eq!(usage.cache_read_input_tokens, Some(60));
        assert_eq!(usage.cache_write_input_tokens, Some(10));
    }

    #[tokio::test]
    async fn test_streamed_multi_tool_response_to_messages() -> anyhow::Result<()> {
        let response_lines = r#"
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":"I'll run both"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":" `ls` commands in a"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":" single turn for you -"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":" one on the current directory an"},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":"d one on the `working_dir`."},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288340}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"id":"toolu_bdrk_01RMTd7R9DzQjEEWgDwzcBsU","type":"function","function":{"name":"developer__shell","arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":"{\""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":"command\": \"l"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":1,"function":{"arguments":"s\"}"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"id":"toolu_bdrk_016bgVTGZdpjP8ehjMWp9cWW","type":"function","function":{"name":"developer__shell","arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288341}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"{\""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"command\""}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":": \"ls wor"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"king_dir"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":null,"tool_calls":[{"index":2,"function":{"arguments":"\"}"}}]},"index":0,"finish_reason":null}],"usage":{"prompt_tokens":4982,"completion_tokens":null,"total_tokens":null},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: {"model":"us.anthropic.claude-sonnet-4-20250514-v1:0","choices":[{"delta":{"role":"assistant","content":""},"index":0,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":4982,"completion_tokens":122,"total_tokens":5104},"object":"chat.completion.chunk","id":"msg_bdrk_014pifLTHsNZz6Lmtw1ywgDJ","created":1753288342}
data: [DONE]
"#;

        let result = run_streaming_test(response_lines).await?;
        assert_eq!(
            result.tool_calls.len(),
            2,
            "Expected 2 tool calls, got {}",
            result.tool_calls.len()
        );
        assert!(result
            .tool_calls
            .iter()
            .all(|name| name == "developer__shell"));

        assert_usage_yielded_once(&result, 4982, 122, 5104);

        Ok(())
    }

    #[tokio::test]
    async fn test_openrouter_streaming_usage_yielded_once() -> anyhow::Result<()> {
        let response_lines = r#"
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":"","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":"There","reasoning":"","reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":" are","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":" **47**","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":" files.","reasoning":null,"reasoning_details":[]},"finish_reason":null,"native_finish_reason":null,"logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":"","reasoning":null,"reasoning_details":[]},"finish_reason":"stop","native_finish_reason":"stop","logprobs":null}]}
data: {"id":"gen-1768896871-9HgAQqS1Z72C6gApaidi","provider":"OpenInference","model":"openai/gpt-oss-120b:free","object":"chat.completion.chunk","created":1768896871,"choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null,"native_finish_reason":null,"logprobs":null}],"usage":{"prompt_tokens":7007,"completion_tokens":49,"total_tokens":7056}}
data: [DONE]
"#;

        let result = run_streaming_test(response_lines).await?;

        assert!(result.has_text_content, "Expected text content in response");
        assert_usage_yielded_once(&result, 7007, 49, 7056);

        Ok(())
    }

    #[tokio::test]
    async fn test_openai_gpt5_streaming_usage_yielded_once() -> anyhow::Result<()> {
        let response_lines = r#"
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[{"index":0,"delta":{"role":"assistant","content":null,"tool_calls":[{"index":0,"id":"call_x4CIvBVfQhYMhyO0T1VEddua","type":"function","function":{"name":"developer__shell","arguments":""}}],"refusal":null},"logprobs":null,"finish_reason":null}],"usage":null}
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"command\":\"ls ~/Desktop | wc -l\"}"}}]},"logprobs":null,"finish_reason":null}],"usage":null}
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"tool_calls"}],"usage":null}
data: {"id":"chatcmpl-Bk9Ye6Y0t9E7bC3DOMxCpW8eJkTKU","object":"chat.completion.chunk","created":1737368310,"model":"gpt-5.2-1106-preview","service_tier":"default","system_fingerprint":"fp_5f325d54e6","choices":[],"usage":{"prompt_tokens":8320,"completion_tokens":172,"total_tokens":8492}}
data: [DONE]
"#;

        let result = run_streaming_test(response_lines).await?;

        assert_eq!(result.tool_calls.len(), 1, "Expected 1 tool call");
        assert_eq!(result.tool_calls[0], "developer__shell");
        assert_usage_yielded_once(&result, 8320, 172, 8492);
        assert_eq!(
            result.usage.as_ref().map(|usage| usage.model.as_str()),
            Some("gpt-5.2-1106-preview")
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_tetrate_claude_streaming_usage_yielded_once() -> anyhow::Result<()> {
        let response_lines = r#"
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{"role":"assistant"}}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk"}
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"toolu_011Yj5pGczhs1597iLXp5XJK","type":"function","function":{"name":"developer__shell","arguments":""}}]}}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk"}
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"type":"function","function":{"arguments":"{\"command\": \"find ~/Desktop -type f | wc -l\"}"}}]}}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk"}
data: {"id":"msg_01BbvMfNhbdm2hmmTbWjaeYt","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"created":1768898776,"model":"claude-sonnet-4-5-20250929","object":"chat.completion.chunk","usage":{"completion_tokens":79,"prompt_tokens":12376,"total_tokens":12455}}
data: [DONE]
"#;

        let result = run_streaming_test(response_lines).await?;

        assert_eq!(result.tool_calls.len(), 1, "Expected 1 tool call");
        assert_eq!(result.tool_calls[0], "developer__shell");
        assert_usage_yielded_once(&result, 12376, 79, 12455);

        Ok(())
    }

    #[tokio::test]
    async fn test_azure_annotation_chunk_without_delta_does_not_fail() -> anyhow::Result<()> {
        let response_lines = r#"
data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":1234567890,"model":"gpt-5.4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}],"usage":null}
data: {"choices":[{"content_filter_offsets":{"check_offset":5,"start_offset":5,"end_offset":5},"content_filter_results":{"hate":{"filtered":false,"severity":"safe"},"self_harm":{"filtered":false,"severity":"safe"},"sexual":{"filtered":false,"severity":"safe"},"violence":{"filtered":false,"severity":"safe"}},"finish_reason":null,"index":0}],"created":0,"id":"","model":"","object":""}
data: {"id":"chatcmpl-test","object":"chat.completion.chunk","created":1234567891,"model":"gpt-5.4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":1,"total_tokens":11}}
data: [DONE]
"#;

        let result = run_streaming_test(response_lines).await?;

        assert!(result.has_text_content, "Expected text content in response");
        assert_usage_yielded_once(&result, 10, 1, 11);

        Ok(())
    }

    #[test]
    fn test_response_to_message_with_nested_extra_content() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_456",
                        "type": "function",
                        "function": {
                            "name": "test_tool",
                            "arguments": "{}"
                        },
                        "extra_content": {
                            "google": {
                                "thought_signature": "nested_sig_xyz789"
                            }
                        }
                    }]
                }
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 1);

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            assert!(request.tool_call.is_ok());
            assert!(request.metadata.is_some());
            let metadata = request.metadata.as_ref().unwrap();
            let extra_content = metadata.get("extra_content").unwrap();
            assert_eq!(
                extra_content["google"]["thought_signature"],
                "nested_sig_xyz789"
            );
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_with_multiple_extra_fields() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_789",
                        "type": "function",
                        "function": {
                            "name": "test_tool",
                            "arguments": "{}"
                        },
                        "thoughtSignature": "sig_top_level",
                        "extra_content": {
                            "google": {
                                "thought_signature": "sig_nested"
                            }
                        },
                        "custom_field": "custom_value"
                    }]
                }
            }]
        });

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let metadata = request.metadata.as_ref().unwrap();
            assert_eq!(metadata.get("thoughtSignature").unwrap(), "sig_top_level");
            assert_eq!(
                metadata.get("extra_content").unwrap()["google"]["thought_signature"],
                "sig_nested"
            );
            assert_eq!(metadata.get("custom_field").unwrap(), "custom_value");
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_streaming_response_with_nested_extra_content() -> anyhow::Result<()> {
        let response_lines = r#"data: {"model":"test-model","choices":[{"delta":{"role":"assistant","tool_calls":[{"extra_content":{"google":{"thought_signature":"nested_stream_sig"}},"id":"call_nested","function":{"name":"test_tool","arguments":"{}"},"type":"function","index":0}]},"index":0,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110},"object":"chat.completion.chunk","id":"test-id","created":1234567890}
data: [DONE]"#;

        let response_stream =
            tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
        let messages = response_to_streaming_message(response_stream);
        pin!(messages);

        while let Some(Ok((message, _usage))) = messages.next().await {
            if let Some(msg) = message {
                if let MessageContent::ToolRequest(request) = &msg.content[0] {
                    assert!(request.tool_call.is_ok());
                    assert!(request.metadata.is_some());
                    let metadata = request.metadata.as_ref().unwrap();
                    let extra_content = metadata.get("extra_content").unwrap();
                    assert_eq!(
                        extra_content["google"]["thought_signature"],
                        "nested_stream_sig"
                    );
                    return Ok(());
                }
            }
        }

        panic!("Expected tool call message with nested extra_content metadata");
    }

    #[tokio::test]
    async fn test_streaming_response_extracts_inline_think_blocks() -> anyhow::Result<()> {
        let response_lines = concat!(
            "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{\"content\":\"<thi\"},\"index\":0,\"finish_reason\":null}]}\n",
            "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{\"content\":\"nk>x</thi\"},\"index\":0,\"finish_reason\":null}]}\n",
            "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{\"content\":\"nk>y\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n",
            "data: [DONE]\n"
        );

        let response_stream =
            tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
        let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

        let mut text = String::new();
        let mut thinking = String::new();

        while let Some(result) = messages.next().await {
            let (message, _) = result?;
            if let Some(message) = message {
                for item in message.content {
                    match item {
                        MessageContent::Text(text_content) => text.push_str(&text_content.text),
                        MessageContent::Thinking(thinking_content) => {
                            thinking.push_str(&thinking_content.thinking)
                        }
                        _ => {}
                    }
                }
            }
        }

        assert_eq!(text, "y");
        assert_eq!(thinking, "x");

        Ok(())
    }

    #[tokio::test]
    async fn test_streaming_suppresses_inline_think_when_structured_reasoning_follows(
    ) -> anyhow::Result<()> {
        // Inline <think>...</think> arrives in an early content chunk, then
        // reasoning_content arrives in a later chunk. The inline thinking
        // should be discarded in favor of the structured reasoning so users
        // do not get duplicated reasoning output.
        let response_lines = concat!(
            "data: {\"id\":\"chunk-1\",\"choices\":[{\"delta\":{\"content\":\"<think>inline reasoning</think>Hi\"},\"index\":0,\"finish_reason\":null}]}\n",
            "data: {\"id\":\"chunk-2\",\"choices\":[{\"delta\":{\"reasoning_content\":\"structured reasoning\"},\"index\":0,\"finish_reason\":null}]}\n",
            "data: {\"id\":\"chunk-3\",\"choices\":[{\"delta\":{\"content\":\" there\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n",
            "data: [DONE]\n"
        );

        let response_stream =
            tokio_stream::iter(response_lines.lines().map(|line| Ok(line.to_string())));
        let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

        let mut text = String::new();
        let mut thinking = String::new();

        while let Some(result) = messages.next().await {
            let (message, _) = result?;
            if let Some(message) = message {
                for item in message.content {
                    match item {
                        MessageContent::Text(text_content) => text.push_str(&text_content.text),
                        MessageContent::Thinking(thinking_content) => {
                            thinking.push_str(&thinking_content.thinking)
                        }
                        _ => {}
                    }
                }
            }
        }

        assert_eq!(text, "Hi there");
        assert_eq!(thinking, "structured reasoning");

        Ok(())
    }

    #[test]
    fn test_response_to_message_with_reasoning_content() -> anyhow::Result<()> {
        // Test capturing reasoning_content from DeepSeek reasoning models
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "reasoning_content": "Let me think about this step by step...",
                    "content": "The answer is 9.11 is greater than 9.8"
                }
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 25,
                "total_tokens": 35
            }
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        // First should be thinking content (reasoning is mapped to thinking)
        if let MessageContent::Thinking(thinking) = &message.content[0] {
            assert_eq!(thinking.thinking, "Let me think about this step by step...");
        } else {
            panic!("Expected Thinking content, got {:?}", message.content[0]);
        }

        // Second should be text content
        if let MessageContent::Text(text) = &message.content[1] {
            assert_eq!(text.text, "The answer is 9.11 is greater than 9.8");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_extracts_inline_think_blocks() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "content": "<think>internal reasoning</think>Visible answer"
                }
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        if let MessageContent::Thinking(thinking) = &message.content[0] {
            assert_eq!(thinking.thinking, "internal reasoning");
        } else {
            panic!("Expected Thinking content, got {:?}", message.content[0]);
        }

        if let MessageContent::Text(text) = &message.content[1] {
            assert_eq!(text.text, "Visible answer");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_prefers_structured_reasoning_over_inline_think(
    ) -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "reasoning_content": "structured reasoning",
                    "content": "<think>inline reasoning</think>Visible answer"
                }
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        if let MessageContent::Thinking(thinking) = &message.content[0] {
            assert_eq!(thinking.thinking, "structured reasoning");
        } else {
            panic!("Expected Thinking content");
        }

        if let MessageContent::Text(text) = &message.content[1] {
            assert_eq!(text.text, "Visible answer");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_format_messages_with_reasoning_content() -> anyhow::Result<()> {
        // Test that reasoning_content is properly included in formatted messages
        let mut message = Message::assistant()
            .with_content(MessageContent::thinking(
                "Thinking through the problem...",
                "",
            ))
            .with_text("The result is 42");

        // Add a tool call to test that reasoning_content works with tool calls
        message = message.with_tool_request(
            "tool1",
            Ok(rmcp::model::CallToolRequestParams::new("test_tool")
                .with_arguments(rmcp::object!({"param": "value"}))),
        );

        let spec = format_messages_with_options(
            &[message],
            &ImageFormat::OpenAi,
            OpenAiFormatOptions {
                preserve_thinking_context: true,
            },
        );

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");

        // Should have reasoning_content field
        assert!(spec[0].get("reasoning_content").is_some());
        assert_eq!(
            spec[0]["reasoning_content"],
            "Thinking through the problem..."
        );

        // Should have content
        assert_eq!(spec[0]["content"], "The result is 42");

        // Should have tool_calls
        assert!(spec[0]["tool_calls"].is_array());
        assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

        Ok(())
    }

    #[test]
    fn test_format_messages_preserves_reasoning_content_for_legacy_compat() -> anyhow::Result<()> {
        let message = Message::assistant()
            .with_content(MessageContent::thinking(
                "Thinking through the problem...",
                "",
            ))
            .with_text("The result is 42");

        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["content"], "The result is 42");
        assert_eq!(
            spec[0]["reasoning_content"],
            "Thinking through the problem..."
        );

        Ok(())
    }

    #[test]
    fn test_format_messages_with_options_can_omit_reasoning_content() -> anyhow::Result<()> {
        let message = Message::assistant()
            .with_content(MessageContent::thinking(
                "Thinking through the problem...",
                "",
            ))
            .with_text("The result is 42");

        let spec = format_messages_with_options(
            &[message],
            &ImageFormat::OpenAi,
            OpenAiFormatOptions {
                preserve_thinking_context: false,
            },
        );

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["content"], "The result is 42");
        assert!(spec[0].get("reasoning_content").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_preserves_reasoning_content_for_legacy_compat() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "deepseek-reasoner".to_string(),
            context_limit: Some(128000),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };
        let message = Message::assistant()
            .with_content(MessageContent::thinking("preserve this", ""))
            .with_tool_request(
                "tool1",
                Ok(rmcp::model::CallToolRequestParams::new("test_tool")
                    .with_arguments(rmcp::object!({}))),
            );

        let request = create_request(
            &model_config,
            "system",
            &[message],
            &[],
            &ImageFormat::OpenAi,
            true,
        )?;

        assert_eq!(request["messages"][1]["reasoning_content"], "preserve this");

        Ok(())
    }

    #[test]
    fn test_format_messages_carries_thinking_only_chunks_to_tool_call() -> anyhow::Result<()> {
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("think ", "")),
            Message::assistant().with_content(MessageContent::thinking("once", "")),
            Message::assistant().with_tool_request(
                "tool1",
                Ok(CallToolRequestParams::new("test_tool")
                    .with_arguments(object!({"param": "value"}))),
            ),
        ];

        let spec = format_messages_with_options(
            &messages,
            &ImageFormat::OpenAi,
            OpenAiFormatOptions {
                preserve_thinking_context: true,
            },
        );

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["reasoning_content"], "think once");
        assert_eq!(spec[0]["content"], json!(null));
        assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

        Ok(())
    }

    #[test]
    fn test_format_messages_does_not_duplicate_pending_thinking() -> anyhow::Result<()> {
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("think once", "")),
            Message::assistant()
                .with_content(MessageContent::thinking("think once", ""))
                .with_tool_request(
                    "tool1",
                    Ok(CallToolRequestParams::new("test_tool")
                        .with_arguments(object!({"param": "value"}))),
                ),
        ];

        let spec = format_messages_with_options(
            &messages,
            &ImageFormat::OpenAi,
            OpenAiFormatOptions {
                preserve_thinking_context: true,
            },
        );

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["reasoning_content"], "think once");
        assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

        Ok(())
    }

    #[test]
    fn test_format_messages_merges_pending_thinking_with_tool_call_suffix() -> anyhow::Result<()> {
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("think ", "")),
            Message::assistant()
                .with_content(MessageContent::thinking("once", ""))
                .with_tool_request(
                    "tool1",
                    Ok(CallToolRequestParams::new("test_tool")
                        .with_arguments(object!({"param": "value"}))),
                ),
        ];

        let spec = format_messages_with_options(
            &messages,
            &ImageFormat::OpenAi,
            OpenAiFormatOptions {
                preserve_thinking_context: true,
            },
        );

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["reasoning_content"], "think once");
        assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

        Ok(())
    }

    #[test]
    fn test_format_messages_does_not_carry_thinking_across_user_message() -> anyhow::Result<()> {
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("stale", "")),
            Message::user().with_text("new turn"),
            Message::assistant()
                .with_tool_request("tool1", Ok(CallToolRequestParams::new("test_tool"))),
        ];

        let spec = format_messages_with_options(
            &messages,
            &ImageFormat::OpenAi,
            OpenAiFormatOptions {
                preserve_thinking_context: true,
            },
        );

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(spec[1]["role"], "assistant");
        assert!(spec[1].get("reasoning_content").is_none());

        Ok(())
    }

    #[test_case(
        "data: {\"error\":{\"message\":\"Internal server error\",\"type\":\"server_error\",\"code\":500}}\ndata: [DONE]",
        "Internal server error";
        "openai error format"
    )]
    #[test_case(
        "data: {\"object\":\"error\",\"message\":\"CUDA out of memory\",\"code\":500}\ndata: [DONE]",
        "CUDA out of memory";
        "vllm error format"
    )]
    #[test_case(
        "data: {\"error\":{\"message\":\"Rate limit exceeded\",\"type\":\"rate_limit_error\"}}",
        "Rate limit exceeded";
        "error as first chunk"
    )]
    #[tokio::test]
    async fn test_mid_stream_server_error(response_lines: &str, expected_msg: &str) {
        let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));
        let mut found_error = false;
        while let Some(result) = messages.next().await {
            if let Err(e) = result {
                let err_str = e.to_string();
                assert!(
                    err_str.contains(expected_msg),
                    "unexpected error text: {err_str}"
                );
                found_error = true;
                break;
            }
        }
        assert!(
            found_error,
            "expected an error but stream completed successfully"
        );
    }

    #[test]
    fn test_merge_split_tool_calls_with_reasoning() {
        let mut messages = vec![
            json!({"role": "assistant", "tool_calls": [{"id": "tc1", "type": "function", "function": {"name": "read", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
            json!({"role": "tool", "tool_call_id": "tc1", "content": "result1"}),
            json!({"role": "assistant", "tool_calls": [{"id": "tc2", "type": "function", "function": {"name": "write", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
            json!({"role": "tool", "tool_call_id": "tc2", "content": "result2"}),
        ];
        merge_split_tool_call_messages(&mut messages);

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 2);
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[2]["role"], "tool");
    }

    #[test]
    fn test_no_merge_without_reasoning() {
        let mut messages = vec![
            json!({"role": "assistant", "tool_calls": [{"id": "tc1", "type": "function", "function": {"name": "read", "arguments": "{}"}}]}),
            json!({"role": "tool", "tool_call_id": "tc1", "content": "result1"}),
            json!({"role": "assistant", "tool_calls": [{"id": "tc2", "type": "function", "function": {"name": "write", "arguments": "{}"}}]}),
            json!({"role": "tool", "tool_call_id": "tc2", "content": "result2"}),
        ];
        merge_split_tool_call_messages(&mut messages);

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 1);
        assert_eq!(messages[2]["tool_calls"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_merge_split_tool_calls_with_image_gap() {
        let mut messages = vec![
            json!({"role": "assistant", "tool_calls": [{"id": "tc1", "type": "function", "function": {"name": "screenshot", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
            json!({"role": "tool", "tool_call_id": "tc1", "content": "This tool result included an image that is uploaded in the next message."}),
            json!({"role": "user", "content": [{"type": "image_url", "image_url": {"url": "data:image/png;base64,abc"}}]}),
            json!({"role": "assistant", "tool_calls": [{"id": "tc2", "type": "function", "function": {"name": "click", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
            json!({"role": "tool", "tool_call_id": "tc2", "content": "clicked"}),
        ];
        merge_split_tool_call_messages(&mut messages);

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 2);
        assert_eq!(messages[0]["role"], "assistant");
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "tc1");
        assert_eq!(messages[2]["role"], "user");
        assert_eq!(messages[3]["role"], "tool");
        assert_eq!(messages[3]["tool_call_id"], "tc2");
    }

    #[test]
    fn test_merge_does_not_skip_real_user_message() {
        let mut messages = vec![
            json!({"role": "assistant", "tool_calls": [{"id": "tc1", "type": "function", "function": {"name": "read", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
            json!({"role": "tool", "tool_call_id": "tc1", "content": "result1"}),
            json!({"role": "user", "content": "what happened?"}),
            json!({"role": "assistant", "tool_calls": [{"id": "tc2", "type": "function", "function": {"name": "write", "arguments": "{}"}}], "reasoning_content": "thinking..."}),
            json!({"role": "tool", "tool_call_id": "tc2", "content": "result2"}),
        ];
        merge_split_tool_call_messages(&mut messages);

        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 1);
        assert_eq!(messages[2]["role"], "user");
        assert_eq!(messages[2]["content"], "what happened?");
        assert_eq!(messages[3]["tool_calls"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_get_usage_with_ollama_native_fields() {
        // Ollama-native fields should be picked up as fallback
        let usage = json!({
            "prompt_eval_count": 42,
            "eval_count": 128
        });
        let result = get_usage(&usage);
        assert_eq!(result.input_tokens, Some(42));
        assert_eq!(result.output_tokens, Some(128));
        assert_eq!(result.total_tokens, Some(170));
    }

    #[test]
    fn test_get_usage_prefers_openai_fields_over_ollama() {
        // Standard OpenAI fields should take precedence
        let usage = json!({
            "prompt_tokens": 10,
            "completion_tokens": 20,
            "prompt_eval_count": 42,
            "eval_count": 128
        });
        let result = get_usage(&usage);
        assert_eq!(result.input_tokens, Some(10));
        assert_eq!(result.output_tokens, Some(20));
        assert_eq!(result.total_tokens, Some(30));
    }

    #[test]
    fn test_get_usage_falls_back_when_openai_fields_are_null() {
        // When OpenAI fields exist but are null, should fall back to Ollama-native
        let usage = json!({
            "prompt_tokens": null,
            "completion_tokens": null,
            "prompt_eval_count": 42,
            "eval_count": 128
        });
        let result = get_usage(&usage);
        assert_eq!(result.input_tokens, Some(42));
        assert_eq!(result.output_tokens, Some(128));
        assert_eq!(result.total_tokens, Some(170));
    }

    // vLLM serving gpt-oss emits both `reasoning` and `reasoning_content`
    // in the same payload; the non-streaming path handles it fine today.
    #[test]
    fn test_response_to_message_with_both_reasoning_fields() -> anyhow::Result<()> {
        let response = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "answer",
                    "reasoning": "thinking...",
                    "reasoning_content": "thinking..."
                }
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);
        if let MessageContent::Thinking(t) = &message.content[0] {
            assert_eq!(t.thinking, "thinking...");
        } else {
            panic!("Expected Thinking content, got {:?}", message.content[0]);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_streaming_chunk_with_only_reasoning_content() -> anyhow::Result<()> {
        let response_lines = "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"hi\"},\"finish_reason\":null}]}\ndata: [DONE]";
        let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));
        while let Some(result) = messages.next().await {
            result?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_streaming_tool_call_does_not_duplicate_yielded_reasoning() -> anyhow::Result<()> {
        let response_lines = concat!(
            "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"think \"},\"finish_reason\":null}]}\n",
            "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"once\"},\"finish_reason\":null}]}\n",
            "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"test_tool\",\"arguments\":\"\"}}]},\"finish_reason\":null}]}\n",
            "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
            "data: [DONE]"
        );
        let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

        let mut thinking = String::new();
        let mut tool_calls = 0;
        let mut history = Vec::new();
        while let Some(result) = messages.next().await {
            let (message, _usage) = result?;
            if let Some(msg) = message {
                for content in &msg.content {
                    match content {
                        MessageContent::Thinking(t) => thinking.push_str(&t.thinking),
                        MessageContent::ToolRequest(_) => tool_calls += 1,
                        _ => {}
                    }
                }
                history.push(msg);
            }
        }

        assert_eq!(thinking, "think once");
        assert_eq!(tool_calls, 1);

        let spec = format_messages_with_options(
            &history,
            &ImageFormat::OpenAi,
            OpenAiFormatOptions {
                preserve_thinking_context: true,
            },
        );
        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["reasoning_content"], "think once");
        assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

        Ok(())
    }

    #[tokio::test]
    async fn test_streaming_tool_call_merges_yielded_reasoning_with_suffix() -> anyhow::Result<()> {
        let response_lines = concat!(
            "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"think \"},\"finish_reason\":null}]}\n",
            "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"once\",\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"test_tool\",\"arguments\":\"\"}}]},\"finish_reason\":null}]}\n",
            "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
            "data: [DONE]"
        );
        let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

        let mut history = Vec::new();
        while let Some(result) = messages.next().await {
            let (message, _usage) = result?;
            if let Some(msg) = message {
                history.push(msg);
            }
        }

        let spec = format_messages_with_options(
            &history,
            &ImageFormat::OpenAi,
            OpenAiFormatOptions {
                preserve_thinking_context: true,
            },
        );
        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["reasoning_content"], "think once");
        assert_eq!(spec[0]["tool_calls"][0]["function"]["name"], "test_tool");

        Ok(())
    }

    #[tokio::test]
    async fn test_streaming_tool_call_preserves_unyielded_reasoning() -> anyhow::Result<()> {
        let response_lines = concat!(
            "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"tool thought\",\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"test_tool\",\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n",
            "data: [DONE]"
        );
        let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

        let mut thinking = String::new();
        let mut tool_calls = 0;
        while let Some(result) = messages.next().await {
            let (message, _usage) = result?;
            if let Some(msg) = message {
                for content in &msg.content {
                    match content {
                        MessageContent::Thinking(t) => thinking.push_str(&t.thinking),
                        MessageContent::ToolRequest(_) => tool_calls += 1,
                        _ => {}
                    }
                }
            }
        }

        assert_eq!(thinking, "tool thought");
        assert_eq!(tool_calls, 1);
        Ok(())
    }

    // Streaming counterpart: both fields in one delta must parse and yield
    // thinking content, not fail with "duplicate field `reasoning_content`".
    #[tokio::test]
    async fn test_streaming_chunk_with_both_reasoning_fields() -> anyhow::Result<()> {
        let response_lines = "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning\":\"thinking...\",\"reasoning_content\":\"thinking...\"},\"finish_reason\":null}]}\ndata: [DONE]";
        let lines: Vec<String> = response_lines.lines().map(|s| s.to_string()).collect();
        let response_stream = tokio_stream::iter(lines.into_iter().map(Ok));
        let mut messages = std::pin::pin!(response_to_streaming_message(response_stream));

        let mut saw_thinking = false;
        while let Some(result) = messages.next().await {
            let (message, _usage) = result?;
            if let Some(msg) = message {
                for c in &msg.content {
                    if let MessageContent::Thinking(t) = c {
                        assert_eq!(t.thinking, "thinking...");
                        saw_thinking = true;
                    }
                }
            }
        }
        assert!(
            saw_thinking,
            "expected thinking content from merged reasoning fields"
        );
        Ok(())
    }

    #[test]
    fn test_delta_tool_call_function_accepts_null_arguments() {
        let raw = r#"{"arguments":null}"#;
        let parsed: DeltaToolCallFunction =
            serde_json::from_str(raw).expect("null arguments must deserialize");
        assert_eq!(parsed.arguments, "");

        let raw = r#"{}"#;
        let parsed: DeltaToolCallFunction =
            serde_json::from_str(raw).expect("missing arguments must deserialize");
        assert_eq!(parsed.arguments, "");

        let raw = r#"{"arguments":"{\"k\":1}"}"#;
        let parsed: DeltaToolCallFunction = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.arguments, "{\"k\":1}");
    }
}
