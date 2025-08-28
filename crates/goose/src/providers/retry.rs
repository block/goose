use super::errors::ProviderError;
use crate::providers::base::Provider;
use async_trait::async_trait;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

pub const DEFAULT_MAX_RETRIES: usize = 3;
pub const DEFAULT_INITIAL_RETRY_INTERVAL_MS: u64 = 1000;
pub const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;
pub const DEFAULT_MAX_RETRY_INTERVAL_MS: u64 = 30_000;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub(crate) max_retries: usize,
    /// Initial interval between retries in milliseconds
    pub(crate) initial_interval_ms: u64,
    /// Multiplier for backoff (exponential)
    pub(crate) backoff_multiplier: f64,
    /// Maximum interval between retries in milliseconds
    pub(crate) max_interval_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            initial_interval_ms: DEFAULT_INITIAL_RETRY_INTERVAL_MS,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            max_interval_ms: DEFAULT_MAX_RETRY_INTERVAL_MS,
        }
    }
}

impl RetryConfig {
    pub fn new(
        max_retries: usize,
        initial_interval_ms: u64,
        backoff_multiplier: f64,
        max_interval_ms: u64,
    ) -> Self {
        Self {
            max_retries,
            initial_interval_ms,
            backoff_multiplier,
            max_interval_ms,
        }
    }

    pub fn delay_for_attempt(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }

        let exponent = (attempt - 1) as u32;
        let base_delay_ms = (self.initial_interval_ms as f64
            * self.backoff_multiplier.powi(exponent as i32)) as u64;

        let capped_delay_ms = std::cmp::min(base_delay_ms, self.max_interval_ms);

        let jitter_factor_to_avoid_thundering_herd = 0.8 + (rand::random::<f64>() * 0.4);
        let jitter_delay_ms =
            (capped_delay_ms as f64 * jitter_factor_to_avoid_thundering_herd) as u64;

        Duration::from_millis(jitter_delay_ms)
    }
}

/// Trait for retry functionality to keep Provider dyn-compatible
#[async_trait]
pub trait ProviderRetry {
    fn retry_config(&self) -> RetryConfig {
        RetryConfig::default()
    }

    async fn with_retry<F, Fut, T>(&self, operation: F) -> Result<T, ProviderError>
    where
        F: Fn() -> Fut + Send,
        Fut: Future<Output = Result<T, ProviderError>> + Send,
        T: Send,
    {
        let mut attempts = 0;
        let config = self.retry_config();

        loop {
            return match operation().await {
                Ok(result) => Ok(result),
                Err(error) => {
                    let should_retry = matches!(
                        error,
                        ProviderError::RateLimitExceeded(_) | ProviderError::ServerError(_)
                    );

                    if should_retry && attempts < config.max_retries {
                        attempts += 1;
                        tracing::warn!(
                            "Request failed, retrying ({}/{}): {:?}",
                            attempts,
                            config.max_retries,
                            error
                        );

                        let delay = config.delay_for_attempt(attempts);
                        tracing::info!("Backing off for {:?} before retry", delay);
                        sleep(delay).await;
                        continue;
                    }

                    Err(error)
                }
            };
        }
    }
}

impl<P: Provider> ProviderRetry for P {}
