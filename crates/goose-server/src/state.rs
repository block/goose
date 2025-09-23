use goose::execution::manager::AgentManager;
use goose::execution::SessionExecutionMode;
use goose::scheduler_trait::SchedulerTrait;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub(crate) agent_manager: Arc<AgentManager>,
    pub recipe_file_hash_map: Arc<Mutex<HashMap<String, PathBuf>>>,
    pub session_counter: Arc<AtomicUsize>,
    /// Tracks sessions that have already emitted recipe telemetry to prevent double counting.
    recipe_session_tracker: Arc<Mutex<HashSet<String>>>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Arc<AppState>> {
        let agent_manager = Arc::new(AgentManager::new().await?);
        Ok(Arc::new(Self {
            agent_manager,
            recipe_file_hash_map: Arc::new(Mutex::new(HashMap::new())),
            session_counter: Arc::new(AtomicUsize::new(0)),
            recipe_session_tracker: Arc::new(Mutex::new(HashSet::new())),
        }))
    }

    pub async fn scheduler(&self) -> Result<Arc<dyn SchedulerTrait>, anyhow::Error> {
        self.agent_manager.scheduler().await
    }

    pub async fn set_recipe_file_hash_map(&self, hash_map: HashMap<String, PathBuf>) {
        let mut map = self.recipe_file_hash_map.lock().await;
        *map = hash_map;
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

    pub async fn get_agent(
        &self,
        session_id: String,
        mode: SessionExecutionMode,
    ) -> anyhow::Result<Arc<goose::agents::Agent>> {
        self.agent_manager
            .get_or_create_agent(session_id, mode)
            .await
    }
}
