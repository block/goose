//! Agent lifecycle management with session isolation

use super::SessionExecutionMode;
use crate::agents::Agent;
use crate::config::APP_STRATEGY;
use crate::model::ModelConfig;
use crate::providers::create;
use crate::scheduler_factory::SchedulerFactory;
use crate::scheduler_trait::SchedulerTrait;
use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub struct AgentManager {
    sessions: Arc<RwLock<LruCache<String, Arc<Agent>>>>,
    scheduler: Arc<dyn SchedulerTrait>,
    default_provider: Arc<RwLock<Option<Arc<dyn crate::providers::base::Provider>>>>,
}

impl AgentManager {
    pub async fn new(max_sessions: Option<usize>) -> Result<Self> {
        // Construct scheduler with the standard goose-server path
        let schedule_file_path = choose_app_strategy(APP_STRATEGY.clone())?
            .data_dir()
            .join("schedule.json");

        let scheduler = SchedulerFactory::create(schedule_file_path).await?;

        let capacity = NonZeroUsize::new(max_sessions.unwrap_or(100))
            .unwrap_or_else(|| NonZeroUsize::new(100).unwrap());

        let manager = Self {
            sessions: Arc::new(RwLock::new(LruCache::new(capacity))),
            scheduler,
            default_provider: Arc::new(RwLock::new(None)),
        };

        if let Err(e) = manager.configure_default_provider().await {
            warn!("❌ Failed to configure default provider during AgentManager initialization: {}", e);
            // Don't fail the entire AgentManager creation, but log the error
        }

        Ok(manager)
    }

    pub async fn scheduler(&self) -> Result<Arc<dyn SchedulerTrait>> {
        Ok(Arc::clone(&self.scheduler))
    }

    pub async fn set_default_provider(&self, provider: Arc<dyn crate::providers::base::Provider>) {
        debug!("Setting default provider on AgentManager");
        *self.default_provider.write().await = Some(provider);
    }

    pub async fn configure_default_provider(&self) -> Result<()> {
        debug!("🔧 Starting configure_default_provider");
        
        let provider_name = std::env::var("GOOSE_DEFAULT_PROVIDER")
            .or_else(|_| std::env::var("GOOSE_PROVIDER__TYPE"))
            .ok();

        let model_name = std::env::var("GOOSE_DEFAULT_MODEL")
            .or_else(|_| std::env::var("GOOSE_PROVIDER__MODEL"))
            .ok();

        debug!("🔧 Environment variables - GOOSE_DEFAULT_PROVIDER: {:?}, GOOSE_DEFAULT_MODEL: {:?}", provider_name, model_name);

        if provider_name.is_none() || model_name.is_none() {
            warn!("❌ Missing provider configuration - provider_name: {:?}, model_name: {:?}", provider_name, model_name);
            warn!("❌ Available env vars: GOOSE_DEFAULT_PROVIDER={:?}, GOOSE_DEFAULT_MODEL={:?}", 
                  std::env::var("GOOSE_DEFAULT_PROVIDER").ok(),
                  std::env::var("GOOSE_DEFAULT_MODEL").ok());
            return Ok(());
        }

        if let (Some(provider_name), Some(model_name)) = (provider_name, model_name) {
            debug!("🔧 Creating provider '{}' with model '{}'", provider_name, model_name);
            match ModelConfig::new(&model_name) {
                Ok(model_config) => {
                    debug!("🔧 Created model config for {}: {:?}", model_name, model_config);
                    match create(&provider_name, model_config) {
                        Ok(provider) => {
                            self.set_default_provider(provider).await;
                            info!(
                                "✅ Successfully configured default provider: {} with model: {}",
                                provider_name, model_name
                            );
                        }
                        Err(e) => {
                            warn!("❌ Failed to create default provider {}: {}", provider_name, e);
                            warn!("❌ This will cause the system to fall back to OpenAI, which may cause quota errors");
                            // This is critical - if provider creation fails, we need to know about it
                            return Err(anyhow::anyhow!("Failed to create default provider {}: {}", provider_name, e));
                        }
                    }
                },
                Err(e) => {
                    warn!("❌ Failed to create model config for {}: {}", model_name, e);
                    warn!("❌ This will cause the system to fall back to OpenAI, which may cause quota errors");
                    return Err(anyhow::anyhow!("Failed to create model config for {}: {}", model_name, e));
                }
            }
        }
        Ok(())
    }

    pub async fn get_or_create_agent(
        &self,
        session_id: String,
        mode: SessionExecutionMode,
    ) -> Result<Arc<Agent>> {
        let agent = {
            let mut sessions = self.sessions.write().await;
            if let Some(agent) = sessions.get(&session_id) {
                debug!("Found existing agent for session {}", session_id);
                return Ok(Arc::clone(agent));
            }

            info!(
                "Creating new agent for session {} with mode {}",
                session_id, mode
            );
            let agent = Arc::new(Agent::new());
            sessions.put(session_id.clone(), Arc::clone(&agent));
            agent
        };

        match &mode {
            SessionExecutionMode::Interactive | SessionExecutionMode::Background => {
                debug!("Setting scheduler on agent for session {}", session_id);
                agent.set_scheduler(Arc::clone(&self.scheduler)).await;
            }
            SessionExecutionMode::SubTask { .. } => {
                debug!(
                    "SubTask mode for session {}, skipping scheduler setup",
                    session_id
                );
            }
        }

        if let Some(provider) = &*self.default_provider.read().await {
            info!("🔧 Setting default provider on agent for session {}", session_id);
            match agent.update_provider(Arc::clone(provider)).await {
                Ok(_) => {
                    info!("✅ Successfully set provider on agent for session {}", session_id);
                }
                Err(e) => {
                    warn!("❌ Failed to set provider on agent for session {}: {}", session_id, e);
                    warn!("❌ This may cause the agent to fall back to OpenAI, leading to quota errors");
                }
            }
        } else {
            warn!("❌ No default provider available for session {}", session_id);
            warn!("❌ Agent will likely fall back to OpenAI, which may cause quota errors");
            warn!("❌ Check that GOOSE_DEFAULT_PROVIDER and GOOSE_DEFAULT_MODEL are set correctly");
        }

        Ok(agent)
    }

    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions
            .pop(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;
        info!("Removed session {}", session_id);
        Ok(())
    }

    pub async fn has_session(&self, session_id: &str) -> bool {
        self.sessions.read().await.contains(session_id)
    }

    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}
