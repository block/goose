use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use goose::projects::{ProjectInfoDisplay, ProjectTracker};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsListResponse {
    /// List of tracked projects
    projects: Vec<ProjectInfoDisplay>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfoResponse {
    /// Project information
    project: ProjectInfoDisplay,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    /// Optional instruction to associate with this project
    instruction: Option<String>,
    /// Optional session ID to associate with this project
    session_id: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectResponse {
    /// Success message
    message: String,
}

#[utoipa::path(
    get,
    path = "/projects",
    responses(
        (status = 200, description = "List of projects retrieved successfully", body = ProjectsListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Projects Management"
)]
async fn list_projects() -> Result<Json<ProjectsListResponse>, StatusCode> {
    let tracker = ProjectTracker::load().map_err(|e| {
        tracing::error!("Failed to load projects tracker: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut projects = tracker.list_projects();

    // Sort by last accessed (newest first)
    projects.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

    Ok(Json(ProjectsListResponse { projects }))
}

#[utoipa::path(
    get,
    path = "/projects/{path}",
    params(
        ("path" = String, Path, description = "Project directory path")
    ),
    responses(
        (status = 200, description = "Project information retrieved successfully", body = ProjectInfoResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Project not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Projects Management"
)]
async fn get_project(Path(path): Path<String>) -> Result<Json<ProjectInfoResponse>, StatusCode> {
    let tracker = ProjectTracker::load().map_err(|e| {
        tracing::error!("Failed to load projects tracker: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let project = tracker.get_project(&path).ok_or_else(|| {
        tracing::debug!("Project not found: {}", path);
        StatusCode::NOT_FOUND
    })?;

    Ok(Json(ProjectInfoResponse { project }))
}

#[utoipa::path(
    delete,
    path = "/projects/{path}",
    params(
        ("path" = String, Path, description = "Project directory path")
    ),
    responses(
        (status = 200, description = "Project removed successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Project not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Projects Management"
)]
async fn remove_project(Path(path): Path<String>) -> Result<Json<String>, StatusCode> {
    let mut tracker = ProjectTracker::load().map_err(|e| {
        tracing::error!("Failed to load projects tracker: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let removed = tracker.remove_project(&path).map_err(|e| {
        tracing::error!("Failed to remove project: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if removed {
        Ok(Json(format!("Project '{}' removed successfully", path)))
    } else {
        Ok(Json(format!("Project '{}' not found", path)))
    }
}

#[utoipa::path(
    post,
    path = "/projects",
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "Project updated successfully", body = UpdateProjectResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Projects Management"
)]
async fn update_current_project(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<UpdateProjectRequest>,
) -> Result<Json<UpdateProjectResponse>, StatusCode> {
    goose::projects::update_project_tracker(
        request.instruction.as_deref(),
        request.session_id.as_deref(),
    )
    .map_err(|e| {
        tracing::error!("Failed to update project tracker: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(UpdateProjectResponse {
        message: "Project updated successfully".to_string(),
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/projects", get(list_projects).post(update_current_project))
        .route("/projects/{path}", get(get_project).delete(remove_project))
        .with_state(state)
}
