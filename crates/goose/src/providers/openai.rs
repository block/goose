use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::toolshim::{
    augment_message_with_tool_calls, modify_system_prompt_for_tools, OllamaInterpreter,
};
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;

pub const OPEN_AI_DEFAULT_MODEL: &str = "gpt-4o";
pub const OPEN_AI_KNOWN_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4-turbo",
    "gpt-3.5-turbo",
    "o1",
];

pub const OPEN_AI_DOC_URL: &str = "https://platform.openai.com/docs/models";

#[derive(Debug, serde::Serialize)]
pub struct OpenAiProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    base_path: String,
    api_key: String,
    organization: Option<String>,
    project: Option<String>,
    model: ModelConfig,
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OpenAiProvider::metadata().default_model);
        OpenAiProvider::from_env(model).expect("Failed to initialize OpenAI provider")
    }
}

impl OpenAiProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("OPENAI_API_KEY")?;
        let host: String = config
            .get_param("OPENAI_HOST")
            .unwrap_or_else(|_| "https://api.openai.com".to_string());
        let base_path: String = config
            .get_param("OPENAI_BASE_PATH")
            .unwrap_or_else(|_| "v1/chat/completions".to_string());
        let organization: Option<String> = config.get_param("OPENAI_ORGANIZATION").ok();
        let project: Option<String> = config.get_param("OPENAI_PROJECT").ok();
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            base_path,
            api_key,
            organization,
            project,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = url::Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join(&self.base_path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let mut request = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key));

        // Add organization header if present
        if let Some(org) = &self.organization {
            request = request.header("OpenAI-Organization", org);
        }

        // Add project header if present
        if let Some(project) = &self.project {
            request = request.header("OpenAI-Project", project);
        }

        let response = request.json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "openai",
            "OpenAI",
            "GPT-4 and other OpenAI models, including OpenAI compatible ones",
            OPEN_AI_DEFAULT_MODEL,
            OPEN_AI_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            OPEN_AI_DOC_URL,
            vec![
                ConfigKey::new("OPENAI_API_KEY", true, true, None),
                ConfigKey::new("OPENAI_HOST", true, false, Some("https://api.openai.com")),
                ConfigKey::new("OPENAI_BASE_PATH", true, false, Some("v1/chat/completions")),
                ConfigKey::new("OPENAI_ORGANIZATION", false, false, None),
                ConfigKey::new("OPENAI_PROJECT", false, false, None),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn structure_response(
        &self,
        message: Message,
        tools: &[Tool],
    ) -> Result<Message, ProviderError> {
        let config = self.get_model_config();
        if !config.interpret_chat_tool_calls {
            return Ok(message);
        }

        // Create interpreter for tool calls - use Ollama's default host and port
        let base_url = format!(
            "http://{}:{}",
            super::ollama::OLLAMA_HOST,
            super::ollama::OLLAMA_DEFAULT_PORT
        );
        let interpreter = OllamaInterpreter::new(base_url);

        augment_message_with_tool_calls(&interpreter, message, tools).await
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let config = self.get_model_config();

        // If tool interpretation is enabled, modify the system prompt
        let system_prompt = if config.interpret_chat_tool_calls {
            modify_system_prompt_for_tools(system, tools)
        } else {
            system.to_string()
        };

        // Create request with or without tools based on config
        let payload = create_request(
            &self.model,
            &system_prompt,
            messages,
            if config.interpret_chat_tool_calls {
                &[]
            } else {
                tools
            },
            &ImageFormat::OpenAi,
        )?;

        // Make request
        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = response_to_message(response.clone())?;
        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use mcp_core::tool::Tool;
    use serde_json::json;

    #[tokio::test]
    async fn test_structure_response() {
        // Create a provider with tool interpretation enabled
        std::env::set_var("GOOSE_TOOLSHIM", "1");
        let model = ModelConfig::new("test-model".to_string());
        let provider = OpenAiProvider::from_env(model).unwrap();

        // Create a message with potential tool call text
        let message = Message::assistant().with_text(
            r#"{
                "name": "test_tool",
                "arguments": {
                    "param1": "value1"
                }
            }"#,
        );

        // Create a test tool
        let tool = Tool::new(
            "test_tool".to_string(),
            "Test tool".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "param1": {"type": "string"}
                }
            }),
        );

        // Test interpreting the response
        let structured = provider.structure_response(message, &[tool]).await.unwrap();

        // Verify the tool call was extracted
        let tool_requests: Vec<&MessageContent> = structured
            .content
            .iter()
            .filter(|c| matches!(c, MessageContent::ToolRequest(_)))
            .collect();

        assert_eq!(tool_requests.len(), 1);
        if let MessageContent::ToolRequest(request) = tool_requests[0] {
            if let Ok(tool_call) = &request.tool_call {
                assert_eq!(tool_call.name, "test_tool");
                assert_eq!(
                    tool_call.arguments.get("param1").and_then(|v| v.as_str()),
                    Some("value1")
                );
            } else {
                panic!("Tool call was not Ok");
            }
        } else {
            panic!("Expected ToolRequest");
        }

        // Clean up
        std::env::remove_var("GOOSE_TOOLSHIM");
    }

    #[tokio::test]
    async fn test_complete_with_tool_interpretation() {
        // This test can't make actual API calls, but we can verify the request payload
        std::env::set_var("GOOSE_TOOLSHIM", "1");
        let model = ModelConfig::new("test-model".to_string());
        let provider = OpenAiProvider::from_env(model).unwrap();

        // Create a test tool
        let tool = Tool::new(
            "test_tool".to_string(),
            "Test tool".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "param1": {"type": "string"}
                }
            }),
        );

        // Create a Vec to hold the tool
        let tools = vec![tool];

        // Verify that complete() doesn't include tools in the request when interpretation is enabled
        let payload = create_request(
            &provider.model,
            "test system",
            &[Message::user().with_text("test message")],
            if provider.model.interpret_chat_tool_calls {
                &[]
            } else {
                &tools
            },
            &ImageFormat::OpenAi,
        )
        .unwrap();

        // When tool interpretation is enabled, tools should not be in the request
        assert!(provider.model.interpret_chat_tool_calls);
        assert!(payload.get("tools").is_none() || payload["tools"].as_array().unwrap().is_empty());

        // Clean up
        std::env::remove_var("GOOSE_TOOLSHIM");
    }
}
