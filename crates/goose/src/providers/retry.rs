use crate::providers::base::Provider;
use async_trait::async_trait;
use goose_providers::errors::ProviderError;
use std::env;
use std::fmt::Display;
use std::future::Future;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

pub const DEFAULT_MAX_RETRIES: usize = 3;
pub const DEFAULT_INITIAL_RETRY_INTERVAL_MS: u64 = 1000;
pub const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;
pub const DEFAULT_MAX_RETRY_INTERVAL_MS: u64 = 30_000;

const GOOSE_LLM_MAX_RETRIES: &str = "GOOSE_LLM_MAX_RETRIES";
const GOOSE_LLM_INITIAL_RETRY_INTERVAL_MS: &str = "GOOSE_LLM_INITIAL_RETRY_INTERVAL_MS";
const GOOSE_LLM_BACKOFF_MULTIPLIER: &str = "GOOSE_LLM_BACKOFF_MULTIPLIER";
const GOOSE_LLM_MAX_RETRY_INTERVAL_MS: &str = "GOOSE_LLM_MAX_RETRY_INTERVAL_MS";

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
    /// When true, only retry on transient errors (ServerError, NetworkError,
    /// RateLimitExceeded). RequestFailed (4xx client errors) will not be retried.
    pub(crate) transient_only: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl RetryConfig {
    pub fn from_env() -> Self {
        Self::with_defaults(
            DEFAULT_MAX_RETRIES,
            DEFAULT_INITIAL_RETRY_INTERVAL_MS,
            DEFAULT_BACKOFF_MULTIPLIER,
            DEFAULT_MAX_RETRY_INTERVAL_MS,
        )
    }

    /// Builds a retry config from the universal `GOOSE_LLM_*` environment
    /// variables, falling back to the supplied provider-specific defaults when
    /// a variable is unset or invalid.
    pub fn with_defaults(
        max_retries: usize,
        initial_interval_ms: u64,
        backoff_multiplier: f64,
        max_interval_ms: u64,
    ) -> Self {
        Self {
            max_retries: env_max_retries().unwrap_or(max_retries),
            initial_interval_ms: env_initial_interval_ms().unwrap_or(initial_interval_ms),
            backoff_multiplier: env_backoff_multiplier().unwrap_or(backoff_multiplier),
            max_interval_ms: env_max_interval_ms().unwrap_or(max_interval_ms),
            transient_only: false,
        }
    }

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
            transient_only: false,
        }
    }

    pub fn transient_only(mut self) -> Self {
        self.transient_only = true;
        self
    }

    pub fn max_retries(&self) -> usize {
        self.max_retries
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

/// Reads the universal `GOOSE_LLM_MAX_RETRIES` override, if set and valid.
pub fn env_max_retries() -> Option<usize> {
    read_retry_env(
        GOOSE_LLM_MAX_RETRIES,
        |value| *value > 0,
        "a positive integer",
    )
}

/// Reads the universal `GOOSE_LLM_INITIAL_RETRY_INTERVAL_MS` override, if set and valid.
pub fn env_initial_interval_ms() -> Option<u64> {
    read_retry_env(
        GOOSE_LLM_INITIAL_RETRY_INTERVAL_MS,
        |value| *value > 0,
        "a positive integer",
    )
}

/// Reads the universal `GOOSE_LLM_BACKOFF_MULTIPLIER` override, if set and valid.
pub fn env_backoff_multiplier() -> Option<f64> {
    read_retry_env(
        GOOSE_LLM_BACKOFF_MULTIPLIER,
        |value: &f64| value.is_finite() && *value >= 1.0,
        "a finite number greater than or equal to 1",
    )
}

/// Reads the universal `GOOSE_LLM_MAX_RETRY_INTERVAL_MS` override, if set and valid.
pub fn env_max_interval_ms() -> Option<u64> {
    read_retry_env(
        GOOSE_LLM_MAX_RETRY_INTERVAL_MS,
        |value| *value > 0,
        "a positive integer",
    )
}

fn read_retry_env<T, F>(name: &str, is_valid: F, expected: &str) -> Option<T>
where
    T: Display + FromStr,
    F: Fn(&T) -> bool,
{
    let value = match env::var(name) {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => return None,
        Err(error) => {
            tracing::warn!(
                env_var = name,
                error = %error,
                "Ignoring invalid retry environment variable"
            );
            return None;
        }
    };

    match value.parse::<T>() {
        Ok(parsed) if is_valid(&parsed) => Some(parsed),
        _ => {
            tracing::warn!(
                env_var = name,
                value = %value,
                expected,
                "Ignoring invalid retry environment variable"
            );
            None
        }
    }
}

pub fn should_retry(error: &ProviderError, config: &RetryConfig) -> bool {
    match error {
        ProviderError::RateLimitExceeded { .. }
        | ProviderError::ServerError(_)
        | ProviderError::NetworkError(_) => true,
        ProviderError::RequestFailed(_) => !config.transient_only,
        _ => false,
    }
}

pub async fn retry_operation<F, Fut, T>(
    config: &RetryConfig,
    operation: F,
) -> Result<T, ProviderError>
where
    F: Fn() -> Fut + Send,
    Fut: Future<Output = Result<T, ProviderError>> + Send,
    T: Send,
{
    let mut attempts = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if should_retry(&error, config) && attempts < config.max_retries {
                    attempts += 1;
                    tracing::warn!(
                        "Request failed, retrying ({}/{}): {:?}",
                        attempts,
                        config.max_retries,
                        error
                    );

                    let delay = match &error {
                        ProviderError::RateLimitExceeded {
                            retry_delay: Some(d),
                            ..
                        } => *d,
                        _ => config.delay_for_attempt(attempts),
                    };

                    sleep(delay).await;
                    continue;
                }
                return Err(error);
            }
        }
    }
}

/// Trait for retry functionality to keep Provider dyn-compatible.
///
/// All `Provider` implementors get this via the blanket impl below.
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
        self.with_retry_config(operation, self.retry_config()).await
    }

    async fn with_retry_config<F, Fut, T>(
        &self,
        operation: F,
        config: RetryConfig,
    ) -> Result<T, ProviderError>
    where
        F: Fn() -> Fut + Send,
        Fut: Future<Output = Result<T, ProviderError>> + Send,
        T: Send;
}

#[async_trait]
impl<P: Provider> ProviderRetry for P {
    fn retry_config(&self) -> RetryConfig {
        Provider::retry_config(self)
    }

    async fn with_retry_config<F, Fut, T>(
        &self,
        operation: F,
        config: RetryConfig,
    ) -> Result<T, ProviderError>
    where
        F: Fn() -> Fut + Send,
        Fut: Future<Output = Result<T, ProviderError>> + Send,
        T: Send,
    {
        let mut attempts = 0;
        let mut auth_retried = false;

        loop {
            return match operation().await {
                Ok(result) => Ok(result),
                Err(error) => {
                    // Auth retry is separate from transient-error retries: we get
                    // at most 1 credential refresh, independent of max_retries.
                    if matches!(error, ProviderError::Authentication(_)) && !auth_retried {
                        auth_retried = true;
                        match self.refresh_credentials().await {
                            Ok(()) => {
                                tracing::warn!(
                                    "Credentials refreshed after auth error, retrying: {:?}",
                                    error
                                );
                                continue;
                            }
                            Err(refresh_err) => {
                                tracing::warn!(
                                    "Credential refresh failed, returning original auth error: {:?}",
                                    refresh_err
                                );
                            }
                        }
                    }

                    if should_retry(&error, &config) && attempts < config.max_retries {
                        attempts += 1;
                        tracing::warn!(
                            "Request failed, retrying ({}/{}): {:?}",
                            attempts,
                            config.max_retries,
                            error
                        );

                        let delay = match &error {
                            ProviderError::RateLimitExceeded {
                                retry_delay: Some(provider_delay),
                                ..
                            } => *provider_delay,
                            _ => config.delay_for_attempt(attempts),
                        };

                        let skip_backoff = std::env::var("GOOSE_PROVIDER_SKIP_BACKOFF")
                            .unwrap_or_default()
                            .parse::<bool>()
                            .unwrap_or(false);

                        if skip_backoff {
                            tracing::info!("Skipping backoff due to GOOSE_PROVIDER_SKIP_BACKOFF");
                        } else {
                            tracing::info!("Backing off for {:?} before retry", delay);
                            sleep(delay).await;
                        }
                        continue;
                    }

                    Err(error)
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_retry_env(
        max_retries: Option<&'static str>,
        initial_interval_ms: Option<&'static str>,
        backoff_multiplier: Option<&'static str>,
        max_interval_ms: Option<&'static str>,
    ) -> env_lock::EnvGuard<'static> {
        env_lock::lock_env([
            (GOOSE_LLM_MAX_RETRIES, max_retries),
            (GOOSE_LLM_INITIAL_RETRY_INTERVAL_MS, initial_interval_ms),
            (GOOSE_LLM_BACKOFF_MULTIPLIER, backoff_multiplier),
            (GOOSE_LLM_MAX_RETRY_INTERVAL_MS, max_interval_ms),
        ])
    }

    #[test]
    fn default_config_uses_hardcoded_defaults_without_env() {
        let _guard = lock_retry_env(None, None, None, None);
        let config = RetryConfig::default();

        assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
        assert_eq!(
            config.initial_interval_ms,
            DEFAULT_INITIAL_RETRY_INTERVAL_MS
        );
        assert_eq!(config.backoff_multiplier, DEFAULT_BACKOFF_MULTIPLIER);
        assert_eq!(config.max_interval_ms, DEFAULT_MAX_RETRY_INTERVAL_MS);
        assert!(!config.transient_only);
    }

    #[test]
    fn default_config_uses_llm_retry_env_overrides() {
        let _guard = lock_retry_env(Some("8"), Some("250"), Some("1.5"), Some("60000"));
        let config = RetryConfig::default();

        assert_eq!(config.max_retries, 8);
        assert_eq!(config.initial_interval_ms, 250);
        assert_eq!(config.backoff_multiplier, 1.5);
        assert_eq!(config.max_interval_ms, 60_000);
    }

    #[test]
    fn default_config_ignores_invalid_llm_retry_env_overrides() {
        let _guard = lock_retry_env(Some("0"), Some("not-a-number"), Some("NaN"), Some("0"));
        let config = RetryConfig::default();

        assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
        assert_eq!(
            config.initial_interval_ms,
            DEFAULT_INITIAL_RETRY_INTERVAL_MS
        );
        assert_eq!(config.backoff_multiplier, DEFAULT_BACKOFF_MULTIPLIER);
        assert_eq!(config.max_interval_ms, DEFAULT_MAX_RETRY_INTERVAL_MS);
    }

    #[test]
    fn with_defaults_keeps_provider_defaults_without_env() {
        let _guard = lock_retry_env(None, None, None, None);
        let config = RetryConfig::with_defaults(6, 2000, 2.5, 120_000);

        assert_eq!(config.max_retries, 6);
        assert_eq!(config.initial_interval_ms, 2000);
        assert_eq!(config.backoff_multiplier, 2.5);
        assert_eq!(config.max_interval_ms, 120_000);
    }

    #[test]
    fn with_defaults_prefers_llm_retry_env_overrides() {
        let _guard = lock_retry_env(Some("8"), Some("250"), Some("1.5"), Some("60000"));
        let config = RetryConfig::with_defaults(6, 2000, 2.5, 120_000);

        assert_eq!(config.max_retries, 8);
        assert_eq!(config.initial_interval_ms, 250);
        assert_eq!(config.backoff_multiplier, 1.5);
        assert_eq!(config.max_interval_ms, 60_000);
    }

    #[test]
    fn explicit_config_ignores_llm_retry_env_overrides() {
        let _guard = lock_retry_env(Some("8"), Some("250"), Some("1.5"), Some("60000"));
        let config = RetryConfig::new(1, 2, 3.0, 4);

        assert_eq!(config.max_retries, 1);
        assert_eq!(config.initial_interval_ms, 2);
        assert_eq!(config.backoff_multiplier, 3.0);
        assert_eq!(config.max_interval_ms, 4);
    }

    #[test]
    fn default_config_retries_request_failed() {
        let config = RetryConfig::default();
        let error = ProviderError::RequestFailed("Bad request (400): model not found".into());
        assert!(should_retry(&error, &config));
    }

    #[test]
    fn transient_only_skips_request_failed() {
        let config = RetryConfig::default().transient_only();
        let error = ProviderError::RequestFailed("Bad request (400): model not found".into());
        assert!(!should_retry(&error, &config));
    }

    #[test]
    fn transient_only_still_retries_server_error() {
        let config = RetryConfig::default().transient_only();
        assert!(should_retry(
            &ProviderError::ServerError("500 internal".into()),
            &config
        ));
    }

    #[test]
    fn transient_only_still_retries_network_error() {
        let config = RetryConfig::default().transient_only();
        assert!(should_retry(
            &ProviderError::NetworkError("connection refused".into()),
            &config
        ));
    }

    #[test]
    fn transient_only_still_retries_rate_limit() {
        let config = RetryConfig::default().transient_only();
        assert!(should_retry(
            &ProviderError::RateLimitExceeded {
                details: "too many requests".into(),
                retry_delay: None,
            },
            &config
        ));
    }

    #[test]
    fn never_retries_auth_errors() {
        let config = RetryConfig::default();
        assert!(!should_retry(
            &ProviderError::Authentication("invalid key".into()),
            &config
        ));
    }
}
