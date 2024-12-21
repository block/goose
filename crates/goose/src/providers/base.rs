use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::configs::ModelConfig;
use crate::message::Message;
use mcp_core::tool::Tool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub model: String,
    pub usage: Usage,
    pub cost: Option<Decimal>,
}

impl ProviderUsage {
    pub fn new(model: String, usage: Usage, cost: Option<Decimal>) -> Self {
        Self { model, usage, cost }
    }
}

#[derive(Debug, Clone)]
pub struct Pricing {
    /// Prices are per million tokens.
    pub input_token_price: Decimal,
    pub output_token_price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

use async_trait::async_trait;
use serde_json::Value;

/// Base trait for AI providers (OpenAI, Anthropic, etc)
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the model configuration
    fn get_model_config(&self) -> &ModelConfig;

    /// Generate the next message using the configured model and other parameters
    ///
    /// # Arguments
    /// * `system` - The system prompt that guides the model's behavior
    /// * `messages` - The conversation history as a sequence of messages
    /// * `tools` - Optional list of tools the model can use
    ///
    /// # Returns
    /// A tuple containing the model's response message and provider usage statistics
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage)>;

    fn get_usage(&self, data: &Value) -> Result<Usage>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_usage_creation() {
        let usage = Usage::new(Some(10), Some(20), Some(30));
        assert_eq!(usage.input_tokens, Some(10));
        assert_eq!(usage.output_tokens, Some(20));
        assert_eq!(usage.total_tokens, Some(30));
    }

    #[test]
    fn test_usage_serialization() -> Result<()> {
        let usage = Usage::new(Some(10), Some(20), Some(30));
        let serialized = serde_json::to_string(&usage)?;
        let deserialized: Usage = serde_json::from_str(&serialized)?;

        assert_eq!(usage.input_tokens, deserialized.input_tokens);
        assert_eq!(usage.output_tokens, deserialized.output_tokens);
        assert_eq!(usage.total_tokens, deserialized.total_tokens);

        // Test JSON structure
        let json_value: serde_json::Value = serde_json::from_str(&serialized)?;
        assert_eq!(json_value["input_tokens"], json!(10));
        assert_eq!(json_value["output_tokens"], json!(20));
        assert_eq!(json_value["total_tokens"], json!(30));

        Ok(())
    }
}
