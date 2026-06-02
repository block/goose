use crate::conversation::message::{Message, MessageContent};
use crate::mcp_utils::extract_text_from_resource;
use crate::model::{ModelConfig, ThinkingEffort};
use crate::providers::base::Usage;
use crate::providers::errors::ProviderError;
use crate::providers::utils::{convert_image, ImageFormat};
use anyhow::{anyhow, Result};
use rmcp::model::{object, CallToolRequestParams, ErrorCode, ErrorData, JsonObject, Role, Tool};
use rmcp::object as json_object;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

mod messages;
mod streaming;
mod thinking;
mod types;

pub use messages::*;
pub use streaming::*;
pub use thinking::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use crate::model::ModelConfig;
    use rmcp::object;
    use serde_json::json;

    #[test]
    fn test_parse_text_response() -> Result<()> {
        let response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Hello! How can I assist you today?"
            }],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 12,
                "output_tokens": 15,
                "cache_creation_input_tokens": 12,
                "cache_read_input_tokens": 0
            }
        });

        let message = response_to_message(&response)?;
        let usage = get_usage(&response)?;

        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello! How can I assist you today?");
        } else {
            panic!("Expected Text content");
        }

        assert_eq!(usage.input_tokens, Some(24)); // 12 + 12 = 24 actual tokens
        assert_eq!(usage.output_tokens, Some(15));
        assert_eq!(usage.total_tokens, Some(39)); // 24 + 15

        Ok(())
    }

    #[test]
    fn test_parse_tool_response() -> Result<()> {
        let response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "tool_use",
                "id": "tool_1",
                "name": "calculator",
                "input": {
                    "expression": "2 + 2"
                }
            }],
            "model": "claude-3-sonnet-20240229",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 15,
                "output_tokens": 20,
                "cache_creation_input_tokens": 15,
                "cache_read_input_tokens": 0,
            }
        });

        let message = response_to_message(&response)?;
        let usage = get_usage(&response)?;

        if let MessageContent::ToolRequest(tool_request) = &message.content[0] {
            let tool_call = tool_request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "calculator");
            assert_eq!(tool_call.arguments, Some(object!({"expression": "2 + 2"})));
        } else {
            panic!("Expected ToolRequest content");
        }

        assert_eq!(usage.input_tokens, Some(30)); // 15 + 15 = 30 actual tokens
        assert_eq!(usage.output_tokens, Some(20));
        assert_eq!(usage.total_tokens, Some(50)); // 30 + 20

        Ok(())
    }

    #[test]
    fn test_parse_unsigned_thinking_response() -> Result<()> {
        let response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "thinking",
                "thinking": "internal reasoning"
            }],
            "model": "glm-4.7",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 12,
                "output_tokens": 15
            }
        });

        let message = response_to_message(&response)?;

        if let MessageContent::Thinking(thinking) = &message.content[0] {
            assert_eq!(thinking.thinking, "internal reasoning");
            assert_eq!(thinking.signature, "");
        } else {
            panic!("Expected Thinking content");
        }

        Ok(())
    }

    #[test]
    fn test_message_to_anthropic_spec() {
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there"),
            Message::user().with_text("How are you?"),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 3);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(spec[0]["content"][0]["type"], "text");
        assert_eq!(spec[0]["content"][0]["text"], "Hello");
        assert_eq!(spec[1]["role"], "assistant");
        assert_eq!(spec[1]["content"][0]["text"], "Hi there");
        assert_eq!(spec[2]["role"], "user");
        assert_eq!(spec[2]["content"][0]["text"], "How are you?");
    }

    #[test]
    fn test_message_to_anthropic_spec_skips_unsigned_thinking() {
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("internal", "")),
            Message::assistant().with_text("Hi there"),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"][0]["type"], "text");
        assert_eq!(spec[0]["content"][0]["text"], "Hi there");
    }

    #[test]
    fn test_message_to_anthropic_spec_preserves_unsigned_thinking_when_enabled() {
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("internal", "")),
            Message::assistant().with_text("Hi there"),
        ];

        let spec = format_messages_with_options(
            &messages,
            AnthropicFormatOptions {
                preserve_unsigned_thinking: true,
                preserve_thinking_context: false,
            },
        );

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"][0]["type"], "thinking");
        assert_eq!(spec[0]["content"][0]["thinking"], "internal");
        assert!(spec[0]["content"][0].get("signature").is_none());
        assert_eq!(spec[1]["content"][0]["text"], "Hi there");
    }

    #[test]
    fn test_tools_to_anthropic_spec() {
        let tools = vec![
            Tool::new(
                "calculator",
                "Calculate mathematical expressions",
                object!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "The mathematical expression to evaluate"
                        }
                    }
                }),
            ),
            Tool::new(
                "weather",
                "Get weather information",
                object!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The location to get weather for"
                        }
                    }
                }),
            ),
        ];

        let spec = format_tools(&tools);

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[0]["name"], "calculator");
        assert_eq!(spec[0]["description"], "Calculate mathematical expressions");
        assert_eq!(spec[1]["name"], "weather");
        assert_eq!(spec[1]["description"], "Get weather information");

        // Verify cache control is added to last tool
        assert!(spec[1].get("cache_control").is_some());
    }

    #[test]
    fn test_system_to_anthropic_spec() {
        let system = "You are a helpful assistant.";
        let spec = format_system(system);

        assert!(spec.is_array());
        let spec_array = spec.as_array().unwrap();
        assert_eq!(spec_array.len(), 1);
        assert_eq!(spec_array[0]["type"], "text");
        assert_eq!(spec_array[0]["text"], system);
        assert!(spec_array[0].get("cache_control").is_some());
    }

    #[test]
    fn test_cache_pricing_calculation() -> Result<()> {
        // Test realistic cache scenario: small fresh input, large cached content
        let response = json!({
            "id": "msg_cache_test",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Based on the cached context, here's my response."
            }],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 7,        // Small fresh input
                "output_tokens": 50,      // Output tokens
                "cache_creation_input_tokens": 10000, // Large cache creation
                "cache_read_input_tokens": 5000       // Large cache read
            }
        });

        let usage = get_usage(&response)?;

        // ACTUAL input tokens should be:
        // 7 + 10000 + 5000 = 15007 total actual tokens
        assert_eq!(usage.input_tokens, Some(15007));
        assert_eq!(usage.output_tokens, Some(50));
        assert_eq!(usage.total_tokens, Some(15057)); // 15007 + 50

        Ok(())
    }

    #[test]
    fn test_create_request_adaptive_thinking_for_46_models() -> Result<()> {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);

        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!("high"));

        let mut config = cfg("claude-opus-4-6");
        config.max_tokens = Some(4096);
        config.request_params = Some(params);
        let messages = vec![Message::user().with_text("Hello")];
        let payload = create_request(&config, "system", &messages, &[])?;

        assert_eq!(payload["thinking"]["type"], "adaptive");
        assert_eq!(payload["output_config"]["effort"], "high");
        assert!(payload.get("budget_tokens").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_enabled_thinking_with_budget() -> Result<()> {
        let _guard = env_lock::lock_env([
            ("GOOSE_THINKING_EFFORT", None::<&str>),
            ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
        ]);

        let mut config = cfg_with_effort("claude-3-7-sonnet-20250219", "high");
        config.max_tokens = Some(4096);

        let messages = vec![Message::user().with_text("Hello")];
        let payload = create_request(&config, "system", &messages, &[])?;

        assert_eq!(payload["thinking"]["type"], "enabled");
        let budget = payload["thinking"]["budget_tokens"].as_i64().unwrap();
        assert!(budget > 0);
        assert_eq!(payload["max_tokens"], 4096 + budget);

        Ok(())
    }

    #[test]
    fn test_create_request_disabled_thinking_no_thinking_field() -> Result<()> {
        let _guard = env_lock::lock_env([
            ("GOOSE_THINKING_EFFORT", None::<&str>),
            ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
        ]);

        let config = cfg_with_effort("claude-sonnet-4-20250514", "off");
        let messages = vec![Message::user().with_text("Hello")];
        let payload = create_request(&config, "system", &messages, &[])?;

        assert!(payload.get("thinking").is_none());
        assert!(payload.get("output_config").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_preserves_thinking_context_for_compatible_models() -> Result<()> {
        let _guard = env_lock::lock_env([
            ("CLAUDE_THINKING_TYPE", None::<&str>),
            ("CLAUDE_THINKING_ENABLED", None::<&str>),
            ("ANTHROPIC_THINKING_BUDGET", None::<&str>),
            ("CLAUDE_THINKING_BUDGET", None::<&str>),
            ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
            ("ANTHROPIC_PRESERVE_UNSIGNED_THINKING", None::<&str>),
        ]);

        let mut config = cfg("glm-4.7");
        config.max_tokens = Some(4096);
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("internal", "")),
            Message::user().with_text("Continue"),
        ];

        let payload = create_request_with_options(
            &config,
            "system",
            &messages,
            &[],
            AnthropicFormatOptions {
                preserve_unsigned_thinking: true,
                preserve_thinking_context: true,
            },
        )?;

        assert_eq!(payload["thinking"]["type"], "enabled");
        assert_eq!(payload["thinking"]["budget_tokens"], 16000);
        assert_eq!(payload["thinking"]["clear_thinking"], false);
        assert_eq!(payload["max_tokens"], 4096 + 16000);
        assert_eq!(payload["messages"][0]["content"][0]["type"], "thinking");
        assert_eq!(payload["messages"][0]["content"][0]["thinking"], "internal");
        assert!(payload["messages"][0]["content"][0]
            .get("signature")
            .is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_model_params_enable_preserved_thinking_context() -> Result<()> {
        let _guard = env_lock::lock_env([
            ("CLAUDE_THINKING_TYPE", None::<&str>),
            ("CLAUDE_THINKING_ENABLED", None::<&str>),
            ("ANTHROPIC_THINKING_BUDGET", None::<&str>),
            ("CLAUDE_THINKING_BUDGET", None::<&str>),
            ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
            ("ANTHROPIC_PRESERVE_UNSIGNED_THINKING", None::<&str>),
        ]);

        let mut params = std::collections::HashMap::new();
        params.insert("preserve_thinking_context".to_string(), json!(true));

        let mut config = cfg("glm-4.7");
        config.request_params = Some(params);
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("internal", "")),
            Message::user().with_text("Continue"),
        ];

        let payload = create_request(&config, "system", &messages, &[])?;

        assert_eq!(payload["thinking"]["clear_thinking"], false);
        assert_eq!(payload["messages"][0]["content"][0]["type"], "thinking");
        assert_eq!(payload["messages"][0]["content"][0]["thinking"], "internal");

        Ok(())
    }

    #[test]
    fn test_tool_error_handling_maintains_pairing() {
        use crate::conversation::message::Message;
        use rmcp::model::{ErrorCode, ErrorData};

        let messages = vec![
            Message::assistant().with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("calculator")
                    .with_arguments(object!({"expression": "2 + 2"}))),
            ),
            Message::user().with_tool_response(
                "tool_1",
                Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Tool failed".to_string(),
                    None,
                )),
            ),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 2);

        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"][0]["type"], "tool_use");
        assert_eq!(spec[0]["content"][0]["id"], "tool_1");
        assert_eq!(spec[0]["content"][0]["name"], "calculator");

        assert_eq!(spec[1]["role"], "user");
        assert_eq!(spec[1]["content"][0]["type"], "tool_result");
        assert_eq!(spec[1]["content"][0]["tool_use_id"], "tool_1");
        assert_eq!(
            spec[1]["content"][0]["content"],
            "Error: -32603: Tool failed"
        );
        assert_eq!(spec[1]["content"][0]["is_error"], true);
    }

    #[test]
    fn test_whitespace_only_text_blocks_are_skipped() {
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("").with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("search").with_arguments(object!({"query": "test"}))),
            ),
            Message::user()
                .with_tool_response("tool_1", Ok(rmcp::model::CallToolResult::success(vec![]))),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 3);

        let assistant_content = spec[1]["content"].as_array().unwrap();
        assert_eq!(assistant_content.len(), 1);
        assert_eq!(assistant_content[0]["type"], "tool_use");
    }

    #[test]
    fn test_tool_response_with_resource_content() {
        use rmcp::model::{CallToolResult, Content};

        let resource_content = Content::embedded_text(
            "file:///test/file.txt",
            "This is the file content from a resource",
        );

        let messages = vec![
            Message::assistant().with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("view_file")
                    .with_arguments(object!({"path": "/test/file.txt"}))),
            ),
            Message::user().with_tool_response(
                "tool_1",
                Ok(CallToolResult::success(vec![resource_content])),
            ),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[1]["role"], "user");
        assert_eq!(spec[1]["content"][0]["type"], "tool_result");
        assert_eq!(spec[1]["content"][0]["tool_use_id"], "tool_1");
        assert_eq!(
            spec[1]["content"][0]["content"],
            "This is the file content from a resource"
        );
    }

    #[test]
    fn test_tool_response_with_mixed_content() {
        use rmcp::model::{CallToolResult, Content};

        let text_content = Content::text("Summary: file loaded");
        let resource_content = Content::embedded_text("file:///test/file.txt", "File content here");

        let messages = vec![
            Message::assistant().with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("view_file")
                    .with_arguments(object!({"path": "/test/file.txt"}))),
            ),
            Message::user().with_tool_response(
                "tool_1",
                Ok(CallToolResult::success(vec![
                    text_content,
                    resource_content,
                ])),
            ),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec[1]["content"][0]["type"], "tool_result");
        assert_eq!(
            spec[1]["content"][0]["content"],
            "Summary: file loaded\nFile content here"
        );
    }

    #[test]
    fn test_args_to_input_value_returns_empty_object_for_none() {
        let value = messages::args_to_input_value(None);
        assert!(value.is_object(), "expected JSON object, got {value:?}");
        assert_eq!(value, json!({}));
        assert!(!value.is_null());
    }

    #[test]
    fn test_args_to_input_value_preserves_existing_args() {
        let args = object!({"query": "rust"});
        let value = messages::args_to_input_value(Some(args));
        assert_eq!(value, json!({"query": "rust"}));
    }

    #[test]
    fn test_parameterless_tool_request_serializes_input_as_empty_object() {
        // Regression test for #9287: when arguments is None (parameterless
        // MCP tool, session reload, or provider switching) the `input` field
        // must serialize as `{}` so the Anthropic API does not reject the
        // replayed tool_use block with a 400 error.
        let messages = vec![
            Message::assistant()
                .with_tool_request("tool_1", Ok(CallToolRequestParams::new("list_things"))),
            Message::user()
                .with_tool_response("tool_1", Ok(rmcp::model::CallToolResult::success(vec![]))),
        ];

        let spec = format_messages(&messages);

        let input = &spec[0]["content"][0]["input"];
        assert!(input.is_object(), "expected object, got {input:?}");
        assert!(!input.is_null());
        assert_eq!(input, &json!({}));
    }

    #[test]
    fn test_parameterless_frontend_tool_request_serializes_input_as_empty_object() {
        // Same regression as above, but exercises the FrontendToolRequest
        // branch which is reached for UI-originated tool calls.
        let messages = vec![Message::assistant().with_frontend_tool_request(
            "frontend_tool_1",
            Ok(CallToolRequestParams::new("list_things")),
        )];

        let spec = format_messages(&messages);

        let input = &spec[0]["content"][0]["input"];
        assert!(input.is_object(), "expected object, got {input:?}");
        assert!(!input.is_null());
        assert_eq!(input, &json!({}));
    }

    fn cfg(name: &str) -> ModelConfig {
        ModelConfig {
            model_name: name.to_string(),
            ..Default::default()
        }
    }

    fn cfg_with_effort(name: &str, effort: &str) -> ModelConfig {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!(effort));
        ModelConfig {
            model_name: name.to_string(),
            request_params: Some(params),
            ..Default::default()
        }
    }

    #[test]
    fn test_thinking_type_from_effort() {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);
        // Adaptive model with effort → adaptive
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-opus-4-6", "high")),
            ThinkingType::Adaptive
        );
        // Adaptive model with off → disabled
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-opus-4-6", "off")),
            ThinkingType::Disabled
        );
        // Non-adaptive Claude with effort → enabled
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-3-7-sonnet-20250219", "high")),
            ThinkingType::Enabled
        );
        // Non-adaptive Claude with off → disabled
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-3-7-sonnet-20250219", "off")),
            ThinkingType::Disabled
        );
    }

    #[test]
    fn test_thinking_budget_uses_legacy_env() {
        let _guard = env_lock::lock_env([
            ("GOOSE_THINKING_EFFORT", None::<&str>),
            ("ANTHROPIC_THINKING_BUDGET", Some("8192")),
            ("CLAUDE_THINKING_BUDGET", None::<&str>),
        ]);
        let config = cfg_with_effort("claude-3-7-sonnet-20250219", "high");
        assert_eq!(thinking_budget_tokens(&config), 8192);
    }

    #[test]
    fn test_thinking_type_non_claude_always_disabled() {
        assert_eq!(
            thinking_type(&cfg_with_effort("gpt-4o", "off")),
            ThinkingType::Disabled
        );
        assert_eq!(
            thinking_type(&cfg_with_effort("gpt-4o", "high")),
            ThinkingType::Disabled
        );
    }

    #[test]
    fn test_thinking_type_off_means_disabled() {
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-opus-4-6", "off")),
            ThinkingType::Disabled
        );
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-3-7-sonnet-20250219", "off")),
            ThinkingType::Disabled
        );
    }

    #[derive(Default)]
    struct StreamedParts {
        thinking: Vec<(String, String)>,
        redacted_thinking: Vec<String>,
        text: Vec<String>,
        tool_calls: Vec<String>,
    }

    async fn collect_stream(events: &str) -> StreamedParts {
        use futures::StreamExt;

        let lines: Vec<Result<String, anyhow::Error>> =
            events.lines().map(|l| Ok(l.to_string())).collect();
        let stream = Box::pin(futures::stream::iter(lines));
        let mut msg_stream = std::pin::pin!(response_to_streaming_message(stream));
        let mut parts = StreamedParts::default();

        while let Some(Ok((message, _usage))) = msg_stream.next().await {
            if let Some(msg) = message {
                for c in &msg.content {
                    match c {
                        MessageContent::Thinking(t) => {
                            parts
                                .thinking
                                .push((t.thinking.clone(), t.signature.clone()));
                        }
                        MessageContent::RedactedThinking(r) => {
                            parts.redacted_thinking.push(r.data.clone());
                        }
                        MessageContent::Text(t) => {
                            parts.text.push(t.text.clone());
                        }
                        MessageContent::ToolRequest(req) => {
                            if let Ok(call) = &req.tool_call {
                                parts.tool_calls.push(call.name.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        parts
    }

    #[tokio::test]
    async fn test_streaming_thinking_and_text() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me analyze"}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":" this problem."}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig_abc"}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"123"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Here is the answer."}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":1}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.thinking.len(), 1);
        assert_eq!(parts.thinking[0].0, "Let me analyze this problem.");
        assert_eq!(parts.thinking[0].1, "sig_abc123");
        assert_eq!(parts.text, vec!["Here is the answer."]);
    }

    #[tokio::test]
    async fn test_streaming_thinking_from_start_block_without_signature() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":10,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"Initial reasoning "}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"continues."}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.thinking.len(), 1);
        assert_eq!(parts.thinking[0].0, "Initial reasoning continues.");
        assert_eq!(parts.thinking[0].1, "");
    }

    #[tokio::test]
    async fn test_streaming_redacted_thinking() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_2","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":5,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"redacted_thinking","data":"opaque_base64_data"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Done."}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":1}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":10}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.redacted_thinking, vec!["opaque_base64_data"]);
        assert_eq!(parts.text, vec!["Done."]);
    }

    #[tokio::test]
    async fn test_streaming_thinking_text_then_tool_call() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_3","role":"assistant","content":[],"model":"claude-sonnet-4-6","usage":{"input_tokens":8,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"I should search for this."}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"tool_sig_xyz"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Let me search for that."}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":1}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"tool_1","name":"search","input":{}}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"query\":\"rust\"}"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":2}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":15}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.thinking.len(), 1);
        assert_eq!(
            parts.thinking[0],
            (
                "I should search for this.".to_string(),
                "tool_sig_xyz".to_string()
            )
        );
        assert_eq!(parts.text, vec!["Let me search for that."]);
        assert_eq!(parts.tool_calls, vec!["search"]);
    }
}
