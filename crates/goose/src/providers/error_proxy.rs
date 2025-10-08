/// Error Injection Proxy Provider
///
/// A proxy provider that wraps any real provider and can inject errors on demand
/// via external control mechanisms. This allows testing error conditions in a
/// running goose instance without recompilation.
///
/// ## Usage
///
/// 1. Set environment variables:
///    ```bash
///    export GOOSE_PROVIDER=error_proxy
///    export ERROR_PROXY_TARGET_PROVIDER=openai  # or anthropic, etc.
///    export ERROR_PROXY_CONTROL_FILE=/tmp/goose-error-control.json
///    goose session start
///    ```
///
/// 2. Control errors via the control file:
///    ```bash
///    # Enable rate limit errors
///    echo '{"enabled": true, "error_type": "rate_limit", "pattern": "every_nth", "nth": 3}' > /tmp/goose-error-control.json
///    
///    # Disable errors
///    echo '{"enabled": false}' > /tmp/goose-error-control.json
///    ```
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::base::{Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::factory;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

/// Control configuration for error injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorControl {
    /// Whether error injection is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Type of error to inject
    #[serde(default)]
    pub error_type: ErrorType,

    /// Pattern for when to inject errors
    #[serde(default)]
    pub pattern: ErrorPattern,

    /// For every_nth pattern: inject error every N calls
    #[serde(default = "default_nth")]
    pub nth: usize,

    /// For random pattern: probability of error (0.0 to 1.0)
    #[serde(default = "default_probability")]
    pub probability: f64,

    /// For burst pattern: number of consecutive errors
    #[serde(default = "default_burst_count")]
    pub burst_count: usize,

    /// For rate limit errors: retry after seconds
    #[serde(default = "default_retry_after")]
    pub retry_after_seconds: u64,

    /// Optional: only inject errors for specific models
    pub target_models: Option<Vec<String>>,

    /// Optional: custom error message
    pub custom_message: Option<String>,
}

fn default_nth() -> usize {
    3
}
fn default_probability() -> f64 {
    0.5
}
fn default_burst_count() -> usize {
    3
}
fn default_retry_after() -> u64 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    #[default]
    RateLimit,
    ContextExceeded,
    ServerError,
    AuthError,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ErrorPattern {
    #[default]
    EveryNth, // Error every Nth call
    Random,     // Random errors with given probability
    Burst,      // Burst of errors then normal
    Continuous, // Always error when enabled
    Once,       // Error once then disable
}

impl Default for ErrorControl {
    fn default() -> Self {
        Self {
            enabled: false,
            error_type: ErrorType::default(),
            pattern: ErrorPattern::default(),
            nth: default_nth(),
            probability: default_probability(),
            burst_count: default_burst_count(),
            retry_after_seconds: default_retry_after(),
            target_models: None,
            custom_message: None,
        }
    }
}

/// Error injection proxy provider
pub struct ErrorProxyProvider {
    /// The wrapped provider
    inner: Arc<dyn Provider>,

    /// Path to the control file
    control_file: PathBuf,

    /// Call counter for patterns
    call_counter: Arc<AtomicUsize>,

    /// Burst counter
    burst_counter: Arc<AtomicUsize>,
}

impl ErrorProxyProvider {
    /// Create a new error proxy provider
    pub fn new(inner: Arc<dyn Provider>, control_file: PathBuf) -> Self {
        // Create default control file if it doesn't exist
        if !control_file.exists() {
            let default_control = ErrorControl::default();
            let _ = fs::write(
                &control_file,
                serde_json::to_string_pretty(&default_control).unwrap(),
            );
        }

        Self {
            inner,
            control_file,
            call_counter: Arc::new(AtomicUsize::new(0)),
            burst_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create from environment variables
    pub fn from_env(model_config: ModelConfig) -> Result<Self> {
        // Get target provider from env
        let target_provider =
            std::env::var("ERROR_PROXY_TARGET_PROVIDER").unwrap_or_else(|_| "openai".to_string());

        // Get control file path
        let control_file = std::env::var("ERROR_PROXY_CONTROL_FILE")
            .unwrap_or_else(|_| "/tmp/goose-error-control.json".to_string());

        // Create the inner provider
        let inner = factory::create(&target_provider, model_config)?;

        Ok(Self::new(inner, PathBuf::from(control_file)))
    }

    /// Read the current control configuration
    fn read_control(&self) -> ErrorControl {
        fs::read_to_string(&self.control_file)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Check if we should inject an error based on the current configuration
    fn should_inject_error(&self, model_name: &str) -> Option<ErrorControl> {
        let control = self.read_control();

        if !control.enabled {
            return None;
        }

        // Check if this model is targeted (if target_models is specified)
        if let Some(ref targets) = control.target_models {
            if !targets.iter().any(|t| t == model_name) {
                return None;
            }
        }

        // Check pattern
        let should_error = match control.pattern {
            ErrorPattern::Continuous => true,
            ErrorPattern::EveryNth => {
                let count = self.call_counter.fetch_add(1, Ordering::SeqCst);
                (count + 1) % control.nth == 0
            }
            ErrorPattern::Random => {
                use rand::Rng;
                rand::thread_rng().gen::<f64>() < control.probability
            }
            ErrorPattern::Burst => {
                let burst_count = self.burst_counter.load(Ordering::SeqCst);
                if burst_count < control.burst_count {
                    self.burst_counter.fetch_add(1, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
            ErrorPattern::Once => {
                // Check if we've already errored once
                let count = self.call_counter.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    // Disable after this error
                    let mut control_copy = control.clone();
                    control_copy.enabled = false;
                    let _ = fs::write(
                        &self.control_file,
                        serde_json::to_string_pretty(&control_copy).unwrap(),
                    );
                    true
                } else {
                    false
                }
            }
        };

        if should_error {
            Some(control)
        } else {
            None
        }
    }

    /// Create an error based on the control configuration
    fn create_error(&self, control: &ErrorControl) -> ProviderError {
        let message = control.custom_message.as_deref();

        match control.error_type {
            ErrorType::RateLimit => ProviderError::RateLimitExceeded {
                details: message
                    .unwrap_or("Error proxy: Simulated rate limit error")
                    .to_string(),
                retry_delay: Some(Duration::from_secs(control.retry_after_seconds)),
            },
            ErrorType::ContextExceeded => ProviderError::ContextLengthExceeded(
                message
                    .unwrap_or("Error proxy: Simulated context length exceeded")
                    .to_string(),
            ),
            ErrorType::ServerError => ProviderError::ServerError(
                message
                    .unwrap_or("Error proxy: Simulated server error (500)")
                    .to_string(),
            ),
            ErrorType::AuthError => ProviderError::Authentication(
                message
                    .unwrap_or("Error proxy: Simulated authentication error")
                    .to_string(),
            ),
            ErrorType::Timeout => ProviderError::RequestFailed(
                message
                    .unwrap_or("Error proxy: Simulated timeout")
                    .to_string(),
            ),
        }
    }

    /// Reset counters (useful for testing)
    pub fn reset_counters(&self) {
        self.call_counter.store(0, Ordering::SeqCst);
        self.burst_counter.store(0, Ordering::SeqCst);
    }
}

#[async_trait]
impl Provider for ErrorProxyProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "error_proxy",
            "Error Injection Proxy",
            "A proxy provider that can inject errors for testing",
            "proxy",
            vec![],
            "https://github.com/block/goose",
            vec![],
        )
    }

    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Check if we should inject an error
        if let Some(control) = self.should_inject_error(&model_config.model_name) {
            tracing::warn!(
                "ErrorProxyProvider: Injecting {} error for model {}",
                serde_json::to_string(&control.error_type).unwrap_or_default(),
                model_config.model_name
            );
            return Err(self.create_error(&control));
        }

        // Otherwise, pass through to the inner provider
        tracing::debug!("ErrorProxyProvider: Passing through to inner provider");
        self.inner
            .complete_with_model(model_config, system, messages, tools)
            .await
    }

    fn get_model_config(&self) -> ModelConfig {
        self.inner.get_model_config()
    }

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<super::base::MessageStream, ProviderError> {
        // Check if we should inject an error
        let model_config = self.get_model_config();
        if let Some(control) = self.should_inject_error(&model_config.model_name) {
            tracing::warn!("ErrorProxyProvider: Injecting error in stream");
            return Err(self.create_error(&control));
        }

        self.inner.stream(system, messages, tools).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    // Mock provider for testing
    struct MockInnerProvider {
        model_config: ModelConfig,
    }

    #[async_trait]
    impl Provider for MockInnerProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::new("mock", "Mock", "Mock provider", "mock", vec![], "", vec![])
        }

        async fn complete_with_model(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock".to_string(), super::super::base::Usage::default()),
            ))
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }
    }

    #[tokio::test]
    async fn test_error_injection() {
        let temp_file = NamedTempFile::new().unwrap();
        let control_path = temp_file.path().to_path_buf();

        // Create control config
        let control = ErrorControl {
            enabled: true,
            error_type: ErrorType::RateLimit,
            pattern: ErrorPattern::EveryNth,
            nth: 2,
            ..Default::default()
        };
        fs::write(&control_path, serde_json::to_string(&control).unwrap()).unwrap();

        // Create proxy provider
        let inner = Arc::new(MockInnerProvider {
            model_config: ModelConfig::new_or_fail("test-model"),
        });
        let proxy = ErrorProxyProvider::new(inner, control_path);

        // First call should succeed
        let result1 = proxy.complete("test", &[], &[]).await;
        assert!(result1.is_ok());

        // Second call should error (every 2nd)
        let result2 = proxy.complete("test", &[], &[]).await;
        assert!(result2.is_err());
        match result2.unwrap_err() {
            ProviderError::RateLimitExceeded { .. } => {}
            _ => panic!("Expected rate limit error"),
        }

        // Third call should succeed
        let result3 = proxy.complete("test", &[], &[]).await;
        assert!(result3.is_ok());
    }
}
