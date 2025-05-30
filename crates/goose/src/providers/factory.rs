use serde::Deserialize;
use std::sync::Arc;

use super::{
    anthropic::AnthropicProvider,
    azure::AzureProvider,
    base::{Provider, ProviderMetadata},
    bedrock::BedrockProvider,
    databricks::DatabricksProvider,
    gcpvertexai::GcpVertexAIProvider,
    githubcopilot::GithubCopilotProvider,
    google::GoogleProvider,
    groq::GroqProvider,
    lead_worker::LeadWorkerProvider,
    ollama::OllamaProvider,
    openai::OpenAiProvider,
    openrouter::OpenRouterProvider,
    venice::VeniceProvider,
};
use crate::model::ModelConfig;
use anyhow::Result;

#[cfg(test)]
use super::errors::ProviderError;
#[cfg(test)]
use mcp_core::tool::Tool;

/// Configuration for lead/worker provider setup
#[derive(Debug, Clone, Deserialize)]
pub struct LeadWorkerConfig {
    /// Whether lead/worker mode is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Lead provider configuration
    pub lead_provider: Option<String>,
    /// Lead model name
    pub lead_model: Option<String>,
    /// Worker provider configuration (optional, defaults to main provider)
    pub worker_provider: Option<String>,
    /// Worker model name (optional, defaults to main model)
    pub worker_model: Option<String>,
    /// Number of turns to use lead model (default: 3)
    #[serde(default = "default_lead_turns")]
    pub lead_turns: usize,
    /// Number of consecutive failures before fallback (default: 2)
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: usize,
    /// Number of turns to use lead model in fallback mode (default: 2)
    #[serde(default = "default_fallback_turns")]
    pub fallback_turns: usize,
}

fn default_lead_turns() -> usize {
    3
}
fn default_failure_threshold() -> usize {
    2
}
fn default_fallback_turns() -> usize {
    2
}

pub fn providers() -> Vec<ProviderMetadata> {
    vec![
        AnthropicProvider::metadata(),
        AzureProvider::metadata(),
        BedrockProvider::metadata(),
        DatabricksProvider::metadata(),
        GcpVertexAIProvider::metadata(),
        GithubCopilotProvider::metadata(),
        GoogleProvider::metadata(),
        GroqProvider::metadata(),
        OllamaProvider::metadata(),
        OpenAiProvider::metadata(),
        OpenRouterProvider::metadata(),
        VeniceProvider::metadata(),
    ]
}

pub fn create(name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
    let config = crate::config::Config::global();

    // PRECEDENCE ORDER (highest to lowest):
    // 1. Environment variables (GOOSE_LEAD_MODEL)
    // 2. YAML lead_worker config section
    // 3. Regular provider (no lead/worker)

    // Check for environment variable first (highest precedence)
    if let Ok(lead_model_name) = config.get_param::<String>("GOOSE_LEAD_MODEL") {
        tracing::info!("Creating lead/worker provider from environment variable");

        // Worker model is always the main configured model
        let worker_model_config = model.clone();
        let lead_turns = 3; // Fixed for env var approach

        // Create lead and worker providers (same provider type)
        let lead_model_config = crate::model::ModelConfig::new(lead_model_name);
        let lead_provider = create_provider(name, lead_model_config)?;
        let worker_provider = create_provider(name, worker_model_config)?;

        return Ok(Arc::new(LeadWorkerProvider::new(
            lead_provider,
            worker_provider,
            Some(lead_turns),
        )));
    }

    // Check for YAML lead_worker config (second precedence)
    if let Ok(lead_worker_config) = config.get_param::<LeadWorkerConfig>("lead_worker") {
        if lead_worker_config.enabled {
            tracing::info!("Creating lead/worker provider from YAML configuration");

            return create_lead_worker_from_config(name, &model, &lead_worker_config);
        }
    }

    // Default: create regular provider (lowest precedence)
    create_provider(name, model)
}

/// Create a lead/worker provider from YAML configuration
fn create_lead_worker_from_config(
    default_provider_name: &str,
    default_model: &ModelConfig,
    config: &LeadWorkerConfig,
) -> Result<Arc<dyn Provider>> {
    // Determine lead provider and model
    let lead_provider_name = config
        .lead_provider
        .as_deref()
        .unwrap_or(default_provider_name);
    let lead_model_name = config
        .lead_model
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("lead_model is required when lead_worker is enabled"))?;
    let lead_model_config = ModelConfig::new(lead_model_name.to_string());

    // Determine worker provider and model
    let worker_provider_name = config
        .worker_provider
        .as_deref()
        .unwrap_or(default_provider_name);
    let worker_model_config = if let Some(worker_model_name) = &config.worker_model {
        ModelConfig::new(worker_model_name.clone())
    } else {
        default_model.clone()
    };

    // Create the providers
    let lead_provider = create_provider(lead_provider_name, lead_model_config)?;
    let worker_provider = create_provider(worker_provider_name, worker_model_config)?;

    // Create the lead/worker provider with configured settings
    Ok(Arc::new(LeadWorkerProvider::new_with_settings(
        lead_provider,
        worker_provider,
        config.lead_turns,
        config.failure_threshold,
        config.fallback_turns,
    )))
}

fn create_provider(name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
    // We use Arc instead of Box to be able to clone for multiple async tasks
    match name {
        "openai" => Ok(Arc::new(OpenAiProvider::from_env(model)?)),
        "anthropic" => Ok(Arc::new(AnthropicProvider::from_env(model)?)),
        "azure_openai" => Ok(Arc::new(AzureProvider::from_env(model)?)),
        "aws_bedrock" => Ok(Arc::new(BedrockProvider::from_env(model)?)),
        "databricks" => Ok(Arc::new(DatabricksProvider::from_env(model)?)),
        "groq" => Ok(Arc::new(GroqProvider::from_env(model)?)),
        "ollama" => Ok(Arc::new(OllamaProvider::from_env(model)?)),
        "openrouter" => Ok(Arc::new(OpenRouterProvider::from_env(model)?)),
        "gcp_vertex_ai" => Ok(Arc::new(GcpVertexAIProvider::from_env(model)?)),
        "google" => Ok(Arc::new(GoogleProvider::from_env(model)?)),
        "venice" => Ok(Arc::new(VeniceProvider::from_env(model)?)),
        "github_copilot" => Ok(Arc::new(GithubCopilotProvider::from_env(model)?)),
        _ => Err(anyhow::anyhow!("Unknown provider: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Message, MessageContent};
    use crate::providers::base::{ProviderMetadata, ProviderUsage, Usage};
    use chrono::Utc;
    use mcp_core::{content::TextContent, Role};
    use std::env;

    #[derive(Clone)]
    struct MockTestProvider {
        name: String,
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockTestProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::new(
                "mock_test",
                "Mock Test Provider",
                "A mock provider for testing",
                "mock-model",
                vec!["mock-model"],
                "",
                vec![],
            )
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
                        text: format!(
                            "Response from {} with model {}",
                            self.name, self.model_config.model_name
                        ),
                        annotations: None,
                    })],
                },
                ProviderUsage::new(self.model_config.model_name.clone(), Usage::default()),
            ))
        }
    }

    #[test]
    fn test_create_lead_worker_provider() {
        // Save current env var
        let saved_lead = env::var("GOOSE_LEAD_MODEL").ok();

        // Test with lead model configuration
        env::set_var("GOOSE_LEAD_MODEL", "gpt-4o");

        // This will try to create a lead/worker provider
        let result = create("openai", ModelConfig::new("gpt-4o-mini".to_string()));

        // The creation might succeed or fail depending on API keys, but we can verify the logic path
        match result {
            Ok(_) => {
                // If it succeeds, it means we created a lead/worker provider successfully
                // This would happen if API keys are available in the test environment
            }
            Err(error) => {
                // If it fails, it should be due to missing API keys, confirming we tried to create providers
                let error_msg = error.to_string();
                assert!(error_msg.contains("OPENAI_API_KEY") || error_msg.contains("secret"));
            }
        }

        // Restore env var
        match saved_lead {
            Some(val) => env::set_var("GOOSE_LEAD_MODEL", val),
            None => env::remove_var("GOOSE_LEAD_MODEL"),
        }
    }

    #[test]
    fn test_lead_worker_config_structure() {
        // Test that the LeadWorkerConfig can be deserialized properly
        let yaml_config = r#"
enabled: true
lead_provider: openai
lead_model: gpt-4o
worker_provider: anthropic
worker_model: claude-3-haiku-20240307
lead_turns: 5
failure_threshold: 3
fallback_turns: 2
"#;

        let config: LeadWorkerConfig = serde_yaml::from_str(yaml_config).unwrap();
        assert!(config.enabled);
        assert_eq!(config.lead_provider, Some("openai".to_string()));
        assert_eq!(config.lead_model, Some("gpt-4o".to_string()));
        assert_eq!(config.worker_provider, Some("anthropic".to_string()));
        assert_eq!(
            config.worker_model,
            Some("claude-3-haiku-20240307".to_string())
        );
        assert_eq!(config.lead_turns, 5);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.fallback_turns, 2);
    }

    #[test]
    fn test_lead_worker_config_defaults() {
        // Test that defaults work correctly
        let yaml_config = r#"
enabled: true
lead_model: gpt-4o
"#;

        let config: LeadWorkerConfig = serde_yaml::from_str(yaml_config).unwrap();
        assert!(config.enabled);
        assert_eq!(config.lead_model, Some("gpt-4o".to_string()));
        assert_eq!(config.lead_provider, None); // Should default
        assert_eq!(config.worker_provider, None); // Should default
        assert_eq!(config.worker_model, None); // Should default
        assert_eq!(config.lead_turns, 3); // Default
        assert_eq!(config.failure_threshold, 2); // Default
        assert_eq!(config.fallback_turns, 2); // Default
    }

    #[test]
    fn test_create_regular_provider_without_lead_config() {
        // Save current env var
        let saved_lead = env::var("GOOSE_LEAD_MODEL").ok();

        // Ensure GOOSE_LEAD_MODEL is not set
        env::remove_var("GOOSE_LEAD_MODEL");

        // This should try to create a regular provider
        let result = create("openai", ModelConfig::new("gpt-4o-mini".to_string()));

        // The creation might succeed or fail depending on API keys
        match result {
            Ok(_) => {
                // If it succeeds, it means we created a regular provider successfully
                // This would happen if API keys are available in the test environment
            }
            Err(error) => {
                // If it fails, it should be due to missing API keys
                let error_msg = error.to_string();
                assert!(error_msg.contains("OPENAI_API_KEY") || error_msg.contains("secret"));
            }
        }

        // Restore env var
        if let Some(val) = saved_lead {
            env::set_var("GOOSE_LEAD_MODEL", val);
        }
    }
}
