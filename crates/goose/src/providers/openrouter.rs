use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::base::ProviderUsage;
use super::base::{Provider, Usage};
use super::configs::OpenAiProviderConfig;
use super::configs::{ModelConfig, ProviderModelConfig};
use super::model_pricing::cost;
use super::model_pricing::model_pricing_for;
use super::utils::{get_model, handle_response};
use crate::message::Message;
use crate::providers::openai_utils::{
    check_openai_context_length_error, create_openai_request_payload_with_concat_response_content,
    get_openai_usage, openai_response_to_message,
};
use mcp_core::tool::Tool;

pub const OPENROUTER_DEFAULT_MODEL: &str = "anthropic/claude-3.5-sonnet";

pub struct OpenRouterProvider {
    client: Client,
    config: OpenAiProviderConfig,
}

impl OpenRouterProvider {
    pub fn new(config: OpenAiProviderConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(600)) // 10 minutes timeout
            .build()?;

        Ok(Self { client, config })
    }

    async fn post(&self, payload: Value) -> Result<Value> {
        let url = format!(
            "{}/api/v1/chat/completions",
            self.config.host.trim_end_matches('/')
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("HTTP-Referer", "https://github.com/block/goose")
            .header("X-Title", "Goose")
            .json(&payload)
            .send()
            .await?;

        handle_response(payload, response).await?
    }
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn get_model_config(&self) -> &ModelConfig {
        self.config.model_config()
    }

    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage)> {
        // Create the base payload
        let payload = create_openai_request_payload_with_concat_response_content(
            &self.config.model,
            system,
            messages,
            tools,
        )?;

        // Make request
        let response = self.post(payload).await?;

        // Raise specific error if context length is exceeded
        if let Some(error) = response.get("error") {
            if let Some(err) = check_openai_context_length_error(error) {
                return Err(err.into());
            }
            return Err(anyhow!("OpenRouter API error: {}", error));
        }

        // Parse response
        let message = openai_response_to_message(response.clone())?;
        let usage = self.get_usage(&response)?;
        let model = get_model(&response);
        let cost = cost(&usage, &model_pricing_for(&model));

        Ok((message, ProviderUsage::new(model, usage, cost)))
    }

    fn get_usage(&self, data: &Value) -> Result<Usage> {
        get_openai_usage(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use crate::providers::configs::ModelConfig;
    use crate::providers::mock_server::{
        create_mock_open_ai_response, create_mock_open_ai_response_with_tools, create_test_tool,
        get_expected_function_call_arguments, setup_mock_server, TEST_INPUT_TOKENS,
        TEST_OUTPUT_TOKENS, TEST_TOOL_FUNCTION_NAME, TEST_TOTAL_TOKENS,
    };
    use rust_decimal_macros::dec;
    use wiremock::MockServer;

    async fn _setup_mock_response(response_body: Value) -> (MockServer, OpenRouterProvider) {
        let mock_server = setup_mock_server("/api/v1/chat/completions", response_body).await;

        // Create the OpenRouterProvider with the mock server's URL as the host
        let config = OpenAiProviderConfig {
            host: mock_server.uri(),
            api_key: "test_api_key".to_string(),
            model: ModelConfig::new("gpt-3.5-turbo".to_string()).with_temperature(Some(0.7)),
        };

        let provider = OpenRouterProvider::new(config).unwrap();
        (mock_server, provider)
    }

    #[tokio::test]
    async fn test_complete_basic() -> Result<()> {
        let model_name = "gpt-4";
        // Mock response for normal completion
        let response_body =
            create_mock_open_ai_response(model_name, "Hello! How can I assist you today?");

        let (_, provider) = _setup_mock_response(response_body).await;

        // Prepare input messages
        let messages = vec![Message::user().with_text("Hello?")];

        // Call the complete method
        let (message, usage) = provider
            .complete("You are a helpful assistant.", &messages, &[])
            .await?;

        // Assert the response
        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello! How can I assist you today?");
        } else {
            panic!("Expected Text content");
        }
        assert_eq!(usage.usage.input_tokens, Some(TEST_INPUT_TOKENS));
        assert_eq!(usage.usage.output_tokens, Some(TEST_OUTPUT_TOKENS));
        assert_eq!(usage.usage.total_tokens, Some(TEST_TOTAL_TOKENS));
        assert_eq!(usage.model, model_name);
        assert_eq!(usage.cost, Some(dec!(0.00018)));

        Ok(())
    }

    #[tokio::test]
    async fn test_complete_tool_request() -> Result<()> {
        // Mock response for tool calling
        let response_body = create_mock_open_ai_response_with_tools("gpt-4");

        let (_, provider) = _setup_mock_response(response_body).await;

        // Input messages
        let messages = vec![Message::user().with_text("What's the weather in San Francisco?")];

        // Call the complete method
        let (message, usage) = provider
            .complete(
                "You are a helpful assistant.",
                &messages,
                &[create_test_tool()],
            )
            .await?;

        // Assert the response
        if let MessageContent::ToolRequest(tool_request) = &message.content[0] {
            let tool_call = tool_request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, TEST_TOOL_FUNCTION_NAME);
            assert_eq!(tool_call.arguments, get_expected_function_call_arguments());
        } else {
            panic!("Expected ToolCall content");
        }

        assert_eq!(usage.usage.input_tokens, Some(TEST_INPUT_TOKENS));
        assert_eq!(usage.usage.output_tokens, Some(TEST_OUTPUT_TOKENS));
        assert_eq!(usage.usage.total_tokens, Some(TEST_TOTAL_TOKENS));

        Ok(())
    }
}
