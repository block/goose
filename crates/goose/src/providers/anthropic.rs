use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::time::Duration;

use super::base::{Provider, Usage};
use super::configs::AnthropicProviderConfig;
use crate::models::message::Message;
use crate::models::tool::Tool;
use crate::models::role::Role;

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

    fn get_usage(_data: &Value) -> Result<Usage> {
        // Anthropic currently doesn't provide detailed token usage
        // We'll return None for all values until they add this feature
        Ok(Usage::new(None, None, None))
    }

    fn messages_to_anthropic_spec(messages: &[Message], system: &str) -> Vec<Value> {
        let mut anthropic_messages = Vec::new();
        
        // Add system message if present
        if !system.is_empty() {
            anthropic_messages.push(json!({
                "role": "assistant",
                "content": system
            }));
        }

        // Convert messages to Anthropic format
        for message in messages {
            let role = match message.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };

            let mut content = String::new();
            for msg_content in &message.content {
                match msg_content {
                    crate::models::message::MessageContent::Text(text) => {
                        content.push_str(&text.text);
                    }
                    _ => {} // Skip other content types for now
                }
            }

            // Skip messages with empty content
            if content.is_empty() {
                continue;
            }

            anthropic_messages.push(json!({
                "role": role,
                "content": content
            }));
        }

        anthropic_messages
    }

    async fn post(&self, payload: Value) -> Result<Value> {
        let url = format!(
            "{}/v1/messages",
            self.config.host.trim_end_matches('/')
        );

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
                Err(anyhow!(
                    "Request failed: {} - {}",
                    status,
                    error_text
                ))
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
        _tools: &[Tool],
    ) -> Result<(Message, Usage)> {
        let anthropic_messages = Self::messages_to_anthropic_spec(messages, system);

        // Check if we have any messages to send
        if anthropic_messages.is_empty() {
            return Err(anyhow!("No valid messages to send to Anthropic API"));
        }

        let mut payload = json!({
            "model": self.config.model,
            "messages": anthropic_messages,
            "max_tokens": self.config.max_tokens.unwrap_or(1000)
        });

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
        let content = response
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("text"))
            .and_then(|text| text.as_str())
            .ok_or_else(|| anyhow!("Invalid response format from Anthropic API"))?;

        let message = Message::assistant().with_text(content);
        let usage = Self::get_usage(&response)?;

        Ok((message, usage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::MessageContent;
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

        // Anthropic provider currently returns None for all usage values
        assert_eq!(usage.input_tokens, None);
        assert_eq!(usage.output_tokens, None);
        assert_eq!(usage.total_tokens, None);

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