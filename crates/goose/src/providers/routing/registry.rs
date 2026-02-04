//! Provider registry for managing endpoints and health checks

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use super::{ErrorCategory, ProviderCapabilities, RoutingError, RoutingResult};

/// Unique identifier for an endpoint
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EndpointId(String);

impl EndpointId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for EndpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for EndpointId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for EndpointId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Authentication configuration for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthConfig {
    /// No authentication required
    None,
    /// Environment variable containing API key
    Environment { name: String },
    /// File containing API key
    File { path: String },
    /// OAuth2 configuration
    OAuth2 {
        client_id: String,
        token_url: String,
        scopes: Vec<String>,
    },
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Whether to verify TLS certificates
    pub verify: bool,
    /// Custom CA certificate path
    pub ca_cert_path: Option<String>,
    /// Client certificate path
    pub client_cert_path: Option<String>,
    /// Client key path
    pub client_key_path: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            verify: true,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
        }
    }
}

/// Configuration for a provider endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// Unique endpoint identifier
    pub endpoint_id: EndpointId,
    /// Provider type (anthropic, openai_compat, etc.)
    pub provider: String,
    /// Base URL for the endpoint
    pub base_url: String,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Default HTTP headers
    pub default_headers: HashMap<String, String>,
    /// TLS configuration
    pub tls: TlsConfig,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// Models available at this endpoint
    pub available_models: Option<Vec<String>>,
}

impl EndpointConfig {
    pub fn new(
        endpoint_id: impl Into<EndpointId>,
        provider: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            endpoint_id: endpoint_id.into(),
            provider: provider.into(),
            base_url: base_url.into(),
            auth: AuthConfig::None,
            default_headers: HashMap::new(),
            tls: TlsConfig::default(),
            timeout_seconds: 30,
            max_retries: 3,
            available_models: None,
        }
    }

    pub fn with_env_auth(mut self, env_var: impl Into<String>) -> Self {
        self.auth = AuthConfig::Environment {
            name: env_var.into(),
        };
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(key.into(), value.into());
        self
    }

    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.available_models = Some(models);
        self
    }
}

/// Health status of an endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointHealth {
    /// Endpoint is healthy and responding
    Healthy,
    /// Endpoint is degraded (slow responses, some errors)
    Degraded,
    /// Endpoint is unhealthy (not responding, authentication failed)
    Unhealthy,
    /// Health status unknown (not checked yet)
    Unknown,
}

impl Default for EndpointHealth {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Health check result for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Health status
    pub status: EndpointHealth,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Last check timestamp
    pub checked_at: SystemTime,
    /// Error message if unhealthy
    pub error_message: Option<String>,
    /// Available models (if discoverable)
    pub available_models: Option<Vec<String>>,
    /// Detected capabilities
    pub capabilities: Option<ProviderCapabilities>,
}

impl HealthCheckResult {
    pub fn healthy(response_time_ms: u64) -> Self {
        Self {
            status: EndpointHealth::Healthy,
            response_time_ms: Some(response_time_ms),
            checked_at: SystemTime::now(),
            error_message: None,
            available_models: None,
            capabilities: None,
        }
    }

    pub fn unhealthy(error: impl Into<String>) -> Self {
        Self {
            status: EndpointHealth::Unhealthy,
            response_time_ms: None,
            checked_at: SystemTime::now(),
            error_message: Some(error.into()),
            available_models: None,
            capabilities: None,
        }
    }
}

/// Registry for managing provider endpoints
#[derive(Debug)]
pub struct ProviderRegistry {
    /// Registered endpoints
    endpoints: HashMap<EndpointId, EndpointConfig>,
    /// Health check results
    health_cache: HashMap<EndpointId, HealthCheckResult>,
    /// Health check interval
    health_check_interval: Duration,
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
            health_cache: HashMap::new(),
            health_check_interval: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Register an endpoint
    pub fn register_endpoint(&mut self, config: EndpointConfig) -> RoutingResult<()> {
        let endpoint_id = config.endpoint_id.clone();

        // Validate configuration
        if config.base_url.is_empty() {
            return Err(RoutingError::config_error("Base URL cannot be empty"));
        }

        self.endpoints.insert(endpoint_id, config);
        Ok(())
    }

    /// Remove an endpoint
    pub fn remove_endpoint(&mut self, endpoint_id: &EndpointId) -> RoutingResult<()> {
        if self.endpoints.remove(endpoint_id).is_none() {
            return Err(RoutingError::endpoint_not_found(endpoint_id.to_string()));
        }
        self.health_cache.remove(endpoint_id);
        Ok(())
    }

    /// Get an endpoint configuration
    pub fn get_endpoint(&self, endpoint_id: &EndpointId) -> RoutingResult<&EndpointConfig> {
        self.endpoints
            .get(endpoint_id)
            .ok_or_else(|| RoutingError::endpoint_not_found(endpoint_id.to_string()))
    }

    /// List all endpoints
    pub fn list_endpoints(&self) -> Vec<&EndpointConfig> {
        self.endpoints.values().collect()
    }

    /// List endpoints for a specific provider
    pub fn list_endpoints_for_provider(&self, provider: &str) -> Vec<&EndpointConfig> {
        self.endpoints
            .values()
            .filter(|config| config.provider == provider)
            .collect()
    }

    /// Get health status for an endpoint
    pub fn get_health(&self, endpoint_id: &EndpointId) -> EndpointHealth {
        self.health_cache
            .get(endpoint_id)
            .map(|result| result.status)
            .unwrap_or(EndpointHealth::Unknown)
    }

    /// Update health status for an endpoint
    pub fn update_health(&mut self, endpoint_id: &EndpointId, result: HealthCheckResult) {
        self.health_cache.insert(endpoint_id.clone(), result);
    }

    /// Get healthy endpoints for a provider
    pub fn get_healthy_endpoints(&self, provider: &str) -> Vec<&EndpointConfig> {
        self.endpoints
            .values()
            .filter(|config| {
                config.provider == provider
                    && matches!(
                        self.get_health(&config.endpoint_id),
                        EndpointHealth::Healthy | EndpointHealth::Unknown
                    )
            })
            .collect()
    }

    /// Check if any endpoint for a provider is healthy
    pub fn has_healthy_provider(&self, provider: &str) -> bool {
        !self.get_healthy_endpoints(provider).is_empty()
    }

    /// Get the best endpoint for a provider (lowest response time)
    pub fn get_best_endpoint(&self, provider: &str) -> RoutingResult<&EndpointConfig> {
        let mut candidates: Vec<_> = self
            .get_healthy_endpoints(provider)
            .into_iter()
            .map(|config| {
                let response_time = self
                    .health_cache
                    .get(&config.endpoint_id)
                    .and_then(|result| result.response_time_ms)
                    .unwrap_or(u64::MAX);
                (config, response_time)
            })
            .collect();

        if candidates.is_empty() {
            return Err(RoutingError::provider_not_found(provider));
        }

        // Sort by response time (ascending)
        candidates.sort_by_key(|(_, response_time)| *response_time);

        Ok(candidates[0].0)
    }

    /// Classify an error into a category
    pub fn classify_error(&self, error: &anyhow::Error) -> ErrorCategory {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("unauthorized") || error_str.contains("forbidden") {
            ErrorCategory::AuthError
        } else if error_str.contains("quota")
            || error_str.contains("limit")
            || error_str.contains("credits")
        {
            ErrorCategory::QuotaExhausted
        } else if error_str.contains("rate limit") || error_str.contains("too many requests") {
            ErrorCategory::RateLimited
        } else if error_str.contains("timeout") || error_str.contains("timed out") {
            ErrorCategory::Timeout
        } else if error_str.contains("connection") || error_str.contains("network") {
            ErrorCategory::EndpointUnreachable
        } else if error_str.contains("model not found") || error_str.contains("invalid model") {
            ErrorCategory::ModelNotFound
        } else if error_str.contains("server error") || error_str.contains("internal error") {
            ErrorCategory::ServerError
        } else {
            ErrorCategory::Unknown
        }
    }

    /// Load configuration from file
    pub async fn load_from_file(&mut self, path: &std::path::Path) -> RoutingResult<()> {
        let content = tokio::fs::read_to_string(path).await?;
        let configs: Vec<EndpointConfig> = serde_json::from_str(&content)?;

        for config in configs {
            self.register_endpoint(config)?;
        }

        Ok(())
    }

    /// Save configuration to file
    pub async fn save_to_file(&self, path: &std::path::Path) -> RoutingResult<()> {
        let configs: Vec<_> = self.endpoints.values().collect();
        let content = serde_json::to_string_pretty(&configs)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_registration() {
        let mut registry = ProviderRegistry::new();

        let config = EndpointConfig::new(
            "anthropic_primary",
            "anthropic",
            "https://api.anthropic.com",
        )
        .with_env_auth("ANTHROPIC_API_KEY")
        .with_header("anthropic-version", "2023-06-01");

        registry.register_endpoint(config).unwrap();

        let retrieved = registry
            .get_endpoint(&EndpointId::new("anthropic_primary"))
            .unwrap();
        assert_eq!(retrieved.provider, "anthropic");
        assert_eq!(retrieved.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn test_health_tracking() {
        let mut registry = ProviderRegistry::new();
        let endpoint_id = EndpointId::new("test_endpoint");

        // Initially unknown
        assert_eq!(registry.get_health(&endpoint_id), EndpointHealth::Unknown);

        // Update to healthy
        let health_result = HealthCheckResult::healthy(100);
        registry.update_health(&endpoint_id, health_result);
        assert_eq!(registry.get_health(&endpoint_id), EndpointHealth::Healthy);
    }

    #[test]
    fn test_error_classification() {
        let registry = ProviderRegistry::new();

        let quota_error = anyhow::anyhow!("Quota exceeded for this request");
        assert_eq!(
            registry.classify_error(&quota_error),
            ErrorCategory::QuotaExhausted
        );

        let auth_error = anyhow::anyhow!("Unauthorized access");
        assert_eq!(
            registry.classify_error(&auth_error),
            ErrorCategory::AuthError
        );
    }
}
