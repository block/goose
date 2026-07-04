use axum::http::StatusCode;
use gosling::builtin_extension::register_builtin_extensions;
use gosling::config::Config;
use gosling::execution::manager::{AgentManager, DEFAULT_MAX_SESSION};
use gosling::session::SessionManager;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::session_event_bus::SessionEventBus;
use gosling::agents::ExtensionLoadResult;

type ExtensionLoadingTasks =
    Arc<Mutex<HashMap<String, Arc<Mutex<Option<JoinHandle<Vec<ExtensionLoadResult>>>>>>>>;

#[derive(Clone)]
pub struct AppState {
    pub(crate) agent_manager: Arc<AgentManager>,
    pub extension_loading_tasks: ExtensionLoadingTasks,
    /// Bounded like the agent LRU: SSE clients rarely call `stop_agent`, so
    /// without eviction each abandoned session would pin its bus (and up to
    /// 8 MiB of replay buffer) for the lifetime of the server. In-flight
    /// subscribers hold their own `Arc` clone and are unaffected by eviction.
    session_buses: Arc<Mutex<LruCache<String, Arc<SessionEventBus>>>>,
}

impl AppState {
    pub async fn new(_tls: bool) -> anyhow::Result<Arc<AppState>> {
        register_builtin_extensions(gosling_mcp::BUILTIN_EXTENSIONS.clone());

        let agent_manager = AgentManager::instance().await?;
        let bus_capacity = Config::global()
            .get_gosling_max_active_agents()
            .ok()
            .and_then(NonZeroUsize::new)
            .unwrap_or_else(|| NonZeroUsize::new(DEFAULT_MAX_SESSION).unwrap());
        Ok(Arc::new(Self {
            agent_manager,
            extension_loading_tasks: Arc::new(Mutex::new(HashMap::new())),
            session_buses: Arc::new(Mutex::new(LruCache::new(bus_capacity))),
        }))
    }

    pub async fn set_extension_loading_task(
        &self,
        session_id: String,
        task: JoinHandle<Vec<ExtensionLoadResult>>,
    ) {
        let mut tasks = self.extension_loading_tasks.lock().await;
        tasks.insert(session_id, Arc::new(Mutex::new(Some(task))));
    }

    pub async fn has_extension_loading_task(&self, session_id: &str) -> bool {
        let tasks = self.extension_loading_tasks.lock().await;
        tasks.contains_key(session_id)
    }

    pub async fn take_extension_loading_task(
        &self,
        session_id: &str,
    ) -> Result<Option<Vec<ExtensionLoadResult>>, tokio::task::JoinError> {
        let task_holder = {
            let tasks = self.extension_loading_tasks.lock().await;
            tasks.get(session_id).cloned()
        };

        if let Some(holder) = task_holder {
            let mut task = holder.lock().await;
            if let Some(handle) = task.as_mut() {
                // Keep the per-session task locked and discoverable while awaiting so
                // concurrent routes cannot mutate extensions before background loading finishes.
                match handle.await {
                    Ok(results) => {
                        task.take();
                        return Ok(Some(results));
                    }
                    Err(e) => {
                        task.take();
                        tracing::warn!("Background extension loading task failed: {}", e);
                        return Err(e);
                    }
                }
            }
        }
        Ok(None)
    }

    pub async fn remove_extension_loading_task(&self, session_id: &str) {
        let mut tasks = self.extension_loading_tasks.lock().await;
        tasks.remove(session_id);
    }

    pub fn session_manager(&self) -> &SessionManager {
        self.agent_manager.session_manager()
    }

    pub async fn get_or_create_event_bus(&self, session_id: &str) -> Arc<SessionEventBus> {
        let mut buses = self.session_buses.lock().await;
        buses
            .get_or_insert(session_id.to_string(), || Arc::new(SessionEventBus::new()))
            .clone()
    }

    /// Get an existing event bus for a session without creating one.
    pub async fn get_event_bus(&self, session_id: &str) -> Option<Arc<SessionEventBus>> {
        let mut buses = self.session_buses.lock().await;
        buses.get(session_id).cloned()
    }

    /// Drop a session's event bus when the session is stopped so buses (each of
    /// which retains a replay buffer of events) do not accumulate for the whole
    /// lifetime of the server. Active SSE subscribers hold their own `Arc` clone,
    /// so an in-flight stream keeps working until it finishes. Not called on
    /// restart, where the same session and its subscribers persist.
    pub async fn remove_event_bus(&self, session_id: &str) {
        let mut buses = self.session_buses.lock().await;
        buses.pop(session_id);
    }

    pub async fn get_agent(
        &self,
        session_id: String,
    ) -> anyhow::Result<Arc<gosling::agents::Agent>> {
        self.agent_manager.get_or_create_agent(session_id).await
    }

    pub async fn get_agent_for_route(
        &self,
        session_id: String,
    ) -> Result<Arc<gosling::agents::Agent>, StatusCode> {
        self.get_agent(session_id).await.map_err(|e| {
            tracing::error!("Failed to get agent: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }
}
