use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::base::{LeadWorkerProviderTrait, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use mcp_core::{tool::Tool, Content};

/// Enum representing the current mode of the LeadWorkerProvider
#[derive(Debug, Clone, PartialEq)]
pub enum LeadWorkerMode {
    Lead,
    Worker,
}

/// Configuration for LeadWorkerProvider
#[derive(Debug, Clone)]
pub struct LeadWorkerConfig {
    pub initial_lead_turns: usize,
    pub failure_threshold: usize,
    pub fallback_turns: usize,
}

impl Default for LeadWorkerConfig {
    fn default() -> Self {
        Self {
            initial_lead_turns: 3,
            failure_threshold: 2,
            fallback_turns: 2,
        }
    }
}

/// Failure detector for identifying task-level failures
#[derive(Default)]
pub struct FailureDetector;

impl FailureDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect task-level failures in the model's response
    pub fn detect_task_failure(&self, message: &Message) -> bool {
        for content in &message.content {
            match content {
                MessageContent::ToolRequest(tool_request) => {
                    if tool_request.tool_call.is_err() {
                        return true;
                    }
                }
                MessageContent::ToolResponse(tool_response) => {
                    if tool_response.tool_result.is_err() {
                        return true;
                    } else if let Ok(contents) = &tool_response.tool_result {
                        if self.contains_error_indicators(contents) {
                            return true;
                        }
                    }
                }
                // TODO these are not correctly applied right now because they are checked on the completion
                // output which is never the user message
                MessageContent::Text(text_content) => {
                    if self.contains_user_correction_patterns(&text_content.text) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if tool output contains error indicators
    fn contains_error_indicators(&self, contents: &[Content]) -> bool {
        for content in contents {
            if let Content::Text(text_content) = content {
                let text_lower = text_content.text.to_lowercase();

                // Common error patterns in tool outputs
                if text_lower.contains("error:")
                    || text_lower.contains("failed:")
                    || text_lower.contains("exception:")
                    || text_lower.contains("traceback")
                    || text_lower.contains("syntax error")
                    || text_lower.contains("permission denied")
                    || text_lower.contains("file not found")
                    || text_lower.contains("command not found")
                    || text_lower.contains("compilation failed")
                    || text_lower.contains("test failed")
                    || text_lower.contains("assertion failed")
                {
                    return true;
                }
            }
        }
        false
    }

    /// Check for user correction patterns in text
    fn contains_user_correction_patterns(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();

        // Patterns indicating user is correcting or expressing dissatisfaction
        text_lower.contains("that's wrong")
            || text_lower.contains("that's not right")
            || text_lower.contains("that doesn't work")
            || text_lower.contains("try again")
            || text_lower.contains("let me correct")
            || text_lower.contains("actually, ")
            || text_lower.contains("no, that's")
            || text_lower.contains("that's incorrect")
            || text_lower.contains("fix this")
            || text_lower.contains("this is broken")
            || text_lower.contains("this doesn't")
            || text_lower.starts_with("no,")
            || text_lower.starts_with("wrong")
            || text_lower.starts_with("incorrect")
    }
}

/// A provider that switches between a lead model and a worker model based on success/failure
pub struct LeadWorkerProvider {
    lead_provider: Arc<dyn Provider>,
    worker_provider: Arc<dyn Provider>,
    config: LeadWorkerConfig,
    lead_turns_remaining: Arc<Mutex<usize>>,
    consecutive_failures: Arc<Mutex<usize>>,
    failure_detector: FailureDetector,
}

impl LeadWorkerProvider {
    /// Create a new LeadWorkerProvider with default settings
    pub fn new(
        lead_provider: Arc<dyn Provider>,
        worker_provider: Arc<dyn Provider>,
        lead_turns: Option<usize>,
    ) -> Self {
        let config = LeadWorkerConfig {
            initial_lead_turns: lead_turns.unwrap_or(3),
            ..Default::default()
        };

        Self {
            lead_provider,
            worker_provider,
            lead_turns_remaining: Arc::new(Mutex::new(config.initial_lead_turns)),
            consecutive_failures: Arc::new(Mutex::new(0)),
            config,
            failure_detector: FailureDetector::new(),
        }
    }

    /// Create a new LeadWorkerProvider with custom settings
    pub fn new_with_settings(
        lead_provider: Arc<dyn Provider>,
        worker_provider: Arc<dyn Provider>,
        lead_turns: usize,
        failure_threshold: usize,
        fallback_turns: usize,
    ) -> Self {
        let config = LeadWorkerConfig {
            initial_lead_turns: lead_turns,
            failure_threshold,
            fallback_turns,
        };

        Self {
            lead_provider,
            worker_provider,
            lead_turns_remaining: Arc::new(Mutex::new(config.initial_lead_turns)),
            consecutive_failures: Arc::new(Mutex::new(0)),
            config,
            failure_detector: FailureDetector::new(),
        }
    }

    /// Get the current mode
    pub async fn get_current_mode(&self) -> LeadWorkerMode {
        let lead_turns_remaining = *self.lead_turns_remaining.lock().await;
        if lead_turns_remaining > 0 {
            LeadWorkerMode::Lead
        } else {
            LeadWorkerMode::Worker
        }
    }

    /// Get the currently active provider and mode
    async fn get_active_provider(&self) -> (Arc<dyn Provider>, LeadWorkerMode) {
        let mode = self.get_current_mode().await;
        let provider = match mode {
            LeadWorkerMode::Lead => Arc::clone(&self.lead_provider),
            LeadWorkerMode::Worker => Arc::clone(&self.worker_provider),
        };
        (provider, mode)
    }

    /// Handle the result of a completion attempt and update state
    async fn handle_completion_result(
        &self,
        result: &Result<(Message, ProviderUsage), ProviderError>,
    ) {
        match result {
            Ok((message, _usage)) => {
                let has_task_failure = self.failure_detector.detect_task_failure(message);

                if has_task_failure {
                    // Handle task failure
                    let mut failures = self.consecutive_failures.lock().await;
                    let mut lead_turns = self.lead_turns_remaining.lock().await;

                    let new_failures = *failures + 1;

                    // If we're in worker mode and hit failure threshold, add fallback turns
                    let should_add_fallback =
                        *lead_turns == 0 && new_failures >= self.config.failure_threshold;

                    if should_add_fallback {
                        *lead_turns = self.config.fallback_turns; // Add fallback turns to get back to lead
                        *failures = 0; // Reset when entering fallback
                    } else {
                        *failures = new_failures;
                        // Keep current lead_turns (don't decrement on failure)
                    }
                } else {
                    // Handle success
                    let mut failures = self.consecutive_failures.lock().await;
                    let mut lead_turns = self.lead_turns_remaining.lock().await;

                    // Only decrement lead turns on SUCCESS (move toward worker when things are going well)
                    *lead_turns = lead_turns.saturating_sub(1);
                    // Reset failure count on success
                    *failures = 0;
                }
            }
            Err(_) => {
                // Technical failure - don't change state
            }
        }
    }

    /// Reset the state - for tests
    #[cfg(test)]
    async fn reset_turn_count(&self) {
        let mut lead_turns = self.lead_turns_remaining.lock().await;
        let mut failures = self.consecutive_failures.lock().await;
        *lead_turns = self.config.initial_lead_turns;
        *failures = 0;
    }

    /// Get the current failure count - for tests
    #[cfg(test)]
    async fn get_failure_count(&self) -> usize {
        *self.consecutive_failures.lock().await
    }
}

#[async_trait]
impl LeadWorkerProviderTrait for LeadWorkerProvider {
    /// Get information about the lead and worker models for logging
    fn get_model_info(&self) -> (String, String) {
        let lead_model = self.lead_provider.get_model_config().model_name;
        let worker_model = self.worker_provider.get_model_config().model_name;
        (lead_model, worker_model)
    }

    /// Get the currently active model name (configured name, not usage name)
    fn get_active_model(&self) -> String {
        // Use try_lock to avoid blocking - this is safe for reading simple state
        let lead_turns_remaining = self
            .lead_turns_remaining
            .try_lock()
            .map(|t| *t)
            .unwrap_or(self.config.initial_lead_turns);

        if lead_turns_remaining > 0 {
            self.lead_provider.get_model_config().model_name
        } else {
            self.worker_provider.get_model_config().model_name
        }
    }

    /// Get the current mode (Lead or Worker)
    async fn get_current_mode(&self) -> LeadWorkerMode {
        self.get_current_mode().await
    }
}

#[async_trait]
impl Provider for LeadWorkerProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "lead_worker",
            "Lead/Worker Provider",
            "A provider that switches between lead and worker models based on success/failure",
            "",     // No default model as this is determined by the wrapped providers
            vec![], // No known models as this depends on wrapped providers
            "",     // No doc link
            vec![], // No config keys as configuration is done through wrapped providers
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        // Return the lead provider's model config as the default
        self.lead_provider.get_model_config()
    }

    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let (provider, current_mode) = self.get_active_provider().await;

        // Make the completion request
        let result = provider.complete(system, messages, tools).await;

        // For technical failures, try with lead provider as fallback
        let final_result = match &result {
            Err(_) if current_mode == LeadWorkerMode::Worker => {
                let default_result = self.lead_provider.complete(system, messages, tools).await;
                match &default_result {
                    Ok(_) => default_result,
                    Err(_) => result, // Return the original error
                }
            }
            _ => result, // Success or lead provider failure
        };

        // Handle the result and update state
        self.handle_completion_result(&final_result).await;

        final_result
    }

    async fn fetch_supported_models_async(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // Combine models from both providers
        let lead_models = self.lead_provider.fetch_supported_models_async().await?;
        let worker_models = self.worker_provider.fetch_supported_models_async().await?;

        match (lead_models, worker_models) {
            (Some(lead), Some(worker)) => {
                let mut all_models = lead;
                all_models.extend(worker);
                all_models.sort();
                all_models.dedup();
                Ok(Some(all_models))
            }
            (Some(models), None) | (None, Some(models)) => Ok(Some(models)),
            (None, None) => Ok(None),
        }
    }

    fn supports_embeddings(&self) -> bool {
        self.lead_provider.supports_embeddings() || self.worker_provider.supports_embeddings()
    }

    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        // Use the lead provider for embeddings if it supports them, otherwise use worker
        if self.lead_provider.supports_embeddings() {
            self.lead_provider.create_embeddings(texts).await
        } else if self.worker_provider.supports_embeddings() {
            self.worker_provider.create_embeddings(texts).await
        } else {
            Err(ProviderError::ExecutionError(
                "Neither lead nor worker provider supports embeddings".to_string(),
            ))
        }
    }

    fn as_lead_worker(&self) -> Option<&dyn LeadWorkerProviderTrait> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use crate::providers::base::{ProviderMetadata, ProviderUsage, Usage};
    use chrono::Utc;
    use mcp_core::{content::TextContent, Role};

    #[derive(Clone)]
    struct MockProvider {
        name: String,
        model_config: ModelConfig,
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message {
                    role: Role::Assistant,
                    created: Utc::now().timestamp(),
                    content: vec![MessageContent::Text(TextContent {
                        text: format!("Response from {}", self.name),
                        annotations: None,
                    })],
                },
                ProviderUsage::new(self.name.clone(), Usage::default()),
            ))
        }
    }

    #[tokio::test]
    async fn test_lead_worker_switching() {
        let lead_provider = Arc::new(MockProvider {
            name: "lead".to_string(),
            model_config: ModelConfig::new("lead-model".to_string()),
        });

        let worker_provider = Arc::new(MockProvider {
            name: "worker".to_string(),
            model_config: ModelConfig::new("worker-model".to_string()),
        });

        let provider = LeadWorkerProvider::new(lead_provider, worker_provider, Some(3));

        // First three successful turns should use lead provider
        // After each success, lead_turns_remaining decrements
        for _i in 0..3 {
            // Check mode before completion
            assert_eq!(provider.get_current_mode().await, LeadWorkerMode::Lead);
            let (_message, usage) = provider.complete("system", &[], &[]).await.unwrap();
            assert_eq!(usage.model, "lead");
        }

        // After 3 successful lead turns, should now be in worker mode
        assert_eq!(provider.get_current_mode().await, LeadWorkerMode::Worker);
        let (_message, usage) = provider.complete("system", &[], &[]).await.unwrap();
        assert_eq!(usage.model, "worker");

        // Should stay in worker mode
        assert_eq!(provider.get_current_mode().await, LeadWorkerMode::Worker);

        // Reset and verify it goes back to lead
        provider.reset_turn_count().await;
        assert_eq!(provider.get_current_mode().await, LeadWorkerMode::Lead);

        let (_message, usage) = provider.complete("system", &[], &[]).await.unwrap();
        assert_eq!(usage.model, "lead");
    }

    #[derive(Clone)]
    struct MockFailureProvider {
        name: String,
        model_config: ModelConfig,
        should_fail: bool,
    }

    #[async_trait]
    impl Provider for MockFailureProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            if self.should_fail {
                Err(ProviderError::ExecutionError(
                    "Simulated failure".to_string(),
                ))
            } else {
                Ok((
                    Message {
                        role: Role::Assistant,
                        created: Utc::now().timestamp(),
                        content: vec![MessageContent::Text(TextContent {
                            text: format!("Response from {}", self.name),
                            annotations: None,
                        })],
                    },
                    ProviderUsage::new(self.name.clone(), Usage::default()),
                ))
            }
        }
    }

    #[tokio::test]
    async fn test_technical_failure_retry() {
        let lead_provider = Arc::new(MockFailureProvider {
            name: "lead".to_string(),
            model_config: ModelConfig::new("lead-model".to_string()),
            should_fail: false, // Lead provider works
        });

        let worker_provider = Arc::new(MockFailureProvider {
            name: "worker".to_string(),
            model_config: ModelConfig::new("worker-model".to_string()),
            should_fail: true, // Worker will fail
        });

        let provider = LeadWorkerProvider::new(lead_provider, worker_provider, Some(1));

        // First turn uses lead (should succeed)
        let result = provider.complete("system", &[], &[]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().1.model, "lead");

        // Second turn uses worker (will fail, but should retry with lead and succeed)
        let result = provider.complete("system", &[], &[]).await;
        assert!(result.is_ok()); // Should succeed because lead provider is used as fallback
        assert_eq!(result.unwrap().1.model, "lead"); // Should be lead provider
        assert_eq!(provider.get_failure_count().await, 0); // No failure tracking for technical failures
    }
}
