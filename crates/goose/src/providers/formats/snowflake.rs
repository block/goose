use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::Usage;
use crate::providers::errors::ProviderError;
use crate::providers::formats::snowflake;
use anyhow::{anyhow, Result};
use mcp_core::content::Content;
use mcp_core::role::Role;
use mcp_core::tool::{Tool, ToolCall};
// Ensure TextContent is imported from the correct module
use mcp_core::content::TextContent;
use serde_json::{json, Value};
use std::collections::HashSet;

/// Convert internal Message format to Snowflake's API message specification
pub fn format_messages(messages: &[Message]) -> Vec<Value> {
    let mut snowflake_messages = Vec::new();

    // Convert messages to Snowflake format
    for message in messages {
        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let mut content = Vec::new();
        for msg_content in &message.content {
            match msg_content {
                MessageContent::Text(text) => {
                    content.push(json!({
                        "content": text.text
                    }));
                }
                MessageContent::ToolRequest(tool_request) => {
                    if let Ok(tool_call) = &tool_request.tool_call {
                        content.push(json!({
                            "type": "tool_use",
                            "id": tool_request.id,
                            "name": tool_call.name,
                            "input": tool_call.arguments
                        }));
                    }
                }
                MessageContent::ToolResponse(tool_response) => {
                    if let Ok(result) = &tool_response.tool_result {
                        let text = result
                            .iter()
                            .filter_map(|c| match c {
                                Content::Text(t) => Some(t.text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n");

                        content.push(json!({
                            "type": "tool_result",
                            "tool_use_id": tool_response.id,
                            "content": text
                        }));
                    }
                }
                MessageContent::ToolConfirmationRequest(_tool_confirmation_request) => {
                    // Skip tool confirmation requests
                }
                MessageContent::EnableExtensionRequest(_enable_extension_request) => {
                    // Skip enable extension requests
                }
                MessageContent::Thinking(thinking) => {
                    content.push(json!({
                        "type": "thinking",
                        "thinking": thinking.thinking,
                        "signature": thinking.signature
                    }));
                }
                MessageContent::RedactedThinking(redacted) => {
                    content.push(json!({
                        "type": "redacted_thinking",
                        "data": redacted.data
                    }));
                }
                MessageContent::Image(_) => continue, // Snowflake doesn't support image content yet
                MessageContent::FrontendToolRequest(tool_request) => {
                    if let Ok(tool_call) = &tool_request.tool_call {
                        content.push(json!({
                            "type": "tool_use",
                            "id": tool_request.id,
                            "name": tool_call.name,
                            "input": tool_call.arguments
                        }));
                    }
                }
            }
        }

        // Skip messages with empty content
        if !content.is_empty() {
            snowflake_messages.push(json!({
                "role": role,
                "content": content
            }));
        }
    }

    // If no messages, add a default one
    if snowflake_messages.is_empty() {
        snowflake_messages.push(json!({
            "role": "user",
            "content": [{
                "type": "text",
                "text": "Ignore"
            }]
        }));
    }

    snowflake_messages
}

/// Convert internal Tool format to Snowflake's API tool specification
pub fn format_tools(tools: &[Tool]) -> Vec<Value> {
    let mut unique_tools = HashSet::new();
    let mut tool_specs = Vec::new();

    for tool in tools {
        if tool.name == "developer__shell" {
            if unique_tools.insert(tool.name.clone()) {
                tool_specs.push(json!({
                    "tool_spec": json!({
                        "type": "generic",
                        "name": tool.name,
                        "input_schema": tool.input_schema,
                    }),
                }));
            }
        }
    }

    tool_specs
}

/// Convert Snowflake's API response to internal Message format
pub fn response_to_message(response: Value) -> Result<Message> {
    let content_blocks = response
        .get("choices")
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow!("Invalid response format: missing content array"))?;

    let message = Message::assistant().with_text(content_blocks[0].to_string());

    Ok(message)
}

/// Extract usage information from Snowflake's API response
pub fn get_usage(data: &Value) -> Result<Usage> {
    // Extract usage data if available
    if let Some(usage) = data.get("usage") {
        // Sum up all input token types:
        // - input_tokens (fresh/uncached)
        // - cache_creation_input_tokens (being written to cache)
        // - cache_read_input_tokens (read from cache)
        let total_input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            + usage
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
            + usage
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

        let input_tokens = Some(total_input_tokens as i32);

        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as i32);

        let total_tokens = output_tokens.map(|o| total_input_tokens as i32 + o);

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

/// Create a complete request payload for Snowflake's API
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let snowflake_messages = format_messages(messages);
    let tool_specs = format_tools(tools);

    let system_message = json!({
        "role": "system",
        "content": system,
    });

    // Check if we have any messages to send
    if snowflake_messages.is_empty() {
        return Err(anyhow!("No valid messages to send to Snowflake API"));
    }

    // Dang, hardcoding
    let mut payload = json!({
        "messages": [
            system_message,
            {"content": "List files in the current directory.", "role": "user"},
        ],
        "model": model_config.model_name,
        "tools": [
            {
                "tool_spec": {
                    "type": "generic",
                    "name": "developer__shell",
                    "input_schema": {
                        "type": "object",
                        "properties": {"command": {"type": "string"}},
                        "required": ["command"],
                    },
                },
            }
        ],
    });

    // Add tools if present
    if !tool_specs.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tool_specs));
    }

    // Add temperature if specified and not using extended thinking model
    if let Some(temp) = model_config.temperature {
        // Claude 3.7 models with thinking enabled don't support temperature
        if !model_config.model_name.starts_with("claude-3-7-sonnet-") {
            payload
                .as_object_mut()
                .unwrap()
                .insert("temperature".to_string(), json!(temp));
        }
    }

    // Add thinking parameters for claude-3-7-sonnet model
    let is_thinking_enabled = std::env::var("CLAUDE_THINKING_ENABLED").is_ok();
    if model_config.model_name.starts_with("claude-3-7-sonnet-") && is_thinking_enabled {
        // Minimum budget_tokens is 1024
        let budget_tokens = std::env::var("CLAUDE_THINKING_BUDGET")
            .unwrap_or_else(|_| "16000".to_string())
            .parse()
            .unwrap_or(16000);

        payload
            .as_object_mut()
            .unwrap()
            .insert("max_tokens".to_string(), json!(budget_tokens));

        payload.as_object_mut().unwrap().insert(
            "thinking".to_string(),
            json!({
                "type": "enabled",
                "budget_tokens": budget_tokens
            }),
        );
    }

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            "model": "claude-3-5-sonnet-latest",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 12,
                "output_tokens": 15,
                "cache_creation_input_tokens": 12,
                "cache_read_input_tokens": 0
            }
        });

        let message = response_to_message(response.clone())?;
        let usage = get_usage(&response)?;

        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello! How can I assist you today?");
        } else {
            panic!("Expected Text content");
        }

        assert_eq!(usage.input_tokens, Some(24)); // 12 + 12 + 0
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

        let message = response_to_message(response.clone())?;
        let usage = get_usage(&response)?;

        if let MessageContent::ToolRequest(tool_request) = &message.content[0] {
            let tool_call = tool_request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "calculator");
            assert_eq!(tool_call.arguments, json!({"expression": "2 + 2"}));
        } else {
            panic!("Expected ToolRequest content");
        }

        assert_eq!(usage.input_tokens, Some(30)); // 15 + 15 + 0
        assert_eq!(usage.output_tokens, Some(20));
        assert_eq!(usage.total_tokens, Some(50)); // 30 + 20

        Ok(())
    }

    #[test]
    fn test_parse_thinking_response() -> Result<()> {
        let response = json!({
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "thinking",
                    "thinking": "This is a step-by-step thought process...",
                    "signature": "EuYBCkQYAiJAVbJNBoH7HQiDcMwwAMhWqNyoe4G2xHRprK8ICM8gZzu16i7Se4EiEbmlKqNH1GtwcX1BMK6iLu8bxWn5wPVIFBIMnptdlVal7ZX5iNPFGgwWjX+BntcEOHky4HciMFVef7FpQeqnuiL1Xt7J4OLHZSyu4tcr809AxAbclcJ5dm1xE5gZrUO+/v60cnJM2ipQp4B8/3eHI03KSV6bZR/vMrBSYCV+aa/f5KHX2cRtLGp/Ba+3Tk/efbsg01WSduwAIbR4coVrZLnGJXNyVTFW/Be2kLy/ECZnx8cqvU3oQOg="
                },
                {
                    "type": "redacted_thinking",
                    "data": "EmwKAhgBEgy3va3pzix/LafPsn4aDFIT2Xlxh0L5L8rLVyIwxtE3rAFBa8cr3qpP"
                },
                {
                    "type": "text",
                    "text": "I've analyzed the problem and here's the solution."
                }
            ],
            "model": "claude-3-7-sonnet-20250219",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 45,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
            }
        });

        let message = response_to_message(response.clone())?;
        let usage = get_usage(&response)?;

        assert_eq!(message.content.len(), 3);

        if let MessageContent::Thinking(thinking) = &message.content[0] {
            assert_eq!(
                thinking.thinking,
                "This is a step-by-step thought process..."
            );
            assert!(thinking
                .signature
                .starts_with("EuYBCkQYAiJAVbJNBoH7HQiDcMwwAMhWqNyoe4G2xHRprK8ICM8g"));
        } else {
            panic!("Expected Thinking content at index 0");
        }

        if let MessageContent::RedactedThinking(redacted) = &message.content[1] {
            assert_eq!(
                redacted.data,
                "EmwKAhgBEgy3va3pzix/LafPsn4aDFIT2Xlxh0L5L8rLVyIwxtE3rAFBa8cr3qpP"
            );
        } else {
            panic!("Expected RedactedThinking content at index 1");
        }

        if let MessageContent::Text(text) = &message.content[2] {
            assert_eq!(
                text.text,
                "I've analyzed the problem and here's the solution."
            );
        } else {
            panic!("Expected Text content at index 2");
        }

        assert_eq!(usage.input_tokens, Some(10));
        assert_eq!(usage.output_tokens, Some(45));
        assert_eq!(usage.total_tokens, Some(55));

        Ok(())
    }

    #[test]
    fn test_message_to_snowflake_spec() {
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
    fn test_tools_to_snowflake_spec() {
        let tools = vec![
            Tool::new(
                "calculator",
                "Calculate mathematical expressions",
                json!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "The mathematical expression to evaluate"
                        }
                    }
                }),
                None,
            ),
            Tool::new(
                "weather",
                "Get weather information",
                json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The location to get weather for"
                        }
                    }
                }),
                None,
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
    fn test_create_request_with_thinking() -> Result<()> {
        // Save the original env var value if it exists
        let original_value = std::env::var("CLAUDE_THINKING_ENABLED").ok();

        // Set the env var for this test
        std::env::set_var("CLAUDE_THINKING_ENABLED", "true");

        // Execute the test
        let result = (|| {
            let model_config = ModelConfig::new("claude-3-7-sonnet-20250219".to_string());
            let system = "You are a helpful assistant.";
            let messages = vec![Message::user().with_text("Hello")];
            let tools = vec![];

            let payload = create_request(&model_config, system, &messages, &tools)?;

            // Verify basic structure
            assert_eq!(payload["model"], "claude-3-7-sonnet-20250219");
            assert_eq!(payload["messages"][0]["role"], "user");
            assert_eq!(payload["messages"][0]["content"][0]["text"], "Hello");

            // Verify thinking parameters
            assert!(payload.get("thinking").is_some());
            assert_eq!(payload["thinking"]["type"], "enabled");
            assert!(payload["thinking"]["budget_tokens"].as_i64().unwrap() >= 1024);

            // Temperature should not be present for 3.7 models with thinking
            assert!(payload.get("temperature").is_none());

            Ok(())
        })();

        // Restore the original env var state
        match original_value {
            Some(val) => std::env::set_var("CLAUDE_THINKING_ENABLED", val),
            None => std::env::remove_var("CLAUDE_THINKING_ENABLED"),
        }

        // Return the test result
        result
    }
}
