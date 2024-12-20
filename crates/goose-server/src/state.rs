use anyhow::Result;
use goose::providers::configs::GroqProviderConfig;
use goose::{
    agent::Agent,
    developer::DeveloperSystem,
    memory::MemorySystem,
    providers::{configs::ProviderConfig, factory},
    systems::goose_hints::GooseHintsSystem,
};
use std::{env, sync::Arc};
use tokio::sync::Mutex;

/// Shared application state
pub struct AppState {
    pub provider_config: ProviderConfig,
    pub agent: Arc<Mutex<Agent>>,
    pub secret_key: String,
}

impl AppState {
    pub fn new(provider_config: ProviderConfig, secret_key: String) -> Result<Self> {
        let provider = factory::get_provider(provider_config.clone())?;
        let mut agent = Agent::new(provider);
        agent.add_system(Box::new(DeveloperSystem::new()));

        // Add memory system only if GOOSE_SERVER__MEMORY is set to "true"
        if let Ok(memory_enabled) = env::var("GOOSE_SERVER__MEMORY") {
            if memory_enabled.to_lowercase() == "true" {
                agent.add_system(Box::new(MemorySystem::new()));
            }
        }

        let goosehints_system = Box::new(GooseHintsSystem::new());
        agent.add_system(goosehints_system);

        Ok(Self {
            provider_config,
            agent: Arc::new(Mutex::new(agent)),
            secret_key,
        })
    }
}

// Manual Clone implementation since we know ProviderConfig variants can be cloned
impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            provider_config: match &self.provider_config {
                ProviderConfig::OpenAi(config) => {
                    ProviderConfig::OpenAi(goose::providers::configs::OpenAiProviderConfig {
                        host: config.host.clone(),
                        api_key: config.api_key.clone(),
                        model: config.model.clone(),
                    })
                }
                ProviderConfig::Databricks(config) => ProviderConfig::Databricks(
                    goose::providers::configs::DatabricksProviderConfig {
                        host: config.host.clone(),
                        auth: config.auth.clone(),
                        model: config.model.clone(),
                        image_format: config.image_format,
                    },
                ),
                ProviderConfig::Ollama(config) => {
                    ProviderConfig::Ollama(goose::providers::configs::OllamaProviderConfig {
                        host: config.host.clone(),
                        model: config.model.clone(),
                    })
                }
                ProviderConfig::Anthropic(config) => {
                    ProviderConfig::Anthropic(goose::providers::configs::AnthropicProviderConfig {
                        host: config.host.clone(),
                        api_key: config.api_key.clone(),
                        model: config.model.clone(),
                    })
                }
                ProviderConfig::Google(config) => {
                    ProviderConfig::Google(goose::providers::configs::GoogleProviderConfig {
                        host: config.host.clone(),
                        api_key: config.api_key.clone(),
                        model: config.model.clone(),
                    })
                }
                ProviderConfig::Groq(config) => ProviderConfig::Groq(GroqProviderConfig {
                    host: config.host.clone(),
                    api_key: config.api_key.clone(),
                    model: config.model.clone(),
                }),
            },
            agent: self.agent.clone(),
            secret_key: self.secret_key.clone(),
        }
    }
}
