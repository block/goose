use axum::http::StatusCode;
use goose::conversation::message::Message;
use goose::execution::manager::AgentManager;
use goose::scheduler_trait::SchedulerTrait;
use goose::session::Session;
use rmcp::model::ServerNotification;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum MessageEvent {
    Message {
        message: Message,
    },
    Error {
        error: String,
    },
    Finish {
        reason: String,
    },
    ModelChange {
        model: String,
        mode: String,
    },
    Notification {
        request_id: String,
        message: ServerNotification,
    },
    Ping,
    SessionSnapshot {
        session: Session,
    },
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) agent_manager: Arc<AgentManager>,
    pub recipe_file_hash_map: Arc<Mutex<HashMap<String, PathBuf>>>,
    pub session_counter: Arc<AtomicUsize>,
    recipe_session_tracker: Arc<Mutex<HashSet<String>>>,
    session_streams: Arc<Mutex<HashMap<String, broadcast::Sender<MessageEvent>>>>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Arc<AppState>> {
        let agent_manager = AgentManager::instance().await?;
        Ok(Arc::new(Self {
            agent_manager,
            recipe_file_hash_map: Arc::new(Mutex::new(HashMap::new())),
            session_counter: Arc::new(AtomicUsize::new(0)),
            recipe_session_tracker: Arc::new(Mutex::new(HashSet::new())),
            session_streams: Arc::new(Mutex::new(HashMap::new())),
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

    pub async fn get_agent(&self, session_id: String) -> anyhow::Result<Arc<goose::agents::Agent>> {
        self.agent_manager.get_or_create_agent(session_id).await
    }

    pub async fn get_agent_for_route(
        &self,
        session_id: String,
    ) -> Result<Arc<goose::agents::Agent>, StatusCode> {
        self.get_agent(session_id).await.map_err(|e| {
            tracing::error!("Failed to get agent: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }

    pub async fn get_or_create_session_stream(
        &self,
        session_id: &str,
    ) -> broadcast::Sender<MessageEvent> {
        let mut streams = self.session_streams.lock().await;
        streams
            .entry(session_id.to_string())
            .or_insert_with(|| broadcast::channel(100).0)
            .clone()
    }

    pub async fn get_session_stream(
        &self,
        session_id: &str,
    ) -> Option<broadcast::Sender<MessageEvent>> {
        let streams = self.session_streams.lock().await;
        streams.get(session_id).cloned()
    }

    pub async fn remove_session_stream(&self, session_id: &str) {
        let mut streams = self.session_streams.lock().await;
        streams.remove(session_id);
    }
}
