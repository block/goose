use anyhow::{Error, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::toolshim::{
    augment_message_with_tool_calls, modify_system_prompt_for_tools, OllamaInterpreter,
};
use super::utils::{
    emit_debug_trace, get_model, handle_response_google_compat, handle_response_openai_compat,
    is_google_model,
};
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use mcp_core::tool::Tool;
use url::Url;

pub const OPENROUTER_DEFAULT_MODEL: &str = "anthropic/claude-3.5-sonnet";
pub const OPENROUTER_MODEL_PREFIX_ANTHROPIC: &str = "anthropic";

// OpenRouter can run many models, we suggest the default
pub const OPENROUTER_KNOWN_MODELS: &[&str] = &[OPENROUTER_DEFAULT_MODEL];
pub const OPENROUTER_DOC_URL: &str = "https://openrouter.ai/models";

#[derive(serde::Serialize)]
pub struct OpenRouterProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl Default for OpenRouterProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OpenRouterProvider::metadata().default_model);
        OpenRouterProvider::from_env(model).expect("Failed to initialize OpenRouter provider")
    }
}

impl OpenRouterProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("OPENROUTER_API_KEY")?;
        let host: String = config
            .get_param("OPENROUTER_HOST")
            .unwrap_or_else(|_| "https://openrouter.ai".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            api_key,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("api/v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://block.github.io/goose")
            .header("X-Title", "Goose")
            .json(&payload)
            .send()
            .await?;

        if is_google_model(&payload) {
            handle_response_google_compat(response).await
        } else {
            handle_response_openai_compat(response).await
        }
    }

    fn process_response(
        &self,
        payload: Value,
        response: Value,
        message: Message,
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Get usage information
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

/// Update the request when using anthropic model.
/// For anthropic model, we can enable prompt caching to save cost. Since openrouter is the OpenAI compatible
/// endpoint, we need to modify the open ai request to have anthropic cache control field.
fn update_request_for_anthropic(original_payload: &Value) -> Value {
    let mut payload = original_payload.clone();

    if let Some(messages_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("messages"))
        .and_then(|messages| messages.as_array_mut())
    {
        // Add "cache_control" to the last and second-to-last "user" messages.
        // During each turn, we mark the final message with cache_control so the conversation can be
        // incrementally cached. The second-to-last user message is also marked for caching with the
        // cache_control parameter, so that this checkpoint can read from the previous cache.
        let mut user_count = 0;
        for message in messages_spec.iter_mut().rev() {
            if message.get("role") == Some(&json!("user")) {
                if let Some(content) = message.get_mut("content") {
                    if let Some(content_str) = content.as_str() {
                        *content = json!([{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]);
                    }
                }
                user_count += 1;
                if user_count >= 2 {
                    break;
                }
            }
        }

        // Update the system message to have cache_control field.
        if let Some(system_message) = messages_spec
            .iter_mut()
            .find(|msg| msg.get("role") == Some(&json!("system")))
        {
            if let Some(content) = system_message.get_mut("content") {
                if let Some(content_str) = content.as_str() {
                    *system_message = json!({
                        "role": "system",
                        "content": [{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]
                    });
                }
            }
        }
    }

    if let Some(tools_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("tools"))
        .and_then(|tools| tools.as_array_mut())
    {
        // Add "cache_control" to the last tool spec, if any. This means that all tool definitions,
        // will be cached as a single prefix.
        if let Some(last_tool) = tools_spec.last_mut() {
            if let Some(function) = last_tool.get_mut("function") {
                function
                    .as_object_mut()
                    .unwrap()
                    .insert("cache_control".to_string(), json!({ "type": "ephemeral" }));
            }
        }
    }
    payload
}

fn create_request_based_on_model(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> anyhow::Result<Value, Error> {
    let mut payload = create_request(
        model_config,
        system,
        messages,
        tools,
        &super::utils::ImageFormat::OpenAi,
    )?;

    if model_config
        .model_name
        .starts_with(OPENROUTER_MODEL_PREFIX_ANTHROPIC)
    {
        payload = update_request_for_anthropic(&payload);
    }

    Ok(payload)
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "openrouter",
            "OpenRouter",
            "Router for many model providers",
            OPENROUTER_DEFAULT_MODEL,
            OPENROUTER_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            OPENROUTER_DOC_URL,
            vec![
                ConfigKey::new("OPENROUTER_API_KEY", true, true, None),
                ConfigKey::new(
                    "OPENROUTER_HOST",
                    false,
                    false,
                    Some("https://openrouter.ai"),
                ),
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
        let payload = create_request_based_on_model(
            &self.model,
            &system_prompt,
            messages,
            if config.interpret_chat_tool_calls {
                &[]
            } else {
                tools
            },
        )?;

        let response = self.post(payload.clone()).await?;
        let message = response_to_message(response.clone())?;

        self.process_response(payload, response, message)
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
        let provider = OpenRouterProvider::from_env(model).unwrap();

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
        let provider = OpenRouterProvider::from_env(model).unwrap();

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

        // Verify that complete() doesn't include tools in the request when interpretation is enabled
        let payload = create_request_based_on_model(
            &provider.model,
            "test system",
            &[Message::user().with_text("test message")],
            if provider.model.interpret_chat_tool_calls {
                &[]
            } else {
                &[tool]
            },
        )
        .unwrap();

        // When tool interpretation is enabled, tools should not be in the request
        assert!(provider.model.interpret_chat_tool_calls);
        assert!(payload.get("tools").is_none() || payload["tools"].as_array().unwrap().is_empty());

        // Clean up
        std::env::remove_var("GOOSE_TOOLSHIM");
    }
}
