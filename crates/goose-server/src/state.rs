use axum::http::StatusCode;
use goose::execution::manager::AgentManager;
use goose::scheduler_trait::SchedulerTrait;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::Mutex;
use goose::agents::Agent;
use goose::config::Config;
use goose::providers::create_with_named_model;
use crate::routes::errors::ErrorResponse;

#[derive(Clone)]
pub struct AppState {
    pub(crate) agent_manager: Arc<AgentManager>,
    pub next_agent: Arc<Agent>,
    pub recipe_file_hash_map: Arc<Mutex<HashMap<String, PathBuf>>>,
    pub session_counter: Arc<AtomicUsize>,
    /// Tracks sessions that have already emitted recipe telemetry to prevent double counting.
    recipe_session_tracker: Arc<Mutex<HashSet<String>>>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Arc<AppState>> {
        let agent_manager = AgentManager::instance().await?;
        let default_agent = agent_manager.get_or_create_agent(new_session);
        Self::configure_agent_with_defaults(default_agent).await;
        Ok(Arc::new(Self {
            agent_manager,
            recipe_file_hash_map: Arc::new(Mutex::new(HashMap::new())),
            session_counter: Arc::new(AtomicUsize::new(0)),
            recipe_session_tracker: Arc::new(Mutex::new(HashSet::new())),
            next_agent: Arc::new(default_agent),
        }))
    }

    pub async fn configure_agent_with_defaults(agent: Agent) -> Result<(), ErrorResponse> {
        let config = Config::global();

            let provider_name: String =
                config
                    .get_param("GOOSE_PROVIDER")
                    .map_err(|_| ErrorResponse {
                        message: "Could not configure agent: missing provider".into(),
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                    })?;

            let model: String = config.get_param("GOOSE_MODEL").map_err(|_| ErrorResponse {
                message: "Could not configure agent: missing model".into(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            })?;

            let provider = create_with_named_model(&provider_name, &model)
                .await
                .map_err(|_| ErrorResponse {
                    message: "Could not configure agent: missing model".into(),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                })?;

            agent
                .update_provider(provider)
                .await
                .map_err(|e| ErrorResponse {
                    message: format!("Could not configure agent: {}", e),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                })}
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

    /// Get agent for route handlers - always uses Interactive mode and converts any error to 500
    pub async fn get_agent_for_route(
        &self,
        session_id: String,
    ) -> Result<Arc<goose::agents::Agent>, StatusCode> {
        self.get_agent(session_id).await.map_err(|e| {
            tracing::error!("Failed to get agent: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }
}
