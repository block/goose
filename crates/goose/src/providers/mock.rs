use super::base::ProviderUsage;
use crate::message::Message;
use crate::providers::base::{Provider, Usage};
use crate::providers::configs::ModelConfig;
use anyhow::Result;
use async_trait::async_trait;
use mcp_core::tool::Tool;
use rust_decimal_macros::dec;
use serde_json::Value;
use std::sync::Arc;
use std::sync::Mutex;

/// A mock provider that returns pre-configured responses for testing
pub struct MockProvider {
    responses: Arc<Mutex<Vec<Message>>>,
    model_config: ModelConfig,
}

impl MockProvider {
    /// Create a new mock provider with a sequence of responses
    pub fn new(responses: Vec<Message>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            model_config: ModelConfig::new("mock".to_string()),
        }
    }

    /// Create a new mock provider with specific responses and model config
    pub fn with_config(responses: Vec<Message>, model_config: ModelConfig) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            model_config,
        }
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn get_model_config(&self) -> &ModelConfig {
        &self.model_config
    }

    async fn complete(
        &self,
        _system_prompt: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<(Message, ProviderUsage)> {
        let mut responses = self.responses.lock().unwrap();
        let usage = Usage::new(Some(1), Some(1), Some(2));
        if responses.is_empty() {
            // Return empty response if no more pre-configured responses
            Ok((
                Message::assistant().with_text(""),
                ProviderUsage::new("mock".to_string(), usage, Some(dec!(1))),
            ))
        } else {
            Ok((
                responses.remove(0),
                ProviderUsage::new("mock".to_string(), usage, Some(dec!(1))),
            ))
        }
    }

    fn get_usage(&self, _data: &Value) -> Result<Usage> {
        Ok(Usage::new(None, None, None))
    }
}
