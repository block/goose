//! Agent lifecycle management with session isolation

use super::{ExecutionMode, SessionId};
use crate::agents::Agent;
use crate::model::ModelConfig;
use crate::providers::create;
use crate::scheduler_trait::SchedulerTrait;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Manages agents with session awareness and isolation
pub struct AgentManager {
    /// Active sessions mapped to their agents and metadata
    sessions: Arc<RwLock<HashMap<SessionId, SessionData>>>,

    /// Shared scheduler for background tasks (optional)
    scheduler: Arc<RwLock<Option<Arc<dyn SchedulerTrait>>>>,

    /// Maximum number of concurrent sessions
    max_sessions: usize,

    /// Default provider configuration (optional)
    default_provider: Arc<RwLock<Option<Arc<dyn crate::providers::base::Provider>>>>,
}

/// Metadata about an active session
struct SessionData {
    /// The agent instance for this session
    agent: Arc<Agent>,

    /// Execution mode for this session
    #[allow(dead_code)]
    mode: ExecutionMode,

    /// When the session was created
    #[allow(dead_code)]
    created_at: DateTime<Utc>,

    /// Last time the session was accessed
    last_used: DateTime<Utc>,
}

impl AgentManager {
    /// Create a new AgentManager with default settings
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            scheduler: Arc::new(RwLock::new(None)),
            max_sessions: 100, // Default limit
            default_provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with custom max sessions limit
    pub fn with_max_sessions(max_sessions: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            scheduler: Arc::new(RwLock::new(None)),
            max_sessions,
            default_provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the scheduler to be used for background tasks
    pub async fn set_scheduler(&self, scheduler: Arc<dyn SchedulerTrait>) {
        debug!("Setting scheduler on AgentManager");
        *self.scheduler.write().await = Some(scheduler);
    }

    /// Set a default provider to be used for new agents
    pub async fn set_default_provider(&self, provider: Arc<dyn crate::providers::base::Provider>) {
        debug!("Setting default provider on AgentManager");
        *self.default_provider.write().await = Some(provider);
    }

    /// Configure default provider from environment variables
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

    /// Get or create an agent for the given session
    pub async fn get_agent(
        &self,
        session_id: SessionId,
        mode: ExecutionMode,
    ) -> Result<Arc<Agent>> {
        // First check if we already have this session
        {
            let sessions = self.sessions.read().await;
            if let Some(data) = sessions.get(&session_id) {
                debug!("Found existing agent for session {}", session_id);
                let agent = Arc::clone(&data.agent);
                // Drop the lock before calling touch_session
                drop(sessions);
                self.touch_session(&session_id).await;
                return Ok(agent);
            }
        }

        // Need to create a new agent
        let mut sessions = self.sessions.write().await;

        // Double-check after acquiring write lock (another thread might have created it)
        if let Some(data) = sessions.get(&session_id) {
            debug!(
                "Found existing agent for session {} (after write lock)",
                session_id
            );
            return Ok(Arc::clone(&data.agent));
        }

        info!(
            "Creating new agent for session {} with mode {}",
            session_id, mode
        );

        // Enforce session limit
        if sessions.len() >= self.max_sessions {
            warn!(
                "Session limit reached ({}), evicting oldest session",
                self.max_sessions
            );
            self.evict_oldest_session(&mut sessions);
        }

        // Create new agent
        let agent = Arc::new(Agent::new());

        // Configure based on execution mode
        match &mode {
            ExecutionMode::Interactive | ExecutionMode::Background => {
                // These modes might need the scheduler
                if let Some(scheduler) = &*self.scheduler.read().await {
                    debug!("Setting scheduler on agent for session {}", session_id);
                    agent.set_scheduler(Arc::clone(scheduler)).await;
                }
            }
            ExecutionMode::SubTask { .. } => {
                // Sub-tasks typically don't need direct scheduler access
                debug!(
                    "SubTask mode for session {}, skipping scheduler setup",
                    session_id
                );
            }
        }

        // Set default provider if available
        if let Some(provider) = &*self.default_provider.read().await {
            debug!(
                "Setting default provider on agent for session {}",
                session_id
            );
            let _ = agent.update_provider(Arc::clone(provider)).await;
        }

        // Store the new session
        let now = Utc::now();
        sessions.insert(
            session_id.clone(),
            SessionData {
                agent: Arc::clone(&agent),
                mode,
                created_at: now,
                last_used: now,
            },
        );

        Ok(agent)
    }

    /// Remove a specific session
    pub async fn remove_session(&self, session_id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions
            .remove(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;
        info!("Removed session {}", session_id);
        Ok(())
    }

    /// Check if a session exists
    pub async fn has_session(&self, session_id: &SessionId) -> bool {
        self.sessions.read().await.contains_key(session_id)
    }

    /// Get the number of active sessions
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Update the last_used timestamp for a session
    async fn touch_session(&self, session_id: &SessionId) {
        let mut sessions = self.sessions.write().await;
        if let Some(data) = sessions.get_mut(session_id) {
            data.last_used = Utc::now();
        }
    }

    /// Remove the oldest session (by last_used time)
    fn evict_oldest_session(&self, sessions: &mut HashMap<SessionId, SessionData>) {
        if let Some((oldest_id, _)) = sessions
            .iter()
            .min_by_key(|(_, data)| data.last_used)
            .map(|(id, data)| (id.clone(), data.last_used))
        {
            info!("Evicting oldest session: {}", oldest_id);
            sessions.remove(&oldest_id);
        }
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}
