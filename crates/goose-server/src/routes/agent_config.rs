use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use goose::agents::agent_config::{
    load_project_agent_config, save_routing_feedback, ProjectAgentConfig, RoutingFeedbackEntry,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

fn current_dir_or_err() -> Result<std::path::PathBuf, (StatusCode, String)> {
    std::env::current_dir().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Cannot resolve cwd: {e}"),
        )
    })
}

/// GET /agent-config — Load the current .goose/agents.yaml project config
async fn get_agent_config(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ProjectAgentConfig>, (StatusCode, String)> {
    let cwd = current_dir_or_err()?;
    let config = load_project_agent_config(&cwd).unwrap_or_default();
    Ok(Json(config))
}

/// PUT /agent-config — Save an updated .goose/agents.yaml project config
async fn put_agent_config(
    State(_state): State<Arc<AppState>>,
    Json(config): Json<ProjectAgentConfig>,
) -> Result<StatusCode, (StatusCode, String)> {
    let cwd = current_dir_or_err()?;
    let goose_dir = cwd.join(".goose");
    std::fs::create_dir_all(&goose_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create .goose dir: {e}"),
        )
    })?;

    let path = goose_dir.join("agents.yaml");
    let yaml = serde_yaml::to_string(&config).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to serialize config: {e}"),
        )
    })?;
    std::fs::write(&path, yaml).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write config: {e}"),
        )
    })?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct RecordFeedbackRequest {
    pub message: String,
    pub original_agent: String,
    pub original_mode: String,
    pub corrected_agent: String,
    pub corrected_mode: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RecordFeedbackResponse {
    pub recorded: bool,
    pub total_entries: usize,
}

/// POST /agent-config/routing-feedback — Record a routing correction
async fn record_routing_feedback(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<RecordFeedbackRequest>,
) -> Result<Json<RecordFeedbackResponse>, (StatusCode, String)> {
    let cwd = current_dir_or_err()?;
    let mut config = load_project_agent_config(&cwd).unwrap_or_default();

    let entry = RoutingFeedbackEntry {
        message: req.message,
        original_agent: req.original_agent,
        original_mode: req.original_mode,
        corrected_agent: req.corrected_agent,
        corrected_mode: req.corrected_mode,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    config.routing_feedback.push(entry);
    let total = config.routing_feedback.len();

    let saved = save_routing_feedback(&cwd, &config.routing_feedback);
    if !saved {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to save routing feedback".to_string(),
        ));
    }

    Ok(Json(RecordFeedbackResponse {
        recorded: true,
        total_entries: total,
    }))
}

/// GET /agent-config/routing-feedback — Get all routing feedback entries
async fn get_routing_feedback(
    State(_state): State<Arc<AppState>>,
) -> Json<Vec<RoutingFeedbackEntry>> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let config = load_project_agent_config(&cwd).unwrap_or_default();
    Json(config.routing_feedback)
}

pub fn agent_config_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agent-config", get(get_agent_config).put(put_agent_config))
        .route(
            "/agent-config/routing-feedback",
            get(get_routing_feedback).post(record_routing_feedback),
        )
}
