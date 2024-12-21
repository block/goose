use crate::message::Message;
use crate::providers::base::{Provider, ProviderUsage, Usage};
use crate::providers::configs::{GroqProviderConfig, ModelConfig, ProviderModelConfig};
use crate::providers::openai_utils::{
    create_openai_request_payload_with_concat_response_content, get_openai_usage,
    openai_response_to_message,
};
use crate::providers::utils::{get_model, handle_response};
use async_trait::async_trait;
use mcp_core::Tool;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub const GROQ_API_HOST: &str = "https://api.groq.com";
pub const GROQ_DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";

pub struct GroqProvider {
    client: Client,
    config: GroqProviderConfig,
}

impl GroqProvider {
    pub fn new(config: GroqProviderConfig) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(600)) // 10 minutes timeout
            .build()?;

        Ok(Self { client, config })
    }

    async fn post(&self, payload: Value) -> anyhow::Result<Value> {
        let url = format!(
            "{}/openai/v1/chat/completions",
            self.config.host.trim_end_matches('/')
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&payload)
            .send()
            .await?;
        handle_response(payload, response).await?
    }
}

#[async_trait]
impl Provider for GroqProvider {
    fn get_model_config(&self) -> &ModelConfig {
        self.config.model_config()
    }

    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> anyhow::Result<(Message, ProviderUsage)> {
        let payload = create_openai_request_payload_with_concat_response_content(
            &self.config.model,
            system,
            messages,
            tools,
        )?;

        let response = self.post(payload).await?;

        let message = openai_response_to_message(response.clone())?;
        let usage = self.get_usage(&response)?;
        let model = get_model(&response);

        Ok((message, ProviderUsage::new(model, usage, None)))
    }

    fn get_usage(&self, data: &Value) -> anyhow::Result<Usage> {
        get_openai_usage(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use crate::providers::mock_server::{
        create_mock_open_ai_response, create_mock_open_ai_response_with_tools, create_test_tool,
        get_expected_function_call_arguments, setup_mock_server, TEST_INPUT_TOKENS,
        TEST_OUTPUT_TOKENS, TEST_TOOL_FUNCTION_NAME, TEST_TOTAL_TOKENS,
    };
    use wiremock::MockServer;

    async fn _setup_mock_server(response_body: Value) -> (MockServer, GroqProvider) {
        let mock_server = setup_mock_server("/openai/v1/chat/completions", response_body).await;
        let config = GroqProviderConfig {
            host: mock_server.uri(),
            api_key: "test_api_key".to_string(),
            model: ModelConfig::new(GROQ_DEFAULT_MODEL.to_string()),
        };

        let provider = GroqProvider::new(config).unwrap();
        (mock_server, provider)
    }

    #[tokio::test]
    async fn test_complete_basic() -> anyhow::Result<()> {
        let model_name = "gpt-4o";
        // Mock response for normal completion
        let response_body =
            create_mock_open_ai_response(model_name, "Hello! How can I assist you today?");

        let (_, provider) = _setup_mock_server(response_body).await;

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
        assert_eq!(usage.cost, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_complete_tool_request() -> anyhow::Result<()> {
        // Mock response for tool calling
        let response_body = create_mock_open_ai_response_with_tools("gpt-4o");

        let (_, provider) = _setup_mock_server(response_body).await;

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
