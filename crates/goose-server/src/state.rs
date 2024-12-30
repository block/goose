use anyhow::Result;
use goose::providers::configs::GroqProviderConfig;
use goose::{
    agent::Agent,
    developer::DeveloperSystem,
    memory::MemorySystem,
    providers::{configs::ProviderConfig, factory},
    systems::{goose_hints::GooseHintsSystem, non_developer::NonDeveloperSystem},
};
use std::{env, path::Path, process::Command, sync::Arc};
use tokio::sync::Mutex;

/// Check if the current directory or any parent directory contains a .git folder
fn is_in_git_repository() -> bool {
    match Command::new("git")
        .arg("rev-parse")
        .arg("--git-dir")
        .output()
    {
        Ok(output) => output.status.success(),
        Err(_) => false, // Return false if git command fails (e.g., git not installed)
    }
}

/// Check if a .goosehints file exists in the current directory
fn has_goosehints_file() -> bool {
    Path::new(".goosehints").exists()
}

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

        dbg!("Adding DeveloperSystem");
        agent.add_system(Box::new(DeveloperSystem::new()));

        // Only add NonDeveloperSystem if we're not in a git repository and don't have a .goosehints file
        let in_git = is_in_git_repository();
        let has_hints = has_goosehints_file();

        if !in_git && !has_hints {
            dbg!("Adding NonDeveloperSystem");
            agent.add_system(Box::new(NonDeveloperSystem::new()));
        } else {
            dbg!("Skipping NonDeveloperSystem");
        }

        // Add memory system only if GOOSE_SERVER__MEMORY is set to "true"
        if let Ok(memory_enabled) = env::var("GOOSE_SERVER__MEMORY") {
            if memory_enabled.to_lowercase() == "true" {
                dbg!("Adding MemorySystem");
                agent.add_system(Box::new(MemorySystem::new()));
            } else {
                dbg!("Skipping MemorySystem (GOOSE_SERVER__MEMORY not 'true')");
            }
        } else {
            dbg!("Skipping MemorySystem (GOOSE_SERVER__MEMORY not set)");
        }

        dbg!("Adding GooseHintsSystem");
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
