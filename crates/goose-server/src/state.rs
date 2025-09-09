use goose::agents::manager::{AgentManager, AgentManagerConfig};
use goose::agents::Agent;
use goose::scheduler_trait::SchedulerTrait;
use goose::session;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

type AgentRef = Arc<Agent>;

#[derive(Clone)]
pub struct AppState {
    agent_manager: Arc<AgentManager>,
    pub secret_key: String,
    pub scheduler: Arc<RwLock<Option<Arc<dyn SchedulerTrait>>>>,
    pub recipe_file_hash_map: Arc<Mutex<HashMap<String, PathBuf>>>,
    pub session_counter: Arc<AtomicUsize>,
}

impl AppState {
    pub async fn new(secret_key: String) -> Arc<AppState> {
        let agent_manager = Arc::new(AgentManager::new(AgentManagerConfig::default()).await);
        Arc::new(Self {
            agent_manager,
            secret_key,
            scheduler: Arc::new(RwLock::new(None)),
            recipe_file_hash_map: Arc::new(Mutex::new(HashMap::new())),
            session_counter: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub async fn get_agent(
        &self,
        session_id: session::Identifier,
    ) -> Result<AgentRef, anyhow::Error> {
        self.agent_manager
            .get_agent(session_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get agent: {}", e))
    }

    pub async fn set_scheduler(&self, sched: Arc<dyn SchedulerTrait>) {
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

    pub async fn cleanup_idle_agents(&self) -> Result<usize, anyhow::Error> {
        self.agent_manager
            .cleanup()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to cleanup agents: {}", e))
    }

    pub async fn get_agent_metrics(&self) -> goose::agents::manager::AgentMetrics {
        self.agent_manager.get_metrics().await
    }
}
