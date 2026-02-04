//! Swarm error types

use thiserror::Error;

/// Errors that can occur in swarm operations
#[derive(Debug, Error)]
pub enum SwarmError {
    #[error("Swarm error: {kind}")]
    Swarm { kind: SwarmErrorKind, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    #[error("Agent pool error: {0}")]
    AgentPool(String),

    #[error("Communication error: {0}")]
    Communication(String),

    #[error("Consensus failed: {0}")]
    Consensus(String),

    #[error("Task distribution error: {0}")]
    Distribution(String),

    #[error("Shared memory error: {0}")]
    SharedMemory(String),

    #[error("Batch API error: {0}")]
    BatchApi(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Swarm not found: {0}")]
    SwarmNotFound(String),

    #[error("Invalid topology: {0}")]
    InvalidTopology(String),

    #[error("Capacity exceeded: {0}")]
    CapacityExceeded(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Specific kinds of swarm errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmErrorKind {
    /// Failed to spawn agent
    SpawnFailed,
    /// Agent crashed or became unresponsive
    AgentCrashed,
    /// Message delivery failed
    MessageDeliveryFailed,
    /// Consensus could not be reached
    ConsensusTimeout,
    /// All agents failed the task
    AllAgentsFailed,
    /// Task was cancelled
    TaskCancelled,
    /// Invalid configuration
    InvalidConfig,
    /// Resource contention
    ResourceContention,
    /// Scaling limit reached
    ScalingLimitReached,
}

impl std::fmt::Display for SwarmErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpawnFailed => write!(f, "failed to spawn agent"),
            Self::AgentCrashed => write!(f, "agent crashed or became unresponsive"),
            Self::MessageDeliveryFailed => write!(f, "message delivery failed"),
            Self::ConsensusTimeout => write!(f, "consensus could not be reached in time"),
            Self::AllAgentsFailed => write!(f, "all agents failed the task"),
            Self::TaskCancelled => write!(f, "task was cancelled"),
            Self::InvalidConfig => write!(f, "invalid configuration"),
            Self::ResourceContention => write!(f, "resource contention detected"),
            Self::ScalingLimitReached => write!(f, "scaling limit reached"),
        }
    }
}

impl SwarmError {
    pub fn spawn_failed(msg: impl Into<String>) -> Self {
        Self::Swarm {
            kind: SwarmErrorKind::SpawnFailed,
            source: Some(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                msg.into(),
            ))),
        }
    }

    pub fn agent_crashed(agent_id: impl std::fmt::Display) -> Self {
        Self::Swarm {
            kind: SwarmErrorKind::AgentCrashed,
            source: Some(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Agent {} crashed", agent_id),
            ))),
        }
    }

    pub fn consensus_timeout() -> Self {
        Self::Swarm {
            kind: SwarmErrorKind::ConsensusTimeout,
            source: None,
        }
    }

    pub fn all_agents_failed() -> Self {
        Self::Swarm {
            kind: SwarmErrorKind::AllAgentsFailed,
            source: None,
        }
    }
}

pub type SwarmResult<T> = Result<T, SwarmError>;
