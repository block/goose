use goose::agents::Agent;
use goose::scheduler_trait::SchedulerTrait;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type AgentRef = Arc<Agent>;

#[derive(Clone)]
pub struct AppState {
    agent: Option<AgentRef>,
    pub secret_key: String,
    pub scheduler: Arc<Mutex<Option<Arc<dyn SchedulerTrait>>>>,
}

impl AppState {
    pub async fn new(agent: AgentRef, secret_key: String) -> Arc<AppState> {
        Arc::new(Self {
            agent: Some(agent.clone()),
            secret_key,
            scheduler: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn get_agent(&self) -> Result<Arc<Agent>, anyhow::Error> {
        self.agent
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Agent needs to be created first."))
    }

    pub async fn set_scheduler(&self, sched: Arc<dyn SchedulerTrait>) {
        let mut guard = self.scheduler.lock().await;
        *guard = Some(sched);
    }

    pub async fn scheduler(&self) -> Result<Arc<dyn SchedulerTrait>, anyhow::Error> {
        self.scheduler
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))
    }
}
