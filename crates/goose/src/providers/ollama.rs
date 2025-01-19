use super::base::{Moderation, ModerationResult, Provider, ProviderUsage, Usage};
use super::configs::ModelConfig;
use super::utils::{get_model, handle_response};
use crate::message::Message;
use crate::providers::openai_utils::{
    create_openai_request_payload, get_openai_usage, openai_response_to_message,
};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::tool::Tool;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub const OLLAMA_HOST: &str = "http://localhost:11434";
pub const OLLAMA_MODEL: &str = "qwen2.5";

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    model: ModelConfig,
}

impl OllamaProvider {
    pub fn from_env() -> Result<Self> {
        // Although we don't need host to be stored secretly, we use the keyring to make
        // it easier to coordinate with configuration. We could consider a non secret storage tool
        // elsewhere in the future
        let host = crate::key_manager::get_keyring_secret("OLLAMA_HOST", Default::default())
            .unwrap_or_else(|_| OLLAMA_HOST.to_string());
        let model_name = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| OLLAMA_MODEL.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            model: ModelConfig::new(model_name),
        })
    }

    async fn post(&self, payload: Value) -> Result<Value> {
        let url = format!("{}/v1/chat/completions", self.host.trim_end_matches('/'));

        let response = self.client.post(&url).json(&payload).send().await?;

        handle_response(payload, response).await
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn get_model_config(&self) -> &ModelConfig {
        &self.model
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
    ) -> Result<(Message, ProviderUsage)> {
        let payload = create_openai_request_payload(&self.model, system, messages, tools)?;

        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = openai_response_to_message(response.clone())?;
        let usage = self.get_usage(&response)?;
        let model = get_model(&response);
        let cost = None;
        super::utils::emit_debug_trace(self, &payload, &response, &usage, cost);
        Ok((message, ProviderUsage::new(model, usage, cost)))
    }

    fn get_usage(&self, data: &Value) -> Result<Usage> {
        get_openai_usage(data)
    }
}

#[async_trait]
impl Moderation for OllamaProvider {
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

    async fn _setup_mock_server(response_body: Value) -> (MockServer, OllamaProvider) {
        let mock_server = setup_mock_server("/v1/chat/completions", response_body).await;

        let provider = OllamaProvider {
            client: Client::builder().build().unwrap(),
            host: mock_server.uri(),
            model: ModelConfig::new(OLLAMA_MODEL.to_string()),
        };

        (mock_server, provider)
    }

    #[tokio::test]
    async fn test_complete_basic() -> Result<()> {
        let model_name = "gpt-4o";
        let expected_response = "Hello! How can I assist you today?";
        // Mock response for normal completion
        let response_body = create_mock_open_ai_response(model_name, expected_response);

        let (mock_server, provider) = _setup_mock_server(response_body).await;

        // Prepare input messages
        let messages = vec![Message::user().with_text("Hello?")];

        // Call the complete method
        let (message, usage) = provider
            .complete("You are a helpful assistant.", &messages, &[])
            .await?;

        // Assert the response
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

        // Verify usage metrics
        assert_eq!(
            usage.usage.input_tokens,
            Some(TEST_INPUT_TOKENS),
            "Input tokens mismatch"
        );
        assert_eq!(
            usage.usage.output_tokens,
            Some(TEST_OUTPUT_TOKENS),
            "Output tokens mismatch"
        );
        assert_eq!(
            usage.usage.total_tokens,
            Some(TEST_TOTAL_TOKENS),
            "Total tokens mismatch"
        );
        assert_eq!(usage.model, model_name, "Model name mismatch");
        assert_eq!(usage.cost, None, "Cost should be None");

        // Ensure mock server handled the request
        mock_server.verify().await;

        Ok(())
    }

    #[tokio::test]
    async fn test_complete_tool_request() -> Result<()> {
        // Mock response for tool calling
        let response_body = create_mock_open_ai_response_with_tools("gpt-4o");

        let (mock_server, provider) = _setup_mock_server(response_body).await;

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

        mock_server.verify().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_server_error() -> Result<()> {
        let mock_server = setup_mock_server_with_response_code("/v1/chat/completions", 500).await;

        let provider = OllamaProvider {
            client: Client::builder().build().unwrap(),
            host: mock_server.uri(),
            model: ModelConfig::new(OLLAMA_MODEL.to_string()),
        };
        let messages = vec![Message::user().with_text("Hello?")];
        let result = provider
            .complete("You are a helpful assistant.", &messages, &[])
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().starts_with("Server error"));

        Ok(())
    }
}
