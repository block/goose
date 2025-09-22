use goose::agents::Agent;
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
    agent: Arc<RwLock<AgentRef>>,
    pub scheduler: Arc<RwLock<Option<Arc<dyn SchedulerTrait>>>>,
    pub recipe_file_hash_map: Arc<Mutex<HashMap<String, PathBuf>>>,
    pub session_counter: Arc<AtomicUsize>,
    /// Tracks sessions that have already emitted recipe telemetry to prevent double counting.
    recipe_session_tracker: Arc<Mutex<HashSet<String>>>,
}

impl AppState {
    pub fn new(agent: AgentRef) -> Arc<AppState> {
        Arc::new(Self {
            agent: Arc::new(RwLock::new(agent)),
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
}
