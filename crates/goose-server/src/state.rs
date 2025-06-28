use goose::agents::Agent;
use goose::scheduler_trait::SchedulerTrait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

pub type AgentRef = Arc<Agent>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToolCallStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub tool_name: String,
    pub request_id: String,
    pub status: ToolCallStatus,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub error: Option<String>,
}

impl ToolCallInfo {
    pub fn new(tool_name: String, request_id: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            tool_name,
            request_id,
            status: ToolCallStatus::Running,
            started_at: timestamp,
            completed_at: None,
            error: None,
        }
    }

    pub fn complete(&mut self) {
        self.status = ToolCallStatus::Completed;
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }

    pub fn fail(&mut self, error: String) {
        self.status = ToolCallStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }
}

#[derive(Clone)]
pub struct AppState {
    agent: Option<AgentRef>,
    pub secret_key: String,
    pub scheduler: Arc<Mutex<Option<Arc<dyn SchedulerTrait>>>>,
    pub latest_tool_call: Arc<Mutex<Option<ToolCallInfo>>>,
}

impl AppState {
    pub async fn new(agent: AgentRef, secret_key: String) -> Arc<AppState> {
        Arc::new(Self {
            agent: Some(agent.clone()),
            secret_key,
            scheduler: Arc::new(Mutex::new(None)),
            latest_tool_call: Arc::new(Mutex::new(None)),
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

    pub async fn set_tool_call_running(&self, tool_name: String, request_id: String) {
        let tool_call_info = ToolCallInfo::new(tool_name, request_id);
        let mut guard = self.latest_tool_call.lock().await;
        *guard = Some(tool_call_info);
    }

    pub async fn complete_tool_call(&self, request_id: &str) {
        let mut guard = self.latest_tool_call.lock().await;
        if let Some(ref mut tool_call) = guard.as_mut() {
            if tool_call.request_id == request_id {
                tool_call.complete();
            }
        }
    }

    pub async fn fail_tool_call(&self, request_id: &str, error: String) {
        let mut guard = self.latest_tool_call.lock().await;
        if let Some(ref mut tool_call) = guard.as_mut() {
            if tool_call.request_id == request_id {
                tool_call.fail(error);
            }
        }
    }

    pub async fn get_latest_tool_call(&self) -> Option<ToolCallInfo> {
        let guard = self.latest_tool_call.lock().await;
        guard.clone()
    }
}
