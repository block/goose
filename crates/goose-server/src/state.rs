use anyhow::Result;
use goose::{
    agent::Agent,
    developer::DeveloperSystem,
    memory::MemorySystem,
    providers::{configs::ProviderConfig, factory},
    systems::{goose_hints::GooseHintsSystem, non_developer::NonDeveloperSystem},
};
use std::{env, sync::Arc};
use tokio::sync::Mutex;

/// Shared application state
#[allow(dead_code)]
#[derive(Clone)]
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

        // Add NonDeveloperSystem only if GOOSE_SERVER__NON_DEVELOPER is set to "true"
        if let Ok(non_dev_enabled) = env::var("GOOSE_SERVER__NON_DEVELOPER") {
            if non_dev_enabled.to_lowercase() == "true" {
                dbg!("Adding NonDeveloperSystem");
                agent.add_system(Box::new(NonDeveloperSystem::new()));
            } else {
                dbg!("Skipping NonDeveloperSystem (GOOSE_SERVER__NON_DEVELOPER not 'true')");
            }
        } else {
            dbg!("Skipping NonDeveloperSystem (GOOSE_SERVER__NON_DEVELOPER not set)");
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
