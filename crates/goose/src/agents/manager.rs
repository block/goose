use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::agents::Agent;
use crate::session;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;

/// Error types for AgentManager operations
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Failed to create agent: {0}")]
    CreationFailed(String),

    #[error("Failed to acquire lock: {0}")]
    LockError(String),

    #[error("Agent not found for session: {0}")]
    NotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Configuration for the AgentManager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManagerConfig {
    /// Maximum idle time before an agent is cleaned up
    pub max_idle_duration: Duration,

    /// Whether to enable agent pooling (future optimization)
    pub enable_pooling: bool,

    /// Maximum number of agents to keep in memory
    pub max_agents: usize,

    /// Interval for running cleanup
    pub cleanup_interval: Duration,
}

impl Default for AgentManagerConfig {
    fn default() -> Self {
        Self {
            max_idle_duration: Duration::from_secs(3600), // 1 hour
            enable_pooling: false,
            max_agents: 100,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Execution mode for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Interactive mode - user is directly interacting
    Interactive,

    /// Background mode - running as scheduled job
    Background,

    /// SubTask mode - running as subtask of another agent
    SubTask {
        parent: session::Identifier,
        inherit: InheritConfig,
        approval_mode: ApprovalMode,
    },
}

/// Configuration for what a subtask inherits from parent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritConfig {
    pub extensions: bool,
    pub provider: bool,
    pub settings: bool,
}

impl Default for InheritConfig {
    fn default() -> Self {
        Self {
            extensions: true,
            provider: true,
            settings: true,
        }
    }
}

/// Defines how subtask tool approvals are handled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalMode {
    /// Subtask handles all approvals locally (default, backward compatible)
    Autonomous,

    /// All approval requests bubble to parent
    BubbleAll,

    /// Selective bubbling based on tool names
    BubbleFiltered {
        tools: Vec<String>,
        default_action: ApprovalAction,
    },
}

impl Default for ApprovalMode {
    fn default() -> Self {
        Self::Autonomous // Maintain backward compatibility
    }
}

/// Default action for tools not in filter list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalAction {
    Approve,
    Deny,
    Bubble,
}

/// State of a session's agent
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum SessionState {
    Active,
    Idle,
    Executing,
}

/// Wrapper for an agent with session metadata
struct SessionAgent {
    agent: Arc<Agent>,
    #[allow(dead_code)]
    session_id: session::Identifier,
    #[allow(dead_code)]
    created_at: DateTime<Utc>,
    last_used: DateTime<Utc>,
    execution_mode: ExecutionMode,
    #[allow(dead_code)]
    state: SessionState,
}

/// Metrics for monitoring agent manager performance
#[derive(Debug, Default)]
pub struct AgentMetrics {
    pub agents_created: usize,
    pub agents_cleaned: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub active_agents: usize,
}

impl AgentMetrics {
    fn record_agent_created(&mut self) {
        self.agents_created += 1;
        self.active_agents += 1;
    }

    fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    fn record_cleanup(&mut self, count: usize) {
        self.agents_cleaned += count;
        self.active_agents = self.active_agents.saturating_sub(count);
    }
}

/// Optional agent pooling for future optimization
struct AgentPool {
    // Future implementation for agent reuse
    _placeholder: std::marker::PhantomData<()>,
}

/// Manages agent lifecycle and session mapping
///
/// This is the central component that ensures each session gets its own
/// isolated agent instance, solving the shared agent concurrency issues.
pub struct AgentManager {
    /// Maps session IDs to their dedicated agents
    agents: Arc<RwLock<HashMap<session::Identifier, SessionAgent>>>,

    /// Optional pool for agent reuse (future optimization)
    #[allow(dead_code)]
    pool: Option<AgentPool>,

    /// Configuration for agent creation and management
    config: AgentManagerConfig,

    /// Metrics for monitoring and debugging
    metrics: Arc<RwLock<AgentMetrics>>,
}

impl AgentManager {
    /// Create a new AgentManager with the given configuration
    pub async fn new(config: AgentManagerConfig) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            pool: None, // Start without pooling, add later
            config,
            metrics: Arc::new(RwLock::new(AgentMetrics::default())),
        }
    }

    /// Get or create an agent for a session
    ///
    /// This ensures each session has exactly one agent, providing
    /// complete isolation between users/sessions.
    pub async fn get_agent(
        &self,
        session_id: session::Identifier,
    ) -> Result<Arc<Agent>, AgentError> {
        // First try to get existing agent with read lock
        {
            let agents = self.agents.read().await;
            if let Some(session_agent) = agents.get(&session_id) {
                // Update metrics
                self.metrics.write().await.record_cache_hit();

                // Clone Arc and return (last_used will be updated separately)
                return Ok(Arc::clone(&session_agent.agent));
            }
        }

        // Need to create new agent - acquire write lock
        let mut agents = self.agents.write().await;

        // Double-check in case another thread created it while we waited for write lock
        if let Some(session_agent) = agents.get_mut(&session_id) {
            session_agent.last_used = Utc::now();
            self.metrics.write().await.record_cache_hit();
            return Ok(Arc::clone(&session_agent.agent));
        }

        // Create new agent for this session
        self.metrics.write().await.record_cache_miss();
        let agent = self.create_agent_for_session(session_id.clone()).await?;

        // Store the agent
        agents.insert(
            session_id.clone(),
            SessionAgent {
                agent: Arc::clone(&agent),
                session_id: session_id.clone(),
                created_at: Utc::now(),
                last_used: Utc::now(),
                execution_mode: ExecutionMode::Interactive,
                state: SessionState::Active,
            },
        );

        self.metrics.write().await.record_agent_created();
        Ok(agent)
    }

    /// Update the last used time for a session's agent
    pub async fn touch_session(&self, session_id: &session::Identifier) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        if let Some(session_agent) = agents.get_mut(session_id) {
            session_agent.last_used = Utc::now();
            Ok(())
        } else {
            Err(AgentError::NotFound(format!("{:?}", session_id)))
        }
    }

    /// Get agent for a session with specific execution mode
    pub async fn get_agent_with_mode(
        &self,
        session_id: session::Identifier,
        mode: ExecutionMode,
    ) -> Result<Arc<Agent>, AgentError> {
        let agent = self.get_agent(session_id.clone()).await?;

        // Update execution mode for the session
        let mut agents = self.agents.write().await;
        if let Some(session_agent) = agents.get_mut(&session_id) {
            session_agent.execution_mode = mode;
        }

        Ok(agent)
    }

    /// Create a new agent for a session
    async fn create_agent_for_session(
        &self,
        _session_id: session::Identifier,
    ) -> Result<Arc<Agent>, AgentError> {
        // For now, create a new agent directly
        // In the future, this could use pooling or other optimizations

        // Note: Agent::new() is synchronous, so we don't need await here
        let agent = Agent::new();

        // Initialize the agent with a provider from configuration
        if let Err(e) = Self::initialize_agent_provider(&agent).await {
            tracing::warn!("Failed to initialize provider for new agent: {}", e);
            // Continue without provider - it can be set later via API
        }

        // TODO: Once Agent is updated with session support, use:
        // let agent = Agent::new_with_session(session_id, ExecutionMode::Interactive);

        Ok(Arc::new(agent))
    }

    /// Initialize an agent with a provider from configuration
    async fn initialize_agent_provider(agent: &Agent) -> Result<(), AgentError> {
        use crate::config::Config;
        use crate::model::ModelConfig;
        use crate::providers::create;

        let config = Config::global();

        // Get provider and model from environment/config
        let provider_name = config
            .get_param::<String>("GOOSE_PROVIDER")
            .map_err(|e| AgentError::ConfigError(format!("No provider configured: {}", e)))?;

        let model_name = config
            .get_param::<String>("GOOSE_MODEL")
            .map_err(|e| AgentError::ConfigError(format!("No model configured: {}", e)))?;

        // Create model configuration
        let model_config = ModelConfig::new(&model_name)
            .map_err(|e| AgentError::ConfigError(format!("Invalid model config: {}", e)))?;

        // Create provider
        let provider = create(&provider_name, model_config)
            .map_err(|e| AgentError::CreationFailed(format!("Failed to create provider: {}", e)))?;

        // Set the provider on the agent
        agent
            .update_provider(provider)
            .await
            .map_err(|e| AgentError::CreationFailed(format!("Failed to set provider: {}", e)))?;

        Ok(())
    }

    /// Clean up idle agents to manage memory
    ///
    /// Following the pattern from session::storage::cleanup_old_sessions
    pub async fn cleanup_idle(&self, max_idle: Duration) -> Result<usize, AgentError> {
        let mut agents = self.agents.write().await;
        let now = Utc::now();
        let mut removed = 0;

        agents.retain(|_, session_agent| {
            let idle_time = now.signed_duration_since(session_agent.last_used);
            if idle_time > chrono::Duration::from_std(max_idle).unwrap() {
                removed += 1;
                false
            } else {
                true
            }
        });

        self.metrics.write().await.record_cleanup(removed);
        Ok(removed)
    }

    /// Run cleanup based on configured interval
    pub async fn cleanup(&self) -> Result<usize, AgentError> {
        self.cleanup_idle(self.config.max_idle_duration).await
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> AgentMetrics {
        let metrics = self.metrics.read().await;
        AgentMetrics {
            agents_created: metrics.agents_created,
            agents_cleaned: metrics.agents_cleaned,
            cache_hits: metrics.cache_hits,
            cache_misses: metrics.cache_misses,
            active_agents: metrics.active_agents,
        }
    }

    /// Get number of active agents
    pub async fn active_agent_count(&self) -> usize {
        self.agents.read().await.len()
    }

    /// Remove a specific session's agent
    pub async fn remove_agent(&self, session_id: &session::Identifier) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        if agents.remove(session_id).is_some() {
            self.metrics.write().await.record_cleanup(1);
            Ok(())
        } else {
            Err(AgentError::NotFound(format!("{:?}", session_id)))
        }
    }

    /// Check if a session has an active agent
    pub async fn has_agent(&self, session_id: &session::Identifier) -> bool {
        self.agents.read().await.contains_key(session_id)
    }
}
