use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ModelInfo, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::security::model_scanner::{ModelScanResult, ModelScanner};
use rmcp::model::Tool;

/// Configuration for a Gondola BERT model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GondolaConfig {
    /// The model name in Gondola (e.g., "deberta-prompt-injection-v2")
    pub model_name: String,
    /// The model version (e.g., "gmv-zve9abhxe9s7fq1zep5dxd807")
    pub version: String,
    /// The source identifier (e.g., "admin-test")
    pub source: String,
    /// The Gondola endpoint URL
    pub endpoint: String,
}

/// Model registry entry containing all configuration for a specific model
#[derive(Debug, Clone)]
pub struct ModelRegistryEntry {
    pub model_name: String,
    pub version: String,
    pub source: String,
    pub endpoint: String,
}

impl GondolaConfig {
    /// Get the hardcoded model registry
    /// This maps model names to their corresponding configuration
    fn get_model_registry() -> std::collections::HashMap<String, ModelRegistryEntry> {
        let mut registry = std::collections::HashMap::new();

        // DeBERTa Prompt Injection v2 model
        registry.insert(
            "deberta-prompt-injection-v2".to_string(),
            ModelRegistryEntry {
                model_name: "deberta-prompt-injection-v2".to_string(),
                version: "gmv-zve9abhxe9s7fq1zep5dxd807".to_string(),
                source: "admin-test".to_string(),
                endpoint: "https://gondola-ski.stage.sqprod.co".to_string(),
            },
        );

        // Add more models here as they become available
        // registry.insert(
        //     "another-model".to_string(),
        //     ModelRegistryEntry {
        //         model_name: "another-model".to_string(),
        //         version: "gmv-xyz123".to_string(),
        //         source: "production".to_string(),
        //         endpoint: "https://gondola-prod.sqprod.co".to_string(),
        //     },
        // );

        registry
    }

    /// Create configuration from a model name using the registry
    pub fn from_model_name(model_name: &str) -> Result<Self> {
        let registry = Self::get_model_registry();

        let entry = registry.get(model_name).ok_or_else(|| {
            let available_models: Vec<_> = registry.keys().collect();
            anyhow::anyhow!(
                "Unknown Gondola model '{}'. Available models: {:?}",
                model_name,
                available_models
            )
        })?;

        Ok(Self {
            model_name: entry.model_name.clone(),
            version: entry.version.clone(),
            source: entry.source.clone(),
            endpoint: entry.endpoint.clone(),
        })
    }

    /// Get list of available model names from the registry
    pub fn available_models() -> Vec<String> {
        Self::get_model_registry().keys().cloned().collect()
    }
}

impl Default for GondolaConfig {
    fn default() -> Self {
        // Use the first available model as default
        let registry = Self::get_model_registry();
        if let Some((_, entry)) = registry.iter().next() {
            Self {
                model_name: entry.model_name.clone(),
                version: entry.version.clone(),
                source: entry.source.clone(),
                endpoint: entry.endpoint.clone(),
            }
        } else {
            // Fallback if registry is empty (shouldn't happen)
            Self {
                model_name: "deberta-prompt-injection-v2".to_string(),
                version: "gmv-zve9abhxe9s7fq1zep5dxd807".to_string(),
                source: "admin-test".to_string(),
                endpoint: "https://gondola-ski.stage.sqprod.co".to_string(),
            }
        }
    }
}

/// Response from Gondola's BatchInfer endpoint
#[derive(Debug, Deserialize)]
struct GondolaBatchInferResponse {
    model: String,
    version: String,
    occurred_at: String,
    response_items: Vec<GondolaResponseItem>,
}

#[derive(Debug, Deserialize)]
struct GondolaResponseItem {
    double_list_value: DoubleListValue,
}

#[derive(Debug, Deserialize)]
struct DoubleListValue {
    double_values: Vec<f64>,
}

/// Gondola provider for BERT-based prompt injection detection
#[derive(Debug)]
pub struct GondolaProvider {
    api_client: ApiClient,
    config: GondolaConfig,
    model: ModelConfig,
}

impl GondolaProvider {
    /// Create a new GondolaProvider from environment variables
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let global_config = crate::config::Config::global();

        // Check if user specified a model name, otherwise use default
        let model_name = global_config
            .get_param("PROMPT_MODEL_NAME")
            .unwrap_or_else(|_| GondolaConfig::default().model_name);

        // Try to get configuration from the model registry first
        let config = match GondolaConfig::from_model_name(&model_name) {
            Ok(registry_config) => {
                tracing::debug!("🔒 Using Gondola model '{}' from registry", model_name);

                // Allow environment variables to override registry values if needed
                GondolaConfig {
                    model_name: registry_config.model_name,
                    version: global_config
                        .get_param("GONDOLA_MODEL_VERSION")
                        .unwrap_or(registry_config.version),
                    source: global_config
                        .get_param("GONDOLA_SOURCE")
                        .unwrap_or(registry_config.source),
                    endpoint: global_config
                        .get_param("GONDOLA_ENDPOINT")
                        .unwrap_or(registry_config.endpoint),
                }
            }
            Err(e) => {
                tracing::warn!(
                    "🔒 Model '{}' not found in registry: {}. Available models: {:?}",
                    model_name,
                    e,
                    GondolaConfig::available_models()
                );

                // Fallback to manual configuration via environment variables
                GondolaConfig {
                    model_name: model_name.clone(),
                    version: global_config
                        .get_param("GONDOLA_MODEL_VERSION")
                        .map_err(|_| {
                            anyhow::anyhow!(
                                "GONDOLA_MODEL_VERSION is required when using unknown model '{}'",
                                model_name
                            )
                        })?,
                    source: global_config.get_param("GONDOLA_SOURCE").map_err(|_| {
                        anyhow::anyhow!(
                            "GONDOLA_SOURCE is required when using unknown model '{}'",
                            model_name
                        )
                    })?,
                    endpoint: global_config.get_param("GONDOLA_ENDPOINT").map_err(|_| {
                        anyhow::anyhow!(
                            "GONDOLA_ENDPOINT is required when using unknown model '{}'",
                            model_name
                        )
                    })?,
                }
            }
        };

        let timeout_secs: u64 = global_config.get_param("GONDOLA_TIMEOUT").unwrap_or(30);

        // For now, we'll try without explicit authentication, assuming Trogdor handles it
        // Use a placeholder bearer token that will be handled by Trogdor
        let auth = AuthMethod::BearerToken("".to_string());

        let api_client = ApiClient::with_timeout(
            config.endpoint.clone(),
            auth,
            std::time::Duration::from_secs(timeout_secs),
        )?;

        Ok(Self {
            api_client,
            config,
            model,
        })
    }

    /// Create a new GondolaProvider with custom configuration
    pub fn with_config(model: ModelConfig, config: GondolaConfig) -> Result<Self> {
        let auth = AuthMethod::BearerToken("".to_string());
        let api_client = ApiClient::with_timeout(
            config.endpoint.clone(),
            auth,
            std::time::Duration::from_secs(30),
        )?;

        Ok(Self {
            api_client,
            config,
            model,
        })
    }

    /// Scan text for prompt injection using the Gondola BERT model
    pub async fn scan_for_prompt_injection(
        &self,
        text: &str,
    ) -> Result<ModelScanResult, ProviderError> {
        let payload = json!({
            "model": self.config.model_name,
            "version": self.config.version,
            "source": self.config.source,
            "input_names": ["text_input"],
            "request_items": [{
                "inputs": [{
                    "string_value": text
                }]
            }]
        });

        tracing::debug!("🔒 Sending Gondola request to {}/services/squareup.gondola.service.ModelService/BatchInfer", self.config.endpoint);
        tracing::debug!("🔒 Request payload: {}", payload);

        let response = self
            .api_client
            .response_post(
                "services/squareup.gondola.service.ModelService/BatchInfer",
                &payload,
            )
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Gondola request failed: {}", e)))?;

        let status = response.status();
        tracing::debug!("🔒 Gondola response status: {}", status);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("🔒 Gondola API error {}: {}", status, error_text);
            return Err(ProviderError::RequestFailed(format!(
                "Gondola API error {}: {}",
                status, error_text
            )));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to read response: {}", e)))?;

        tracing::debug!(
            "🔒 Gondola raw response (length: {}): {}",
            response_text.len(),
            response_text
        );

        // Check if response is empty or whitespace
        if response_text.trim().is_empty() {
            return Err(ProviderError::RequestFailed(
                "Empty response from Gondola".to_string(),
            ));
        }

        let gondola_response: GondolaBatchInferResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                tracing::error!("🔒 Failed to parse Gondola response. Error: {}", e);
                tracing::error!("🔒 Response text was: '{}'", response_text);
                ProviderError::RequestFailed(format!("Failed to parse Gondola response: {}", e))
            })?;

        // Validate we got the expected model and version back
        if gondola_response.model != self.config.model_name {
            tracing::warn!(
                "🔒 Expected model '{}' but got '{}'",
                self.config.model_name,
                gondola_response.model
            );
        }
        if gondola_response.version != self.config.version {
            tracing::warn!(
                "🔒 Expected version '{}' but got '{}'",
                self.config.version,
                gondola_response.version
            );
        }

        tracing::debug!(
            "🔒 Gondola response validated: model={}, version={}, occurred_at={}",
            gondola_response.model,
            gondola_response.version,
            gondola_response.occurred_at
        );

        if gondola_response.response_items.is_empty() {
            return Err(ProviderError::RequestFailed(
                "No response items from Gondola".to_string(),
            ));
        }

        let scores = gondola_response.response_items[0]
            .double_list_value
            .double_values
            .clone();

        tracing::debug!("🔒 Extracted scores from Gondola: {:?}", scores);

        // Create metadata for the result
        let metadata = serde_json::json!({
            "model": self.config.model_name,
            "version": self.config.version,
            "source": self.config.source,
            "endpoint": self.config.endpoint
        });

        // Use ModelScanResult's binary logits conversion with metadata
        Ok(ModelScanResult::from_binary_logits(scores[0], scores[1])
            .with_metadata_builder(metadata))
    }

    /// Check if the Gondola service is available
    pub async fn is_available(&self) -> bool {
        // Simple health check - try to make a minimal request
        let test_payload = json!({
            "model": self.config.model_name,
            "version": self.config.version,
            "source": self.config.source,
            "input_names": ["text_input"],
            "request_items": [{
                "inputs": [{
                    "string_value": "test"
                }]
            }]
        });

        match self
            .api_client
            .response_post(
                "services/squareup.gondola.service.ModelService/BatchInfer",
                &test_payload,
            )
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl ModelScanner for GondolaProvider {
    async fn scan_text(&self, text: &str) -> Result<ModelScanResult, ProviderError> {
        // Delegate directly to scan_for_prompt_injection which already returns ModelScanResult
        self.scan_for_prompt_injection(text).await
    }

    async fn is_available(&self) -> bool {
        self.is_available().await
    }

    fn model_name(&self) -> &str {
        &self.config.model_name
    }

    fn model_version(&self) -> Option<&str> {
        Some(&self.config.version)
    }
}

#[async_trait]
impl Provider for GondolaProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::with_models(
            "gondola",
            "Gondola",
            "Internal Gondola service for BERT-based security scanning",
            "deberta-prompt-injection-v2",
            vec![ModelInfo::new("deberta-prompt-injection-v2", 512)], // BERT models typically have 512 token limit
            "https://gondola-internal-docs", // Placeholder for internal docs
            vec![
                ConfigKey::new(
                    "PROMPT_MODEL_NAME",
                    false,
                    false,
                    Some("deberta-prompt-injection-v2"),
                ),
                ConfigKey::new(
                    "GONDOLA_ENDPOINT",
                    false,
                    false,
                    Some("https://gondola-ski.stage.sqprod.co"),
                ),
                ConfigKey::new(
                    "GONDOLA_MODEL_VERSION",
                    false,
                    false,
                    Some("gmv-zve9abhxe9s7fq1zep5dxd807"),
                ),
                ConfigKey::new("GONDOLA_SOURCE", false, false, Some("admin-test")),
                ConfigKey::new("GONDOLA_TIMEOUT", false, false, Some("30")),
            ],
        )
    }

    fn get_name(&self) -> &str {
        "gondola"
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn complete_with_model(
        &self,
        _model_config: &ModelConfig,
        _system: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Gondola is not a chat completion provider - it's specialized for security scanning
        Err(ProviderError::NotImplemented(
            "GondolaProvider is specialized for security scanning, not chat completion".to_string(),
        ))
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        // Return all models available in the registry
        Ok(Some(GondolaConfig::available_models()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_scan_result_from_binary_logits() {
        // Test case where injection is detected (injection_logit > safe_logit)
        let result = ModelScanResult::from_binary_logits(2.0, 5.0);

        assert!(result.is_injection);
        // With softmax: safe_prob = exp(2)/(exp(2)+exp(5)) ≈ 0.047, injection_prob ≈ 0.953
        assert!(result.confidence > 0.9);
        assert_eq!(result.raw_scores, vec![2.0, 5.0]);

        // Test case where no injection is detected (safe_logit > injection_logit)
        let result = ModelScanResult::from_binary_logits(5.0, 2.0);

        assert!(!result.is_injection);
        // With softmax: safe_prob ≈ 0.953, injection_prob ≈ 0.047
        assert!(result.confidence > 0.9);
        assert_eq!(result.raw_scores, vec![5.0, 2.0]);

        // Test case with equal logits (uncertain)
        let result = ModelScanResult::from_binary_logits(3.0, 3.0);

        // With equal logits, softmax gives 0.5 probability for each class
        assert!((result.confidence - 0.5).abs() < 0.001);
        assert_eq!(result.raw_scores, vec![3.0, 3.0]);

        // Test with actual example from Gondola
        let result = ModelScanResult::from_binary_logits(5.977328300476074, -6.504494667053223);

        // Safe logit is much higher than injection logit, so should be classified as safe
        assert!(!result.is_injection);
        // Should have very high confidence in the safe classification
        assert!(result.confidence > 0.99);
    }

    #[test]
    fn test_gondola_config_default() {
        let config = GondolaConfig::default();
        assert_eq!(config.model_name, "deberta-prompt-injection-v2");
        assert_eq!(config.version, "gmv-zve9abhxe9s7fq1zep5dxd807");
        assert_eq!(config.source, "admin-test");
        assert_eq!(config.endpoint, "https://gondola-ski.stage.sqprod.co");
    }

    #[test]
    fn test_model_registry() {
        // Test that we can get available models
        let models = GondolaConfig::available_models();
        assert!(!models.is_empty());
        assert!(models.contains(&"deberta-prompt-injection-v2".to_string()));
    }

    #[test]
    fn test_config_from_model_name() {
        // Test with known model
        let config = GondolaConfig::from_model_name("deberta-prompt-injection-v2").unwrap();
        assert_eq!(config.model_name, "deberta-prompt-injection-v2");
        assert_eq!(config.version, "gmv-zve9abhxe9s7fq1zep5dxd807");
        assert_eq!(config.source, "admin-test");
        assert_eq!(config.endpoint, "https://gondola-ski.stage.sqprod.co");

        // Test with unknown model
        let result = GondolaConfig::from_model_name("unknown-model");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown Gondola model"));
    }
}
