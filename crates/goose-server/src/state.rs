use anyhow::Result;
use goose::{
    agents::Agent,
    agents::AgentFactory,
    providers::{configs::ProviderConfig, factory},
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state
#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub provider_config: ProviderConfig,
    pub agent: Arc<Mutex<Box<dyn Agent>>>,
    pub secret_key: String,
    pub agent_version: String,
}

impl AppState {
    pub async fn new(
        provider_config: ProviderConfig,
        secret_key: String,
        agent_version: Option<String>,
    ) -> Result<Self> {
        let provider = factory::get_provider(provider_config.clone())?;
        let agent = AgentFactory::create(
            agent_version
                .clone()
                .unwrap_or(AgentFactory::default_version().to_string())
                .as_str(),
            provider,
        )
        .ok_or(anyhow::Error::msg("Invalid agent version requested"))?;

        Ok(Self {
            provider_config,
            agent: Arc::new(Mutex::new(agent)),
            secret_key,
            agent_version: agent_version
                .clone()
                .unwrap_or(AgentFactory::default_version().to_string()),
        })
    }
}
