use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen::prelude::*,
    js_sys::Promise,
    wasm_bindgen_futures::JsFuture,
};

use crate::{
    message::Message,
    model::ModelConfig,
    providers::{
        base::{Provider, ProviderCompleteResponse, ProviderExtractResponse, Usage},
        errors::ProviderError,
    },
    types::core::{Tool, Role},
};

/// Configuration for the mock provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockProviderConfig {
    /// Simulate a delay in milliseconds
    pub delay_ms: Option<u64>,
    /// Force a specific error (for testing error handling)
    pub force_error: Option<String>,
    /// Mock token counts for usage statistics
    pub mock_tokens: Option<MockTokenCounts>,
}

/// Mock token counts for usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockTokenCounts {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
}

impl Default for MockProviderConfig {
    fn default() -> Self {
        Self {
            delay_ms: None,
            force_error: None,
            mock_tokens: Some(MockTokenCounts {
                input_tokens: Some(10),
                output_tokens: Some(20),
                total_tokens: Some(30),
            }),
        }
    }
}

/// A mock provider for testing and WebAssembly environments
pub struct MockProvider {
    config: MockProviderConfig,
    model: ModelConfig,
}

impl MockProvider {
    pub fn new(config: MockProviderConfig, model: ModelConfig) -> Self {
        Self { config, model }
    }

    pub fn from_config(config: MockProviderConfig, model: ModelConfig) -> Result<Self, ProviderError> {
        Ok(Self::new(config, model))
    }

    /// Create a mock response message
    fn create_mock_message(&self, system: &str, messages: &[Message]) -> Message {
        // Create a simple response based on the last user message
        let last_user_message = messages.iter().rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.content.concat_text_str())
            .unwrap_or_else(|| "No user message found".to_string());
        
        let response_text = format!(
            "This is a mock response to: '{}'\n\nSystem prompt was: '{}'", 
            last_user_message, 
            system
        );
        
        Message::assistant().with_text(response_text)
    }

    /// Create mock usage statistics
    fn create_mock_usage(&self) -> Usage {
        if let Some(mock_tokens) = &self.config.mock_tokens {
            Usage::new(
                mock_tokens.input_tokens,
                mock_tokens.output_tokens,
                mock_tokens.total_tokens,
            )
        } else {
            Usage::default()
        }
    }

    /// Check if we should force an error
    fn check_force_error(&self) -> Result<(), ProviderError> {
        if let Some(error) = &self.config.force_error {
            match error.as_str() {
                "auth" => Err(ProviderError::Authentication("Mock authentication error".into())),
                "context" => Err(ProviderError::ContextLengthExceeded("Mock context length exceeded".into())),
                "rate" => Err(ProviderError::RateLimitExceeded("Mock rate limit exceeded".into())),
                "server" => Err(ProviderError::ServerError("Mock server error".into())),
                "request" => Err(ProviderError::RequestFailed("Mock request failed".into())),
                "execution" => Err(ProviderError::ExecutionError("Mock execution error".into())),
                "usage" => Err(ProviderError::UsageError("Mock usage error".into())),
                "parse" => Err(ProviderError::ResponseParseError("Mock parse error".into())),
                _ => Err(ProviderError::ExecutionError(format!("Unknown mock error: {}", error))),
            }
        } else {
            Ok(())
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for MockProvider {
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<ProviderCompleteResponse, ProviderError> {
        // Check if we should force an error
        self.check_force_error()?;
        
        // Simulate delay
        if let Some(_delay_ms) = self.config.delay_ms {
            #[cfg(not(target_arch = "wasm32"))]
            {
                tokio::time::sleep(std::time::Duration::from_millis(_delay_ms)).await;
            }
            
            #[cfg(target_arch = "wasm32")]
            {
                // In WebAssembly, we need to use a different approach
                // We'll just return immediately since the delay is simulated in JavaScript
            }
        }
        
        // Create mock response
        let message = self.create_mock_message(system, messages);
        let usage = self.create_mock_usage();
        
        Ok(ProviderCompleteResponse::new(
            message,
            self.model.model_name.clone(),
            usage,
        ))
    }

    async fn extract(
        &self,
        system: &str,
        messages: &[Message],
        schema: &serde_json::Value,
    ) -> Result<ProviderExtractResponse, ProviderError> {
        // Check if we should force an error
        self.check_force_error()?;
        
        // Simulate delay
        if let Some(_delay_ms) = self.config.delay_ms {
            #[cfg(not(target_arch = "wasm32"))]
            {
                tokio::time::sleep(std::time::Duration::from_millis(_delay_ms)).await;
            }
        }
        
        // Create a simple mock response based on the schema
        let schema_str = serde_json::to_string_pretty(schema)
            .unwrap_or_else(|_| "Invalid schema".to_string());
        
        // Create a simple mock data object that conforms to the schema
        // In a real implementation, this would be more sophisticated
        let mock_data = serde_json::json!({
            "mockResponse": true,
            "system": system,
            "lastMessage": messages.last().map(|m| m.content.concat_text_str()).unwrap_or_default(),
            "schemaProvided": schema_str,
        });
        
        let usage = self.create_mock_usage();
        
        Ok(ProviderExtractResponse::new(
            mock_data,
            self.model.model_name.clone(),
            usage,
        ))
    }
}

// WebAssembly-specific helper functions
#[cfg(target_arch = "wasm32")]
pub mod wasm_helpers {
    use super::*;
    use crate::types::completion::{CompletionRequest, CompletionResponse};

    // Helper function to create a JavaScript Promise that resolves after a delay
    pub fn create_delay_promise(_delay_ms: f64) -> Promise {
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            // In a real implementation, we would use setTimeout here
            // But for now, we'll just resolve immediately
            resolve.call0(&JsValue::NULL).unwrap();
        });
        
        promise
    }
    
    // Helper function to execute a completion request with a mock provider
    pub async fn execute_mock_completion(
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        // Parse the mock config
        let config: MockProviderConfig = match serde_json::from_value(request.provider_config.clone()) {
            Ok(config) => config,
            Err(_) => MockProviderConfig::default(),
        };
        
        // Check if we should force an error
        if let Some(error) = &config.force_error {
            return Err(match error.as_str() {
                "auth" => ProviderError::Authentication("Mock authentication error".into()),
                "context" => ProviderError::ContextLengthExceeded("Mock context length exceeded".into()),
                "rate" => ProviderError::RateLimitExceeded("Mock rate limit exceeded".into()),
                "server" => ProviderError::ServerError("Mock server error".into()),
                "request" => ProviderError::RequestFailed("Mock request failed".into()),
                "execution" => ProviderError::ExecutionError("Mock execution error".into()),
                "usage" => ProviderError::UsageError("Mock usage error".into()),
                "parse" => ProviderError::ResponseParseError("Mock parse error".into()),
                _ => ProviderError::ExecutionError(format!("Unknown mock error: {}", error)),
            });
        }
        
        // Simulate delay if configured
        if let Some(delay_ms) = config.delay_ms {
            let delay_promise = create_delay_promise(delay_ms as f64);
            let _ = JsFuture::from(delay_promise).await;
        }
        
        // Create a mock provider
        let provider = MockProvider::new(config, request.model_config.clone());
        
        // Call the provider's complete method
        let response = provider
            .complete(&request.system_preamble, &request.messages, &[])
            .await?;
        
        // Create runtime metrics
        let total_time = 0.1; // Placeholder
        let provider_time = 0.05; // Placeholder
        let tokens_per_sec = response.usage.total_tokens.map(|t| t as f64 / provider_time as f64);
        
        // Create the completion response
        let completion_response = crate::types::completion::CompletionResponse::new(
            response.message,
            response.model,
            response.usage,
            crate::types::completion::RuntimeMetrics::new(
                total_time,
                provider_time,
                tokens_per_sec,
            ),
        );
        
        Ok(completion_response)
    }
}