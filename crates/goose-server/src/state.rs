use goose::agents::Agent;
use goose::scheduler_trait::SchedulerTrait;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::RwLock;

type AgentRef = Arc<Agent>;

#[derive(Clone)]
pub struct AppState {
    agent: Arc<RwLock<AgentRef>>,
    pub secret_key: String,
    pub scheduler: Arc<RwLock<Option<Arc<dyn SchedulerTrait>>>>,
    pub session_counter: Arc<AtomicUsize>,
}

impl AppState {
    pub fn new(agent: AgentRef, secret_key: String) -> Arc<AppState> {
        Arc::new(Self {
            agent: Arc::new(RwLock::new(agent)),
            secret_key,
            scheduler: Arc::new(RwLock::new(None)),
            session_counter: Arc::new(AtomicUsize::new(0)),
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

    pub async fn reset(&self) {
        let mut agent = self.agent.write().await;
        *agent = Arc::new(Agent::new());
    }
}
