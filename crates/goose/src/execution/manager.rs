//! Agent lifecycle management with session isolation

use super::ExecutionMode;
use crate::agents::Agent;
use crate::model::ModelConfig;
use crate::providers::create;
use crate::scheduler_trait::SchedulerTrait;
use anyhow::Result;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub struct AgentManager {
    sessions: Arc<RwLock<LruCache<String, Arc<Agent>>>>,
    scheduler: Arc<RwLock<Option<Arc<dyn SchedulerTrait>>>>,
    default_provider: Arc<RwLock<Option<Arc<dyn crate::providers::base::Provider>>>>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self::with_max_sessions(100)
    }

    pub fn with_max_sessions(max_sessions: usize) -> Self {
        let capacity =
            NonZeroUsize::new(max_sessions).unwrap_or_else(|| NonZeroUsize::new(100).unwrap());
        Self {
            sessions: Arc::new(RwLock::new(LruCache::new(capacity))),
            scheduler: Arc::new(RwLock::new(None)),
            default_provider: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_scheduler(&self, scheduler: Arc<dyn SchedulerTrait>) {
        debug!("Setting scheduler on AgentManager");
        *self.scheduler.write().await = Some(scheduler);
    }

    pub async fn set_default_provider(&self, provider: Arc<dyn crate::providers::base::Provider>) {
        debug!("Setting default provider on AgentManager");
        *self.default_provider.write().await = Some(provider);
    }

    pub async fn configure_default_provider(&self) -> Result<()> {
        if let Ok(provider_name) = std::env::var("GOOSE_DEFAULT_PROVIDER") {
            if let Ok(model_name) = std::env::var("GOOSE_DEFAULT_MODEL") {
                match ModelConfig::new(&model_name) {
                    Ok(model_config) => match create(&provider_name, model_config) {
                        Ok(provider) => {
                            self.set_default_provider(provider).await;
                            info!(
                                "Configured default provider: {} with model: {}",
                                provider_name, model_name
                            );
                        }
                        Err(e) => {
                            warn!("Failed to create default provider {}: {}", provider_name, e)
                        }
                    },
                    Err(e) => warn!("Failed to create model config for {}: {}", model_name, e),
                }
            }
        }
        Ok(())
    }

    pub async fn get_agent(&self, session_id: String, mode: ExecutionMode) -> Result<Arc<Agent>> {
        // Try to get existing agent with write lock (for LRU update)
        {
            let mut sessions = self.sessions.write().await;
            if let Some(agent) = sessions.get(&session_id) {
                debug!("Found existing agent for session {}", session_id);
                return Ok(Arc::clone(agent));
            }
        }

        info!(
            "Creating new agent for session {} with mode {}",
            session_id, mode
        );

        let agent = Arc::new(Agent::new());

        // Configure agent based on mode
        match &mode {
            ExecutionMode::Interactive | ExecutionMode::Background => {
                if let Some(scheduler) = &*self.scheduler.read().await {
                    debug!("Setting scheduler on agent for session {}", session_id);
                    agent.set_scheduler(Arc::clone(scheduler)).await;
                }
            }
            ExecutionMode::SubTask { .. } => {
                debug!(
                    "SubTask mode for session {}, skipping scheduler setup",
                    session_id
                );
            }
        }

        if let Some(provider) = &*self.default_provider.read().await {
            debug!(
                "Setting default provider on agent for session {}",
                session_id
            );
            let _ = agent.update_provider(Arc::clone(provider)).await;
        }

        // Store in LRU cache (automatically evicts oldest if at capacity)
        let mut sessions = self.sessions.write().await;
        sessions.put(session_id.clone(), Arc::clone(&agent));

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

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}
