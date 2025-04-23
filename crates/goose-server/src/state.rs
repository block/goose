use goose::agents::Agent;
use std::sync::Arc;

/// Shared reference to an Agent that can be cloned cheaply
/// without cloning the underlying Agent object
pub type AgentRef = Arc<Agent>;

/// Thread-safe container for an optional Agent reference
/// Outer Arc: Allows multiple route handlers to access the same Mutex
/// - Mutex provides exclusive access for updates
/// - Option allows for the case where no agent exists yet
///
/// Shared application state
#[derive(Clone)]
pub struct AppState {
    // agent: SharedAgentStore,
    agent: Option<AgentRef>,
    pub secret_key: String,
}

impl AppState {
    pub async fn new(agent: AgentRef, secret_key: String) -> Arc<AppState> {
        Arc::new(Self {
            agent: Some(agent.clone()),
            secret_key,
        })
    }

    pub async fn get_agent(&self) -> Result<Arc<Agent>, anyhow::Error> {
        self.agent
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Agent needs to be created first."))
    }
}
