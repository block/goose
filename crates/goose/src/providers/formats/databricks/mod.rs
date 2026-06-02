use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::formats::anthropic::{
    thinking_budget_tokens, thinking_effort, thinking_type, ThinkingType,
};
use crate::providers::utils::{
    convert_image, detect_image_path, extract_reasoning_effort, is_openai_responses_model,
    is_valid_function_name, load_image_file, openai_reasoning_effort_for_thinking,
    safely_parse_json, sanitize_function_name, ImageFormat,
};
use anyhow::{anyhow, Error};
use rmcp::model::{
    object, AnnotateAble, CallToolRequestParams, Content, ErrorCode, ErrorData, RawContent,
    ResourceContents, Role, Tool,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::borrow::Cow;

mod messages;
mod request;

pub use messages::*;
pub use request::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::model::CallToolResult;
    use rmcp::object;
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
    fn test_format_messages() -> anyhow::Result<()> {
        let message = Message::user().with_text("Hello");
        let spec = messages::format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0].role, "user");
        assert_eq!(spec[0].content, "Hello");
        Ok(())
    }

    #[test]
    fn test_format_tools() -> anyhow::Result<()> {
        let tool = Tool::new(
            "test_tool",
            "A test tool",
            object!({
                "$schema": "http://json-schema.org/draft-07/schema#",
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

        let spec = format_tools(std::slice::from_ref(&tool), "gpt-4o")?;
        assert_eq!(
            spec[0]["function"]["parameters"]["$schema"],
            "http://json-schema.org/draft-07/schema#"
        );

        let spec = format_tools(std::slice::from_ref(&tool), "gemini-2-5-flash")?;
        assert!(spec[0]["function"].get("parametersJsonSchema").is_some());
        assert_eq!(
            spec[0]["function"]["parametersJsonSchema"]["type"],
            "object"
        );

        let spec = format_tools(&[tool], "databricks-gemini-3-pro")?;
        assert!(spec[0]["function"].get("parametersJsonSchema").is_some());
        assert_eq!(
            spec[0]["function"]["parametersJsonSchema"]["type"],
            "object"
        );

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

        let tool_id = if let MessageContent::ToolRequest(request) = &messages[2].content[0] {
            &request.id
        } else {
            panic!("should be tool request");
        };

        messages.push(Message::user().with_tool_response(
            tool_id,
            Ok(CallToolResult::success(vec![Content::text("Result")])),
        ));

        let as_value =
            serde_json::to_value(messages::format_messages(&messages, &ImageFormat::OpenAi))
                .unwrap();
        let spec = as_value.as_array().unwrap();

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

        let tool_id = if let MessageContent::ToolRequest(request) = &messages[0].content[0] {
            &request.id
        } else {
            panic!("should be tool request");
        };

        messages.push(Message::user().with_tool_response(
            tool_id,
            Ok(CallToolResult::success(vec![Content::text("Result")])),
        ));

        let as_value =
            serde_json::to_value(messages::format_messages(&messages, &ImageFormat::OpenAi))
                .unwrap();
        let spec = as_value.as_array().unwrap();

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

        let result = format_tools(&[tool1, tool2], "gpt-4o");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate tool name"));

        Ok(())
    }

    #[test]
    fn test_format_messages_with_image_path() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let png_path = temp_dir.path().join("test.png");
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, // PNG magic number
            0x0D, 0x0A, 0x1A, 0x0A, // PNG header
            0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
        ];
        std::fs::write(&png_path, png_data)?;
        let png_path_str = png_path.to_str().unwrap();

        // Create message with image path
        let message = Message::user().with_text(format!("Here is an image: {}", png_path_str));
        let as_value =
            serde_json::to_value(messages::format_messages(&[message], &ImageFormat::OpenAi))
                .unwrap();
        let spec = as_value.as_array().unwrap();

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

        Ok(())
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
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "gpt-4o",
            "messages": [
                {
                    "role": "system",
                    "content": "system"
                }
            ],
            "max_completion_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }

        Ok(())
    }

    #[test]
    fn test_create_request_reasoning_effort() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("high"));
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
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["reasoning_effort"], "high");
        Ok(())
    }

    #[test]
    fn test_create_request_off_effort_preserves_none() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("off"));
        let model_config = ModelConfig {
            model_name: "databricks-o3-mini".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: Some(params),
            reasoning: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["reasoning_effort"], "none");
        assert!(request.get("thinking_effort").is_none());
        Ok(())
    }

    #[test]
    fn test_create_request_max_effort_uses_supported_level() -> anyhow::Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("max"));
        let model_config = ModelConfig {
            model_name: "databricks-gpt-5.2-pro".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: Some(params),
            reasoning: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["reasoning_effort"], "high");
        assert!(request.get("thinking_effort").is_none());
        Ok(())
    }

    #[test]
    fn test_create_request_reasoning_effort_xhigh() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "o3-xhigh".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["model"], "o3");
        assert_eq!(request["reasoning_effort"], "xhigh");
        Ok(())
    }

    #[test]
    fn test_create_request_reasoning_effort_none() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "o3-none".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["model"], "o3");
        assert_eq!(request["reasoning_effort"], "none");
        Ok(())
    }

    #[test]
    fn test_create_request_reasoning_effort_for_prefixed_gpt5_model() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "databricks-gpt-5.4-high".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        assert_eq!(request["model"], "databricks-gpt-5.4");
        assert_eq!(request["reasoning_effort"], "high");
        Ok(())
    }

    #[test]
    fn test_create_request_adaptive_thinking_for_46_models() -> anyhow::Result<()> {
        let mut model_config = ModelConfig::new_or_fail("databricks-claude-opus-4-6");
        model_config.max_tokens = Some(4096);
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("low"));
        model_config.request_params = Some(params);

        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;

        assert_eq!(request["thinking"]["type"], "adaptive");
        assert_eq!(request["output_config"]["effort"], "low");
        assert!(request.get("temperature").is_none());
        assert_eq!(request["max_completion_tokens"], 4096);
        assert!(request.get("max_tokens").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_enabled_thinking_with_budget() -> anyhow::Result<()> {
        let mut model_config = ModelConfig::new_or_fail("databricks-claude-3-7-sonnet");
        model_config.max_tokens = Some(4096);
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), serde_json::json!("high"));
        model_config.request_params = Some(params);

        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;

        assert_eq!(request["thinking"]["type"], "enabled");
        assert_eq!(request["thinking"]["budget_tokens"], 16000);
        assert_eq!(request["max_tokens"], 20096);
        assert_eq!(request["temperature"], 2);
        assert!(request.get("max_completion_tokens").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_enabled_thinking_budget_tracks_effort() -> anyhow::Result<()> {
        for (effort, expected_budget) in [
            ("low", 4000),
            ("medium", 10000),
            ("high", 16000),
            ("max", 32000),
        ] {
            let mut model_config = ModelConfig::new_or_fail("databricks-claude-3-7-sonnet");
            model_config.max_tokens = Some(4096);
            let mut params = std::collections::HashMap::new();
            params.insert("thinking_effort".to_string(), serde_json::json!(effort));
            model_config.request_params = Some(params);

            let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;

            assert_eq!(request["thinking"]["type"], "enabled");
            assert_eq!(request["thinking"]["budget_tokens"], expected_budget);
            assert_eq!(request["max_tokens"], 4096 + expected_budget);
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_claude_thinking() -> anyhow::Result<()> {
        let response = json!({
            "model": "us.anthropic.claude-3-7-sonnet-20250219-v1:0",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "reasoning",
                            "summary": [
                                {
                                    "type": "summary_text",
                                    "text": "Test thinking content",
                                    "signature": "test-signature"
                                }
                            ]
                        },
                        {
                            "type": "text",
                            "text": "Regular text content"
                        }
                    ]
                },
                "index": 0,
                "finish_reason": "stop"
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        if let MessageContent::Thinking(thinking) = &message.content[0] {
            assert_eq!(thinking.thinking, "Test thinking content");
            assert_eq!(thinking.signature, "test-signature");
        } else {
            panic!("Expected Thinking content");
        }

        if let MessageContent::Text(text) = &message.content[1] {
            assert_eq!(text.text, "Regular text content");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_claude_encrypted_thinking() -> anyhow::Result<()> {
        let response = json!({
            "model": "claude-3-7-sonnet-20250219",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "reasoning",
                            "summary": [
                                {
                                    "type": "summary_encrypted_text",
                                    "data": "E23sQFCkYIARgCKkATCHitsdf327Ber3v4NYUq2"
                                }
                            ]
                        },
                        {
                            "type": "text",
                            "text": "Regular text content"
                        }
                    ]
                },
                "index": 0,
                "finish_reason": "stop"
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        if let MessageContent::RedactedThinking(redacted) = &message.content[0] {
            assert_eq!(redacted.data, "E23sQFCkYIARgCKkATCHitsdf327Ber3v4NYUq2");
        } else {
            panic!("Expected RedactedThinking content");
        }

        if let MessageContent::Text(text) = &message.content[1] {
            assert_eq!(text.text, "Regular text content");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_format_messages_tool_request_with_none_arguments() -> anyhow::Result<()> {
        // Test that tool calls with None arguments are formatted as "{}" string
        let message = Message::assistant()
            .with_tool_request("tool1", Ok(CallToolRequestParams::new("test_tool")));

        let spec = messages::format_messages(&[message], &ImageFormat::OpenAi);
        let as_value = serde_json::to_value(spec)?;
        let spec_array = as_value.as_array().unwrap();

        assert_eq!(spec_array.len(), 1);
        assert_eq!(spec_array[0]["role"], "assistant");
        assert!(spec_array[0]["tool_calls"].is_array());

        let tool_call = &spec_array[0]["tool_calls"][0];
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

        let spec = messages::format_messages(&[message], &ImageFormat::OpenAi);
        let as_value = serde_json::to_value(spec)?;
        let spec_array = as_value.as_array().unwrap();

        assert_eq!(spec_array.len(), 1);
        assert_eq!(spec_array[0]["role"], "assistant");
        assert!(spec_array[0]["tool_calls"].is_array());

        let tool_call = &spec_array[0]["tool_calls"][0];
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
    fn test_is_claude_model() {
        assert!(messages::is_claude_model("databricks-claude-sonnet-4"));
        assert!(messages::is_claude_model("databricks-claude-3-7-sonnet"));
        assert!(messages::is_claude_model("claude-sonnet-4"));
        assert!(messages::is_claude_model("goose-claude-sonnet"));
        assert!(!messages::is_claude_model("gpt-4o"));
        assert!(!messages::is_claude_model("gemini-2-5-flash"));
        assert!(!messages::is_claude_model("databricks-meta-llama-3-3-70b"));
    }

    #[test]
    fn test_apply_cache_control_for_claude_system_message() -> anyhow::Result<()> {
        let mut payload = json!({
            "model": "databricks-claude-sonnet-4",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a helpful assistant."
                },
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        });

        apply_cache_control_for_claude(&mut payload);

        let messages = payload["messages"].as_array().unwrap();
        let system_msg = &messages[0];

        // System message content should be converted to array with cache_control
        assert!(system_msg["content"].is_array());
        let content = system_msg["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "You are a helpful assistant.");
        assert_eq!(content[0]["cache_control"]["type"], "ephemeral");

        Ok(())
    }

    #[test]
    fn test_apply_cache_control_for_claude_user_messages() -> anyhow::Result<()> {
        let mut payload = json!({
            "model": "databricks-claude-sonnet-4",
            "messages": [
                {
                    "role": "system",
                    "content": "You are helpful"
                },
                {
                    "role": "user",
                    "content": "First question"
                },
                {
                    "role": "assistant",
                    "content": "First answer"
                },
                {
                    "role": "user",
                    "content": "Second question"
                },
                {
                    "role": "assistant",
                    "content": "Second answer"
                },
                {
                    "role": "user",
                    "content": "Third question"
                }
            ]
        });

        apply_cache_control_for_claude(&mut payload);

        let messages = payload["messages"].as_array().unwrap();

        // First user message should NOT have cache_control (only last 2)
        let first_user = &messages[1];
        assert_eq!(first_user["content"], "First question");

        // Second-to-last user message should have cache_control
        let second_user = &messages[3];
        assert!(second_user["content"].is_array());
        assert_eq!(
            second_user["content"][0]["cache_control"]["type"],
            "ephemeral"
        );

        // Last user message should have cache_control
        let last_user = &messages[5];
        assert!(last_user["content"].is_array());
        assert_eq!(
            last_user["content"][0]["cache_control"]["type"],
            "ephemeral"
        );

        Ok(())
    }

    #[test]
    fn test_apply_cache_control_for_claude_tools() -> anyhow::Result<()> {
        let mut payload = json!({
            "model": "databricks-claude-sonnet-4",
            "messages": [
                {
                    "role": "system",
                    "content": "You are helpful"
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "tool1",
                        "description": "First tool"
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "tool2",
                        "description": "Second tool"
                    }
                }
            ]
        });

        apply_cache_control_for_claude(&mut payload);

        let tools = payload["tools"].as_array().unwrap();

        // First tool should NOT have cache_control
        assert!(tools[0]["function"].get("cache_control").is_none());

        // Last tool should have cache_control
        assert_eq!(tools[1]["function"]["cache_control"]["type"], "ephemeral");

        Ok(())
    }

    #[test]
    fn test_format_messages_with_thought_signature_metadata() -> anyhow::Result<()> {
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "thoughtSignature".to_string(),
            json!("sig_abc123_test_signature"),
        );

        let message = Message::assistant().with_tool_request_with_metadata(
            "tool1",
            Ok(CallToolRequestParams::new("test_tool").with_arguments(object!({"param": "value"}))),
            Some(&metadata),
            None,
        );

        let spec = messages::format_messages(&[message], &ImageFormat::OpenAi);
        let as_value = serde_json::to_value(spec)?;
        let spec_array = as_value.as_array().unwrap();

        assert_eq!(spec_array.len(), 1);
        let tool_call = &spec_array[0]["tool_calls"][0];
        assert_eq!(tool_call["id"], "tool1");
        assert_eq!(tool_call["function"]["name"], "test_tool");
        assert_eq!(tool_call["thoughtSignature"], "sig_abc123_test_signature");

        Ok(())
    }

    #[test]
    fn test_create_request_claude_has_cache_control() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "databricks-claude-sonnet-4".to_string(),
            context_limit: Some(200000),
            temperature: None,
            max_tokens: Some(8192),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };

        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
            Message::user().with_text("How are you?"),
        ];

        let tool = Tool::new(
            "test_tool",
            "A test tool",
            object!({
                "type": "object",
                "properties": {}
            }),
        );

        let request = create_request(
            &model_config,
            "You are helpful",
            &messages,
            &[tool],
            &ImageFormat::OpenAi,
        )?;

        // Verify system message has cache_control
        let messages_arr = request["messages"].as_array().unwrap();
        let system_msg = &messages_arr[0];
        assert!(system_msg["content"].is_array());
        assert_eq!(
            system_msg["content"][0]["cache_control"]["type"],
            "ephemeral"
        );

        // Verify last tool has cache_control
        let tools = request["tools"].as_array().unwrap();
        assert_eq!(tools[0]["function"]["cache_control"]["type"], "ephemeral");

        Ok(())
    }

    #[test]
    fn test_create_request_non_claude_no_cache_control() -> anyhow::Result<()> {
        let model_config = ModelConfig {
            model_name: "gpt-4o".to_string(),
            context_limit: Some(128000),
            temperature: None,
            max_tokens: Some(4096),
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };

        let messages = vec![Message::user().with_text("Hello")];

        let tool = Tool::new(
            "test_tool",
            "A test tool",
            object!({
                "type": "object",
                "properties": {}
            }),
        );

        let request = create_request(
            &model_config,
            "You are helpful",
            &messages,
            &[tool],
            &ImageFormat::OpenAi,
        )?;

        // Verify system message does NOT have cache_control (it's a plain string)
        let messages_arr = request["messages"].as_array().unwrap();
        let system_msg = &messages_arr[0];
        assert!(system_msg["content"].is_string());

        // Verify tool does NOT have cache_control
        let tools = request["tools"].as_array().unwrap();
        assert!(tools[0]["function"].get("cache_control").is_none());

        Ok(())
    }

    #[test]
    fn test_format_messages_with_multiple_metadata_fields() -> anyhow::Result<()> {
        let mut metadata = serde_json::Map::new();
        metadata.insert("thoughtSignature".to_string(), json!("sig_top_level"));
        metadata.insert(
            "extra_content".to_string(),
            json!({
                "google": {
                    "thought_signature": "sig_nested"
                }
            }),
        );
        metadata.insert("custom_field".to_string(), json!("custom_value"));

        let message = Message::assistant().with_tool_request_with_metadata(
            "tool1",
            Ok(CallToolRequestParams::new("test_tool")),
            Some(&metadata),
            None,
        );

        let spec = messages::format_messages(&[message], &ImageFormat::OpenAi);
        let as_value = serde_json::to_value(spec)?;
        let spec_array = as_value.as_array().unwrap();

        let tool_call = &spec_array[0]["tool_calls"][0];
        assert_eq!(tool_call["thoughtSignature"], "sig_top_level");
        assert_eq!(
            tool_call["extra_content"]["google"]["thought_signature"],
            "sig_nested"
        );
        assert_eq!(tool_call["custom_field"], "custom_value");

        Ok(())
    }

    #[test]
    fn test_parallel_tool_responses_with_images_are_consecutive() -> anyhow::Result<()> {
        // Regression: #7449 — parallel tool calls with images must keep tool messages consecutive.
        let messages = vec![
            Message::assistant()
                .with_tool_request("id1", Ok(CallToolRequestParams::new("tool_a")))
                .with_tool_request("id2", Ok(CallToolRequestParams::new("tool_b"))),
            Message::user()
                .with_tool_response(
                    "id1",
                    Ok(CallToolResult::success(vec![Content::image(
                        "base64data1".to_string(),
                        "image/png".to_string(),
                    )])),
                )
                .with_tool_response(
                    "id2",
                    Ok(CallToolResult::success(vec![Content::image(
                        "base64data2".to_string(),
                        "image/png".to_string(),
                    )])),
                ),
        ];

        let as_value =
            serde_json::to_value(messages::format_messages(&messages, &ImageFormat::OpenAi))
                .unwrap();
        let spec = as_value.as_array().unwrap();
        let roles: Vec<&str> = spec.iter().map(|m| m["role"].as_str().unwrap()).collect();

        // Without the fix this was ["assistant", "tool", "user", "tool", "user"].
        assert_eq!(roles, vec!["assistant", "tool", "tool", "user", "user"]);

        Ok(())
    }

    #[test]
    fn test_mixed_tool_responses_image_and_text_ordering() -> anyhow::Result<()> {
        // Mixed case: only one tool response has an image.
        let messages = vec![
            Message::assistant()
                .with_tool_request("id1", Ok(CallToolRequestParams::new("tool_a")))
                .with_tool_request("id2", Ok(CallToolRequestParams::new("tool_b"))),
            Message::user()
                .with_tool_response(
                    "id1",
                    Ok(CallToolResult::success(vec![Content::text("text result")])),
                )
                .with_tool_response(
                    "id2",
                    Ok(CallToolResult::success(vec![Content::image(
                        "base64data".to_string(),
                        "image/png".to_string(),
                    )])),
                ),
        ];

        let as_value =
            serde_json::to_value(messages::format_messages(&messages, &ImageFormat::OpenAi))
                .unwrap();
        let spec = as_value.as_array().unwrap();
        let roles: Vec<&str> = spec.iter().map(|m| m["role"].as_str().unwrap()).collect();

        assert_eq!(roles, vec!["assistant", "tool", "tool", "user"]);

        Ok(())
    }
}
