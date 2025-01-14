use crate::message::Message;
use crate::providers::base::{Moderation, ModerationResult, Provider, ProviderUsage, Usage};
use crate::providers::configs::{GroqProviderConfig, ModelConfig, ProviderModelConfig};
use crate::providers::openai_utils::{
    create_openai_request_payload_with_concat_response_content, get_openai_usage,
    openai_response_to_message,
};
use crate::providers::utils::{get_model, handle_response};
use anyhow::Result;
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

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(
            model_config,
            input,
            output,
            input_tokens,
            output_tokens,
            total_tokens,
            cost
        )
    )]
    async fn complete_internal(
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

        let response = self.post(payload.clone()).await?;

        let message = openai_response_to_message(response.clone())?;
        let usage = self.get_usage(&response)?;
        let model = get_model(&response);
        super::utils::emit_debug_trace(&self.config, &payload, &response, &usage, None);
        Ok((message, ProviderUsage::new(model, usage, None)))
    }

    fn get_usage(&self, data: &Value) -> anyhow::Result<Usage> {
        get_openai_usage(data)
    }
}

#[async_trait]
impl Moderation for GroqProvider {
    async fn moderate_content(&self, _content: &str) -> Result<ModerationResult> {
        Ok(ModerationResult::new(false, None, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use crate::providers::mock_server::{
        create_mock_open_ai_response, create_mock_open_ai_response_with_tools, create_test_tool,
        get_expected_function_call_arguments, setup_mock_server,
        setup_mock_server_with_response_code, TEST_INPUT_TOKENS, TEST_OUTPUT_TOKENS,
        TEST_TOOL_FUNCTION_NAME, TEST_TOTAL_TOKENS,
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
        let expected_response = "Hello! How can I assist you today?";
        let response_body = create_mock_open_ai_response(model_name, expected_response);

        let (mock_server, provider) = _setup_mock_server(response_body).await;

        let messages = vec![Message::user().with_text("Hello?")];
        let (message, usage) = provider
            .complete("You are a helpful assistant.", &messages, &[])
            .await?;

        assert!(
            !message.content.is_empty(),
            "Message content should not be empty"
        );
        match &message.content[0] {
            MessageContent::Text(text) => {
                assert_eq!(
                    text.text, expected_response,
                    "Response text does not match expected"
                );
            }
            other => panic!("Expected Text content, got {:?}", other),
        }

        assert_eq!(usage.usage.input_tokens, Some(TEST_INPUT_TOKENS));
        assert_eq!(usage.usage.output_tokens, Some(TEST_OUTPUT_TOKENS));
        assert_eq!(usage.usage.total_tokens, Some(TEST_TOTAL_TOKENS));
        assert_eq!(usage.model, model_name);
        assert_eq!(usage.cost, None);

        mock_server.verify().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_complete_tool_request() -> anyhow::Result<()> {
        let response_body = create_mock_open_ai_response_with_tools("gpt-4o");
        let (mock_server, provider) = _setup_mock_server(response_body).await;

        let messages = vec![Message::user().with_text("What's the weather in San Francisco?")];
        let (message, usage) = provider
            .complete(
                "You are a helpful assistant.",
                &messages,
                &[create_test_tool()],
            )
            .await?;

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

        mock_server.verify().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_server_error() -> anyhow::Result<()> {
        let mock_server =
            setup_mock_server_with_response_code("/openai/v1/chat/completions", 500).await;

        let config = GroqProviderConfig {
            host: mock_server.uri(),
            api_key: "test_api_key".to_string(),
            model: ModelConfig::new(GROQ_DEFAULT_MODEL.to_string()),
        };

        let provider = GroqProvider::new(config)?;
        let messages = vec![Message::user().with_text("Hello?")];
        let result = provider
            .complete("You are a helpful assistant.", &messages, &[])
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Server error: 500"));

        Ok(())
    }
}
