use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::errors::ProviderError;
use crate::{
    message::{Message, MessageContent},
    types::core::{Tool, ToolError},
};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, uniffi::Record)]
pub struct Usage {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

impl Usage {
    pub fn new(
        input_tokens: Option<i32>,
        output_tokens: Option<i32>,
        total_tokens: Option<i32>,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ProviderCompleteResponse {
    pub message: Message,
    pub model: String,
    pub usage: Usage,
}

impl ProviderCompleteResponse {
    pub fn new(message: Message, model: String, usage: Usage) -> Self {
        Self {
            message,
            model,
            usage,
        }
    }
}

/// Response from a structured‐extraction call
#[derive(Debug, Clone, uniffi::Record)]
pub struct ProviderExtractResponse {
    /// The extracted JSON object
    pub data: serde_json::Value,
    /// Which model produced it
    pub model: String,
    /// Token usage stats
    pub usage: Usage,
}

impl ProviderExtractResponse {
    pub fn new(data: serde_json::Value, model: String, usage: Usage) -> Self {
        Self { data, model, usage }
    }
}

/// Base trait for AI providers (OpenAI, Anthropic, etc)
#[async_trait]
pub trait Provider: Send + Sync {
    /// Generate the next message using the configured model and other parameters
    ///
    /// # Arguments
    /// * `system` - The system prompt that guides the model's behavior
    /// * `messages` - The conversation history as a sequence of messages
    /// * `tools` - Optional list of tools the model can use
    ///
    /// # Returns
    /// A tuple containing the model's response message and provider usage statistics
    ///
    /// # Errors
    /// ProviderError
    ///   - It's important to raise ContextLengthExceeded correctly since agent handles it
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<ProviderCompleteResponse, ProviderError>;

    /// Structured extraction: always JSON‐Schema
    ///
    /// # Arguments
    /// * `system`   – system prompt guiding the extraction task  
    /// * `messages` – conversation history  
    /// * `schema`   – a JSON‐Schema for the expected output.
    ///                 Will set strict=true for OpenAI & Databricks.
    ///
    /// # Returns
    /// A `ProviderExtractResponse` whose `data` is a JSON object matching `schema`.  
    ///
    /// # Errors
    /// * `ProviderError::ContextLengthExceeded` if the prompt is too large  
    /// * other `ProviderError` variants for API/network failures
    async fn extract(
        &self,
        system: &str,
        messages: &[Message],
        schema: &serde_json::Value,
    ) -> Result<ProviderExtractResponse, ProviderError> {
        // Build a tool whose parameters *are* the schema
        let extract_tool = Tool::new(
            "extract_structured_data",
            "Return a JSON object that satisfies the supplied JSON-Schema",
            schema.clone(),
        );

        // Modify the system prompt to specify that tool must be used
        let system = format!(
            "{}\n\nYou must only use the `extract_structured_data` tool to return a JSON object that satisfies the supplied schema:\n{}",
            system, schema
        );

        // Call the complete method with the modified system prompt and the tool
        let ProviderCompleteResponse {
            message,
            model,
            usage,
        } = self
            .complete(&system, messages, std::slice::from_ref(&extract_tool))
            .await?;

        // Find the first tool call in the response message
        let tool_call_result = message
            .content
            .iter()
            .find_map(|c| {
                if let MessageContent::ToolReq(tr) = c {
                    Some(&tr.tool_call.0) // ToolResult<ToolCall>
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                ProviderError::ResponseParseError(
                    "assistant did not issue a structured_extract tool call".into(),
                )
            })?;

        match tool_call_result {
            Ok(tool_call) => {
                let data = tool_call.arguments.clone();
                Ok(ProviderExtractResponse { data, model, usage })
            }
            Err(tool_err) => Err(map_tool_error(tool_err)),
        }
    }
}

fn map_tool_error(err: &ToolError) -> ProviderError {
    match err {
        ToolError::InvalidParameters(msg)
        | ToolError::SchemaError(msg)
        | ToolError::ExecutionError(msg) => ProviderError::ExecutionError(msg.clone()),

        ToolError::NotFound(msg) => ProviderError::RequestFailed(msg.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_creation() {
        let usage = Usage::new(Some(10), Some(20), Some(30));
        assert_eq!(usage.input_tokens, Some(10));
        assert_eq!(usage.output_tokens, Some(20));
        assert_eq!(usage.total_tokens, Some(30));
    }

    #[test]
    fn test_provider_complete_response_creation() {
        let message = Message::user().with_text("Hello, world!");
        let usage = Usage::new(Some(10), Some(20), Some(30));
        let response =
            ProviderCompleteResponse::new(message.clone(), "test_model".to_string(), usage.clone());

        assert_eq!(response.message, message);
        assert_eq!(response.model, "test_model");
        assert_eq!(response.usage, usage);
    }
}
