use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::time::Duration;

use super::base::{Provider, Usage};
use super::configs::AnthropicProviderConfig;
use crate::models::content::{Content, TextContent};
use crate::models::message::{Message, MessageContent};
use crate::models::role::Role;
use crate::models::tool::{Tool, ToolCall};

pub struct AnthropicProvider {
    client: Client,
    config: AnthropicProviderConfig,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicProviderConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(600)) // 10 minutes timeout
            .build()?;

        Ok(Self { client, config })
    }

    fn get_usage(data: &Value) -> Result<Usage> {
        // Extract usage data if available
        if let Some(usage) = data.get("usage") {
            let input_tokens = usage
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as i32);
            let output_tokens = usage
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as i32);
            let total_tokens = match (input_tokens, output_tokens) {
                (Some(i), Some(o)) => Some(i + o),
                _ => None,
            };

            Ok(Usage::new(input_tokens, output_tokens, total_tokens))
        } else {
            // If no usage data, return None for all values
            Ok(Usage::new(None, None, None))
        }
    }

    fn tools_to_anthropic_spec(tools: &[Tool]) -> Vec<Value> {
        let mut unique_tools = HashSet::new();
        let mut tool_specs = Vec::new();

        for tool in tools {
            if unique_tools.insert(tool.name.clone()) {
                tool_specs.push(json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.input_schema
                }));
            }
        }

        tool_specs
    }

    fn messages_to_anthropic_spec(messages: &[Message]) -> Vec<Value> {
        let mut anthropic_messages = Vec::new();

        // Convert messages to Anthropic format
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
                            "type": "text",
                            "text": text.text
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
                    MessageContent::Image(_) => continue, // Anthropic doesn't support image content yet
                }
            }

            // Skip messages with empty content
            if !content.is_empty() {
                anthropic_messages.push(json!({
                    "role": role,
                    "content": content
                }));
            }
        }

        // If no messages, add a default one
        if anthropic_messages.is_empty() {
            anthropic_messages.push(json!({
                "role": "user",
                "content": [{
                    "type": "text",
                    "text": "Ignore"
                }]
            }));
        }

        anthropic_messages
    }

    fn parse_anthropic_response(response: Value) -> Result<Message> {
        let content_blocks = response
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or_else(|| anyhow!("Invalid response format: missing content array"))?;

        let mut message = Message::assistant();

        for block in content_blocks {
            match block.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        message = message.with_content(MessageContent::Text(TextContent {
                            text: text.to_string(),
                            audience: None,
                            priority: None,
                        }));
                    }
                }
                Some("tool_use") => {
                    let id = block
                        .get("id")
                        .and_then(|i| i.as_str())
                        .ok_or_else(|| anyhow!("Missing tool_use id"))?;
                    let name = block
                        .get("name")
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| anyhow!("Missing tool_use name"))?;
                    let input = block
                        .get("input")
                        .ok_or_else(|| anyhow!("Missing tool_use input"))?;

                    let tool_call = ToolCall::new(name, input.clone());
                    message = message.with_tool_request(id, Ok(tool_call));
                }
                _ => continue,
            }
        }

        Ok(message)
    }

    async fn post(&self, payload: Value) -> Result<Value> {
        let url = format!("{}/v1/messages", self.config.host.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => Ok(response.json().await?),
            status if status == StatusCode::TOO_MANY_REQUESTS || status.as_u16() >= 500 => {
                Err(anyhow!("Server error: {}", status))
            }
            status => {
                let error_text = response.text().await?;
                Err(anyhow!("Request failed: {} - {}", status, error_text))
            }
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, Usage)> {
        let anthropic_messages = Self::messages_to_anthropic_spec(messages);
        let tool_specs = Self::tools_to_anthropic_spec(tools);

        // Check if we have any messages to send
        if anthropic_messages.is_empty() {
            return Err(anyhow!("No valid messages to send to Anthropic API"));
        }

        let mut payload = json!({
            "model": self.config.model,
            "messages": anthropic_messages,
            "max_tokens": self.config.max_tokens.unwrap_or(4096)
        });

        // Add system message if present
        if !system.is_empty() {
            payload
                .as_object_mut()
                .unwrap()
                .insert("system".to_string(), json!(system));
        }

        // Add tools if present
        if !tool_specs.is_empty() {
            payload
                .as_object_mut()
                .unwrap()
                .insert("tools".to_string(), json!(tool_specs));
        }

        // Add temperature if specified
        if let Some(temp) = self.config.temperature {
            payload
                .as_object_mut()
                .unwrap()
                .insert("temperature".to_string(), json!(temp));
        }

        // Make request
        let response = self.post(payload).await?;

        // Parse response
        let message = Self::parse_anthropic_response(response.clone())?;
        let usage = Self::get_usage(&response)?;

        Ok((message, usage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_mock_server(response_body: Value) -> (MockServer, AnthropicProvider) {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test_api_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let config = AnthropicProviderConfig {
            host: mock_server.uri(),
            api_key: "test_api_key".to_string(),
            model: "claude-3-sonnet-20241022".to_string(),
            temperature: Some(0.7),
            max_tokens: None,
        };

        let provider = AnthropicProvider::new(config).unwrap();
        (mock_server, provider)
    }

    #[tokio::test]
    async fn test_complete_basic() -> Result<()> {
        let response_body = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Hello! How can I assist you today?"
            }],
            "model": "claude-3-sonnet-20240229",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 12,
                "output_tokens": 15
            }
        });

        let (_, provider) = setup_mock_server(response_body).await;

        let messages = vec![Message::user().with_text("Hello?")];

        let (message, usage) = provider
            .complete("You are a helpful assistant.", &messages, &[])
            .await?;

        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello! How can I assist you today?");
        } else {
            panic!("Expected Text content");
        }

        assert_eq!(usage.input_tokens, Some(12));
        assert_eq!(usage.output_tokens, Some(15));
        assert_eq!(usage.total_tokens, Some(27));

        Ok(())
    }

    #[tokio::test]
    async fn test_complete_with_tools() -> Result<()> {
        let response_body = json!({
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
                "output_tokens": 20
            }
        });

        let (_, provider) = setup_mock_server(response_body).await;

        let messages = vec![Message::user().with_text("What is 2 + 2?")];
        let tool = Tool::new(
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
        );

        let (message, usage) = provider
            .complete("You are a helpful assistant.", &messages, &[tool])
            .await?;

        if let MessageContent::ToolRequest(tool_request) = &message.content[0] {
            let tool_call = tool_request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "calculator");
            assert_eq!(tool_call.arguments, json!({"expression": "2 + 2"}));
        } else {
            panic!("Expected ToolRequest content");
        }

        assert_eq!(usage.input_tokens, Some(15));
        assert_eq!(usage.output_tokens, Some(20));
        assert_eq!(usage.total_tokens, Some(35));

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_messages() -> Result<()> {
        let response_body = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Hello!"
            }],
            "model": "claude-3-sonnet-20240229",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 12,
                "output_tokens": 15
            }
        });

        let (_, provider) = setup_mock_server(response_body).await;

        // Create a message with empty content
        let messages = vec![
            Message::user().with_text(""),
            Message::user().with_text("Hello"),
        ];

        let (message, _) = provider
            .complete("You are a helpful assistant.", &messages, &[])
            .await?;

        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello!");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }
}
