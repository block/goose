use anyhow::Result;
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use super::configs::ModelConfig;
use crate::message::Message;
use mcp_core::tool::Tool;

#[derive(Error, Debug)]
pub enum ModerationError {
    #[error("Content was flagged for moderation in categories: {categories}")]
    ContentFlagged {
        categories: String,
        category_scores: Option<serde_json::Value>,
    },
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationResult {
    /// Whether the content was flagged as inappropriate
    pub flagged: bool,
    /// Optional categories that were flagged (provider specific)
    pub categories: Option<Vec<String>>,
    /// Optional scores for each category (provider specific)
    pub category_scores: Option<serde_json::Value>,
}

impl ModerationResult {
    pub fn new(
        flagged: bool,
        categories: Option<Vec<String>>,
        category_scores: Option<serde_json::Value>,
    ) -> Self {
        Self {
            flagged,
            categories,
            category_scores,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModerationCache {
    cache: Arc<RwLock<HashMap<String, ModerationResult>>>,
}

impl ModerationCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, content: &str) -> Option<ModerationResult> {
        let cache = self.cache.read().await;
        cache.get(content).cloned()
    }

    pub async fn set(&self, content: String, result: ModerationResult) {
        let mut cache = self.cache.write().await;
        cache.insert(content, result);
    }
}

lazy_static! {
    static ref DEFAULT_CACHE: ModerationCache = ModerationCache::new();
}

use async_trait::async_trait;
use serde_json::Value;

/// Trait for handling content moderation
#[async_trait]
pub trait Moderation: Send + Sync {
    /// Get the moderation cache
    fn moderation_cache(&self) -> &ModerationCache {
        &DEFAULT_CACHE
    }

    /// Internal moderation method to be implemented by providers
    async fn moderate_content_internal(&self, _content: &str) -> Result<ModerationResult> {
        Ok(ModerationResult::new(false, None, None))
    }

    /// Moderate the given content
    ///
    /// # Arguments
    /// * `content` - The text content to moderate
    ///
    /// # Returns
    /// A ModerationResult containing the moderation decision and details
    async fn moderate_content(&self, content: &str) -> Result<ModerationResult> {
        // Check cache first
        if let Some(cached) = self.moderation_cache().get(content).await {
            return Ok(cached);
        }

        // If not in cache, do moderation
        let result = self.moderate_content_internal(content).await?;

        // Cache the result
        self.moderation_cache()
            .set(content.to_string(), result.clone())
            .await;

        Ok(result)
    }
}

/// Base trait for AI providers (OpenAI, Anthropic, etc)
#[async_trait]
pub trait Provider: Send + Sync + Moderation {
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
    ) -> Result<(Message, ProviderUsage)> {
        self.complete_internal(system, messages, tools).await

        // Get the latest user message
        //let latest_user_msg = messages
        //    .iter()
        //    .rev()
        //    .find(|msg| {
        //        msg.role == Role::User
        //            && msg
        //                .content
        //                .iter()
        //                .any(|content| matches!(content, MessageContent::Text(_)))
        //    })
        //    .ok_or_else(|| anyhow::anyhow!("No user message with text content found in history"))?;
        //
        //// Get the content to moderate
        //let content = latest_user_msg.content.first().unwrap().as_text().unwrap();

        // Start completion and moderation immediately
        //let moderation_fut = self.moderate_content(content);
        //tokio::pin!(completion_fut);
        //tokio::pin!(moderation_fut);

        // Run moderation and completion concurrently
        //select! {
        //    moderation = &mut moderation_fut => {
        //        let result = moderation?;
        //
        //        if result.flagged {
        //            let categories = result.categories
        //                .unwrap_or_else(|| vec!["unknown".to_string()])
        //                .join(", ");
        //            return Err(ModerationError::ContentFlagged {
        //                categories,
        //                category_scores: result.category_scores,
        //            }.into());
        //        }
        //
        //        // Moderation passed, wait for completion
        //        Ok(completion_fut.await?)
        //    }
        //    completion = &mut completion_fut => {
        //        // Completion finished first, still need to check moderation
        //        let completion_result = completion?;
        //        let moderation_result = moderation_fut.await?;
        //
        //        if moderation_result.flagged {
        //            let categories = moderation_result.categories
        //                .unwrap_or_else(|| vec!["unknown".to_string()])
        //                .join(", ");
        //            return Err(ModerationError::ContentFlagged {
        //                categories,
        //                category_scores: moderation_result.category_scores,
        //            }.into());
        //        }
        //
        //        Ok(completion_result)
        //    }
        //}
    }

    /// Internal completion method to be implemented by providers
    async fn complete_internal(
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
