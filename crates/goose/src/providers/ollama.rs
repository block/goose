use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::toolshim::{augment_message_with_tool_calls, OllamaInterpreter};
use super::utils::{get_model, handle_response_openai_compat};
use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::tool::Tool;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen2.5";
// Ollama can run many models, we only provide the default
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[OLLAMA_DEFAULT_MODEL];
pub const OLLAMA_DOC_URL: &str = "https://ollama.com/library";

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    model: ModelConfig,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OllamaProvider::metadata().default_model);
        OllamaProvider::from_env(model).expect("Failed to initialize Ollama provider")
    }
}

impl OllamaProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let host: String = config
            .get_param("OLLAMA_HOST")
            .unwrap_or_else(|_| OLLAMA_HOST.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            model,
        })
    }

    /// Get the base URL for Ollama API calls
    fn get_base_url(&self) -> Result<Url, ProviderError> {
        // OLLAMA_HOST is sometimes just the 'host' or 'host:port' without a scheme
        let base = if self.host.starts_with("http://") || self.host.starts_with("https://") {
            self.host.clone()
        } else {
            format!("http://{}", self.host)
        };

        let mut base_url = Url::parse(&base)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;

        // Set the default port if missing
        let explicit_default_port = self.host.ends_with(":80") || self.host.ends_with(":443");
        if base_url.port().is_none() && !explicit_default_port {
            base_url.set_port(Some(OLLAMA_DEFAULT_PORT)).map_err(|_| {
                ProviderError::RequestFailed("Failed to set default port".to_string())
            })?;
        }

        Ok(base_url)
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = self.get_base_url()?;

        let url = base_url.join("v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self.client.post(url).json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }

    fn process_response(
        &self,
        payload: Value,
        response: Value,
        message: Message,
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Get usage information
        let usage = match get_usage(&response) {
            Ok(usage) => {
                tracing::info!(
                    "Got usage data: input_tokens={:?}, output_tokens={:?}, total_tokens={:?}",
                    usage.input_tokens,
                    usage.output_tokens,
                    usage.total_tokens
                );
                usage
            }
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        tracing::info!("Using model: {}", model);
        super::utils::emit_debug_trace(self, &payload, &response, &usage);

        tracing::info!("Successfully completed request");
        Ok((message, ProviderUsage::new(model, usage)))
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "ollama",
            "Ollama",
            "Local open source models",
            OLLAMA_DEFAULT_MODEL,
            OLLAMA_KNOWN_MODELS.iter().map(|&s| s.to_string()).collect(),
            OLLAMA_DOC_URL,
            vec![ConfigKey::new(
                "OLLAMA_HOST",
                true,
                false,
                Some(OLLAMA_HOST),
            )],
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

        let base_url = self.get_base_url()?;
        let interpreter = OllamaInterpreter::new(base_url.to_string());

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

        tracing::info!(
            "Tool interpretation enabled: {}, tool count: {}",
            config.interpret_chat_tool_calls,
            tools.len()
        );

        // If tool interpretation is enabled, modify the system prompt to include tool info
        let modified_system = if config.interpret_chat_tool_calls {
            // Create a string with tool information
            let mut tool_info = String::new();
            for tool in tools {
                tool_info.push_str(&format!(
                    "Tool Name: {}\nSchema: {}\nDescription: {}\n\n",
                    tool.name,
                    serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default(),
                    tool.description
                ));
            }

            // Append tool information to the system prompt
            format!(
                "{}\n\n{}\n\nBreak down your task into smaller steps and do one step and tool call at a time. Do not try to use multiple tools at once. If you want to use a tool, tell the user what tool to use by specifying the tool in this JSON format\n{{\n  \"name\": \"tool_name\",\n  \"arguments\": {{\n    \"parameter1\": \"value1\",\n    \"parameter2\": \"value2\"\n            }}\n}}. After you get the tool result back, consider the result and then proceed to do the next step and tool call if required.",
                system,
                tool_info
            )
        } else {
            system.to_string()
        };

        // Create request with or without tools based on config
        let tools_for_request = if config.interpret_chat_tool_calls {
            &[]
        } else {
            tools
        };
        tracing::info!("Sending request with {} tools", tools_for_request.len());

        let payload = create_request(
            &self.model,
            &modified_system,
            messages,
            tools_for_request,
            &super::utils::ImageFormat::OpenAi,
        )?;

        let response = self.post(payload.clone()).await?;
        let message = response_to_message(response.clone())?;

        // Process the response and return the result
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
        let provider = OllamaProvider::from_env(model).unwrap();

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
}
