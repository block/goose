use goose::agents::Agent;
use crate::scheduler::Scheduler;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    pub scheduler: Mutex<Option<Arc<Scheduler>>>,
}

impl AppState {
    pub async fn new(agent: AgentRef, secret_key: String) -> Arc<AppState> {
        Arc::new(Self {
            agent: Some(agent.clone()),
            secret_key,
            scheduler: Mutex::new(None),
        })
    }

    pub async fn get_agent(&self) -> Result<Arc<Agent>, anyhow::Error> {
        self.agent
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Agent needs to be created first."))
    }

    pub async fn set_scheduler(&self, sched: Arc<Scheduler>) {
        let mut guard = self.scheduler.lock().await;
        *guard = Some(sched);
    }

    pub async fn scheduler(&self) -> Result<Arc<Scheduler>, anyhow::Error> {
        self.scheduler
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))
    }
}
