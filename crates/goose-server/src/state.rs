use goose::agents::Agent;
use goose::execution::manager::AgentManager;
use goose::execution::{ExecutionMode, SessionId};
use goose::scheduler_trait::SchedulerTrait;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

type AgentRef = Arc<Agent>;

#[derive(Clone)]
pub struct AppState {
    // Legacy: single shared agent (will be removed in future)
    agent: Arc<RwLock<AgentRef>>,
    // New: agent manager for session isolation
    agent_manager: Arc<AgentManager>,
    pub scheduler: Arc<RwLock<Option<Arc<dyn SchedulerTrait>>>>,
    pub recipe_file_hash_map: Arc<Mutex<HashMap<String, PathBuf>>>,
    pub session_counter: Arc<AtomicUsize>,
    /// Tracks sessions that have already emitted recipe telemetry to prevent double counting.
    recipe_session_tracker: Arc<Mutex<HashSet<String>>>,
}

impl AppState {
    pub fn new(agent: AgentRef) -> Arc<AppState> {
        let agent_manager = Arc::new(AgentManager::new());
        Arc::new(Self {
            agent: Arc::new(RwLock::new(agent)),
            agent_manager,
            scheduler: Arc::new(RwLock::new(None)),
            recipe_file_hash_map: Arc::new(Mutex::new(HashMap::new())),
            session_counter: Arc::new(AtomicUsize::new(0)),
            recipe_session_tracker: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub async fn get_agent(&self) -> AgentRef {
        self.agent.read().await.clone()
    }

    pub async fn set_scheduler(&self, sched: Arc<dyn SchedulerTrait>) {
        // Set on agent manager for new session-based agents
        self.agent_manager.set_scheduler(sched.clone()).await;
        // Keep for backward compatibility with legacy code
        let mut guard = self.scheduler.write().await;
        *guard = Some(sched);
    }

    pub async fn scheduler(&self) -> Result<Arc<dyn SchedulerTrait>, anyhow::Error> {
        self.scheduler
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))
    }

    pub async fn set_recipe_file_hash_map(&self, hash_map: HashMap<String, PathBuf>) {
        let mut map = self.recipe_file_hash_map.lock().await;
        *map = hash_map;
    }

    pub async fn reset(&self) {
        let mut agent = self.agent.write().await;
        *agent = Arc::new(Agent::new());
    }

    pub async fn mark_recipe_run_if_absent(&self, session_id: &str) -> bool {
        let mut sessions = self.recipe_session_tracker.lock().await;
        if sessions.contains(session_id) {
            false
        } else {
            sessions.insert(session_id.to_string());
            true
        }
    }

    /// Get an agent for a specific session using the new AgentManager
    /// This provides session isolation - each session gets its own agent
    pub async fn get_session_agent(
        &self,
        session_id: Option<String>,
    ) -> Result<AgentRef, anyhow::Error> {
        let session_id = session_id
            .map(SessionId::from)
            .unwrap_or_else(SessionId::generate);

        self.agent_manager
            .get_agent(session_id, ExecutionMode::Interactive)
            .await
    }

    /// Get the agent manager for direct access if needed
    pub fn agent_manager(&self) -> &Arc<AgentManager> {
        &self.agent_manager
    }
}
