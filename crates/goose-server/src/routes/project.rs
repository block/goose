use super::utils::verify_secret_key;
use std::sync::Arc;

use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use goose::project::{Project, ProjectMetadata};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    /// Display name of the project
    pub name: String,
    /// Optional description of the project
    pub description: Option<String>,
    /// Default working directory for sessions in this project
    #[schema(value_type = String)]
    pub default_directory: std::path::PathBuf,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    /// Display name of the project
    pub name: Option<String>,
    /// Optional description of the project
    pub description: Option<Option<String>>,
    /// Default working directory for sessions in this project
    #[schema(value_type = String)]
    pub default_directory: Option<std::path::PathBuf>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    /// List of available project metadata objects
    pub projects: Vec<ProjectMetadata>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    /// Project details
    pub project: Project,
}

#[utoipa::path(
    get,
    path = "/projects",
    responses(
        (status = 200, description = "List of available projects retrieved successfully", body = ProjectListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Project Management"
)]
// List all available projects
async fn list_projects(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ProjectListResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let projects =
        goose::project::list_projects().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProjectListResponse { projects }))
}

#[utoipa::path(
    get,
    path = "/projects/{project_id}",
    params(
        ("project_id" = String, Path, description = "Unique identifier for the project")
    ),
    responses(
        (status = 200, description = "Project details retrieved successfully", body = ProjectResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Project not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Project Management"
)]
// Get a specific project details
async fn get_project_details(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<Json<ProjectResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let project = goose::project::get_project(&project_id).map_err(|e| {
        if e.to_string().contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(ProjectResponse { project }))
}

#[utoipa::path(
    post,
    path = "/projects",
    request_body = CreateProjectRequest,
    responses(
        (status = 201, description = "Project created successfully", body = ProjectResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 400, description = "Invalid request - Bad input parameters"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Project Management"
)]
// Create a new project
async fn create_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(create_req): Json<CreateProjectRequest>,
) -> Result<Json<ProjectResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Validate input
    if create_req.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let project = goose::project::create_project(
        create_req.name,
        create_req.description,
        create_req.default_directory,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProjectResponse { project }))
}

#[utoipa::path(
    put,
    path = "/projects/{project_id}",
    params(
        ("project_id" = String, Path, description = "Unique identifier for the project")
    ),
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "Project updated successfully", body = ProjectResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Project not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Project Management"
)]
// Update a project
async fn update_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(update_req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let project = goose::project::update_project(
        &project_id,
        update_req.name,
        update_req.description,
        update_req.default_directory,
    )
    .map_err(|e| {
        if e.to_string().contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(ProjectResponse { project }))
}

#[utoipa::path(
    delete,
    path = "/projects/{project_id}",
    params(
        ("project_id" = String, Path, description = "Unique identifier for the project")
    ),
    responses(
        (status = 204, description = "Project deleted successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Project not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Project Management"
)]
// Delete a project
async fn delete_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;

    goose::project::delete_project(&project_id).map_err(|e| {
        if e.to_string().contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/projects/{project_id}/sessions/{session_id}",
    params(
        ("project_id" = String, Path, description = "Unique identifier for the project"),
        ("session_id" = String, Path, description = "Unique identifier for the session to add")
    ),
    responses(
        (status = 204, description = "Session added to project successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Project or session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Project Management"
)]
// Add session to project
async fn add_session_to_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, session_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Add the session to project
    goose::project::add_session_to_project(&project_id, &session_id).map_err(|e| {
        if e.to_string().contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Also update session metadata to include the project_id
    let session_path =
        goose::session::get_path(goose::session::Identifier::Name(session_id.clone()))
            .map_err(|_| StatusCode::NOT_FOUND)?;
    let mut metadata =
        goose::session::read_metadata(&session_path).map_err(|_| StatusCode::NOT_FOUND)?;
    metadata.project_id = Some(project_id);

    tokio::task::spawn(async move {
        if let Err(e) = goose::session::update_metadata(&session_path, &metadata).await {
            tracing::error!("Failed to update session metadata: {}", e);
        }
    });

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/projects/{project_id}/sessions/{session_id}",
    params(
        ("project_id" = String, Path, description = "Unique identifier for the project"),
        ("session_id" = String, Path, description = "Unique identifier for the session to remove")
    ),
    responses(
        (status = 204, description = "Session removed from project successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Project or session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Project Management"
)]
// Remove session from project
async fn remove_session_from_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, session_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Remove from project
    goose::project::remove_session_from_project(&project_id, &session_id).map_err(|e| {
        if e.to_string().contains("not found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Also update session metadata to remove the project_id
    let session_path =
        goose::session::get_path(goose::session::Identifier::Name(session_id.clone()))
            .map_err(|_| StatusCode::NOT_FOUND)?;
    let mut metadata =
        goose::session::read_metadata(&session_path).map_err(|_| StatusCode::NOT_FOUND)?;

    // Only update if this session was actually in this project
    if metadata.project_id.as_deref() == Some(&project_id) {
        metadata.project_id = None;

        tokio::task::spawn(async move {
            if let Err(e) = goose::session::update_metadata(&session_path, &metadata).await {
                tracing::error!("Failed to update session metadata: {}", e);
            }
        });
    }

    Ok(StatusCode::NO_CONTENT)
}

// Configure routes for this module
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/projects", get(list_projects))
        .route("/projects", post(create_project))
        .route("/projects/{project_id}", get(get_project_details))
        .route("/projects/{project_id}", put(update_project))
        .route("/projects/{project_id}", delete(delete_project))
        .route(
            "/projects/{project_id}/sessions/{session_id}",
            post(add_session_to_project),
        )
        .route(
            "/projects/{project_id}/sessions/{session_id}",
            delete(remove_session_from_project),
        )
        .with_state(state)
}
