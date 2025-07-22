use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::errors::ProviderError;
use crate::{message::Message, types::core::Tool};

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
    /// * `request_id` - Optional request ID (only used by some providers like Databricks)
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
        request_id: Option<&str>,
    ) -> Result<ProviderCompleteResponse, ProviderError>;

    /// Structured extraction: always JSON‐Schema
    ///
    /// # Arguments
    /// * `system`   – system prompt guiding the extraction task  
    /// * `messages` – conversation history  
    /// * `schema`   – a JSON‐Schema for the expected output.
    ///                 Will set strict=true for OpenAI & Databricks.
    /// * `request_id` - Optional request ID (only used by some providers like Databricks)
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
        request_id: Option<&str>,
    ) -> Result<ProviderExtractResponse, ProviderError>;
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
