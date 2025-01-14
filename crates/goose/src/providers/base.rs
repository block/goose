use anyhow::Result;
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::select;
use tokio::sync::RwLock;

use super::configs::ModelConfig;
use crate::message::{Message, MessageContent};
use mcp_core::role::Role;
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
        // Get the latest user message
        let latest_user_msg = messages
            .iter()
            .rev()
            .find(|msg| {
                msg.role == Role::User
                    && msg
                        .content
                        .iter()
                        .any(|content| matches!(content, MessageContent::Text(_)))
            })
            .ok_or_else(|| anyhow::anyhow!("No user message with text content found in history"))?;

        // Get the content to moderate
        let content = latest_user_msg.content.first().unwrap().as_text().unwrap();

        // Start completion and moderation immediately
        let completion_fut = self.complete_internal(system, messages, tools);
        let moderation_fut = self.moderate_content(content);
        tokio::pin!(completion_fut);
        tokio::pin!(moderation_fut);

        // Run moderation and completion concurrently
        select! {
            moderation = &mut moderation_fut => {
                let result = moderation?;

                if result.flagged {
                    let categories = result.categories
                        .unwrap_or_else(|| vec!["unknown".to_string()])
                        .join(", ");
                    return Err(ModerationError::ContentFlagged {
                        categories,
                        category_scores: result.category_scores,
                    }.into());
                }

                // Moderation passed, wait for completion
                Ok(completion_fut.await?)
            }
            completion = &mut completion_fut => {
                // Completion finished first, still need to check moderation
                let completion_result = completion?;
                let moderation_result = moderation_fut.await?;

                if moderation_result.flagged {
                    let categories = moderation_result.categories
                        .unwrap_or_else(|| vec!["unknown".to_string()])
                        .join(", ");
                    return Err(ModerationError::ContentFlagged {
                        categories,
                        category_scores: moderation_result.category_scores,
                    }.into());
                }

                Ok(completion_result)
            }
        }
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
    use mcp_core::content::TextContent;
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::sleep;

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

    #[test]
    fn test_moderation_result_creation() {
        let categories = vec!["hate".to_string(), "violence".to_string()];
        let scores = json!({
            "hate": 0.9,
            "violence": 0.8
        });
        let result = ModerationResult::new(true, Some(categories.clone()), Some(scores.clone()));

        assert!(result.flagged);
        assert_eq!(result.categories.unwrap(), categories);
        assert_eq!(result.category_scores.unwrap(), scores);
    }

    #[tokio::test]
    async fn test_moderation_blocks_completion() {
        #[derive(Clone)]
        struct TestProvider;

        #[async_trait]
        impl Moderation for TestProvider {
            async fn moderate_content_internal(&self, _content: &str) -> Result<ModerationResult> {
                // Return quickly with flagged content
                Ok(ModerationResult::new(
                    true,
                    Some(vec!["test".to_string()]),
                    None,
                ))
            }
        }

        #[async_trait]
        impl Provider for TestProvider {
            fn get_usage(&self, _data: &Value) -> Result<Usage> {
                Ok(Usage::new(Some(1), Some(1), Some(2)))
            }

            fn get_model_config(&self) -> &ModelConfig {
                panic!("Should not be called");
            }

            async fn complete_internal(
                &self,
                _system: &str,
                _messages: &[Message],
                _tools: &[Tool],
            ) -> Result<(Message, ProviderUsage)> {
                // Simulate a slow completion
                sleep(Duration::from_secs(1)).await;
                panic!("complete_internal should not finish when moderation fails");
            }
        }

        let provider = TestProvider;
        let test_message = Message {
            role: Role::User,
            created: chrono::Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: "test".to_string(),
                annotations: None,
            })],
        };

        let result = provider.complete("system", &[test_message], &[]).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.downcast_ref::<ModerationError>().is_some());
    }

    #[tokio::test]
    async fn test_moderation_blocks_completion_delayed() {
        #[derive(Clone)]
        struct TestProvider;

        #[async_trait]
        impl Moderation for TestProvider {
            async fn moderate_content_internal(&self, _content: &str) -> Result<ModerationResult> {
                sleep(Duration::from_secs(1)).await;
                // Return quickly with flagged content
                Ok(ModerationResult::new(
                    true,
                    Some(vec!["test".to_string()]),
                    None,
                ))
            }
        }

        #[async_trait]
        impl Provider for TestProvider {
            fn get_usage(&self, _data: &Value) -> Result<Usage> {
                Ok(Usage::new(Some(1), Some(1), Some(2)))
            }

            fn get_model_config(&self) -> &ModelConfig {
                panic!("Should not be called");
            }

            async fn complete_internal(
                &self,
                _system: &str,
                _messages: &[Message],
                _tools: &[Tool],
            ) -> Result<(Message, ProviderUsage)> {
                // Simulate a fast completion=
                Ok((
                    Message {
                        role: Role::Assistant,
                        created: chrono::Utc::now().timestamp(),
                        content: vec![MessageContent::text("test response")],
                    },
                    ProviderUsage::new(
                        "test-model".to_string(),
                        Usage::new(Some(1), Some(1), Some(2)),
                        None,
                    ),
                ))
            }
        }

        let provider = TestProvider;
        let test_message = Message {
            role: Role::User,
            created: chrono::Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: "test".to_string(),
                annotations: None,
            })],
        };

        let result = provider.complete("system", &[test_message], &[]).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.downcast_ref::<ModerationError>().is_some());
    }

    #[tokio::test]
    async fn test_moderation_pass_completion_pass() {
        // Create a dedicated cache for this test
        let cache = Arc::new(ModerationCache::new());

        #[derive(Clone)]
        struct TestProvider {
            cache: Arc<ModerationCache>,
        }

        impl TestProvider {
            fn new(cache: Arc<ModerationCache>) -> Self {
                Self { cache }
            }
        }

        #[async_trait]
        impl Moderation for TestProvider {
            fn moderation_cache(&self) -> &ModerationCache {
                &self.cache
            }

            async fn moderate_content_internal(&self, _content: &str) -> Result<ModerationResult> {
                Ok(ModerationResult::new(false, None, None))
            }
        }

        #[async_trait]
        impl Provider for TestProvider {
            fn get_usage(&self, _data: &Value) -> Result<Usage> {
                Ok(Usage::new(Some(1), Some(1), Some(2)))
            }

            fn get_model_config(&self) -> &ModelConfig {
                panic!("Should not be called");
            }

            async fn complete_internal(
                &self,
                _system: &str,
                _messages: &[Message],
                _tools: &[Tool],
            ) -> Result<(Message, ProviderUsage)> {
                Ok((
                    Message {
                        role: Role::Assistant,
                        created: chrono::Utc::now().timestamp(),
                        content: vec![MessageContent::text("test response")],
                    },
                    ProviderUsage::new(
                        "test-model".to_string(),
                        Usage::new(Some(1), Some(1), Some(2)),
                        None,
                    ),
                ))
            }
        }

        let provider = TestProvider::new(cache);
        let test_message = Message {
            role: Role::User,
            created: chrono::Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: "test".to_string(),
                annotations: None,
            })],
        };

        let result = provider.complete("system", &[test_message], &[]).await;
        assert!(result.is_ok(), "Expected Ok result, got {:?}", result);

        let (message, usage) = result.unwrap();
        assert_eq!(message.content[0].as_text().unwrap(), "test response");
        assert_eq!(usage.model, "test-model");
    }

    #[tokio::test]
    async fn test_completion_succeeds_when_moderation_passes() {
        #[derive(Clone)]
        struct TestProvider;

        #[async_trait]
        impl Moderation for TestProvider {
            async fn moderate_content_internal(&self, _content: &str) -> Result<ModerationResult> {
                // Simulate some processing time
                sleep(Duration::from_millis(100)).await;
                Ok(ModerationResult::new(false, None, None))
            }
        }

        #[async_trait]
        impl Provider for TestProvider {
            fn get_usage(&self, _data: &Value) -> Result<Usage> {
                Ok(Usage::new(Some(1), Some(1), Some(2)))
            }

            fn get_model_config(&self) -> &ModelConfig {
                panic!("Should not be called");
            }

            async fn complete_internal(
                &self,
                _system: &str,
                _messages: &[Message],
                _tools: &[Tool],
            ) -> Result<(Message, ProviderUsage)> {
                Ok((
                    Message {
                        role: Role::Assistant,
                        created: chrono::Utc::now().timestamp(),
                        content: vec![MessageContent::text("test response")],
                    },
                    ProviderUsage::new(
                        "test-model".to_string(),
                        Usage::new(Some(1), Some(1), Some(2)),
                        None,
                    ),
                ))
            }
        }

        let provider = TestProvider;
        let test_message = Message {
            role: Role::User,
            created: chrono::Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: "test".to_string(),
                annotations: None,
            })],
        };

        let result = provider.complete("system", &[test_message], &[]).await;

        assert!(result.is_ok());
        let (message, usage) = result.unwrap();
        assert_eq!(message.content[0].as_text().unwrap(), "test response");
        assert_eq!(usage.model, "test-model");
    }

    #[tokio::test]
    async fn test_moderation_cache() {
        // Create a local cache for this test
        let cache = Arc::new(ModerationCache::new());

        #[derive(Clone)]
        struct TestProvider {
            moderation_count: Arc<RwLock<i32>>,
            cache: Arc<ModerationCache>,
        }

        impl TestProvider {
            fn new(cache: Arc<ModerationCache>, count: Arc<RwLock<i32>>) -> Self {
                Self {
                    moderation_count: count,
                    cache,
                }
            }
        }

        #[async_trait]
        impl Moderation for TestProvider {
            fn moderation_cache(&self) -> &ModerationCache {
                &self.cache
            }

            async fn moderate_content_internal(&self, _content: &str) -> Result<ModerationResult> {
                // Increment the moderation count
                let mut count = self.moderation_count.write().await;
                *count += 1;

                Ok(ModerationResult::new(false, None, None))
            }
        }

        #[async_trait]
        impl Provider for TestProvider {
            fn get_usage(&self, _data: &Value) -> Result<Usage> {
                Ok(Usage::new(Some(1), Some(1), Some(2)))
            }

            fn get_model_config(&self) -> &ModelConfig {
                panic!("Should not be called");
            }

            async fn complete_internal(
                &self,
                _system: &str,
                _messages: &[Message],
                _tools: &[Tool],
            ) -> Result<(Message, ProviderUsage)> {
                Ok((
                    Message {
                        role: Role::Assistant,
                        created: chrono::Utc::now().timestamp(),
                        content: vec![MessageContent::text("test response")],
                    },
                    ProviderUsage::new(
                        "test-model".to_string(),
                        Usage::new(Some(1), Some(1), Some(2)),
                        None,
                    ),
                ))
            }
        }

        let count = Arc::new(RwLock::new(0));
        let provider = TestProvider::new(cache.clone(), count.clone());
        let test_message = Message {
            role: Role::User,
            created: chrono::Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: "test".to_string(),
                annotations: None,
            })],
        };

        // First call should trigger moderation
        let result = provider
            .complete("system", &[test_message.clone()], &[])
            .await;
        assert!(result.is_ok(), "First call failed: {:?}", result);

        // Second call with same message should use cache
        let result = provider.complete("system", &[test_message], &[]).await;
        assert!(result.is_ok(), "Second call failed: {:?}", result);

        // Check that moderation was only called once
        let count = count.read().await;
        assert_eq!(
            *count, 1,
            "Expected moderation to be called once, got {}",
            *count
        );
    }
}
