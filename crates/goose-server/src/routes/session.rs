use super::utils::verify_secret_key;
use std::sync::Arc;

use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use goose::message::Message;
use goose::session;
use goose::session::info::{get_session_info, SessionInfo, SortOrder};
use goose::session::SessionMetadata;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResponse {
    /// List of available session information objects
    sessions: Vec<SessionInfo>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryResponse {
    /// Unique identifier for the session
    session_id: String,
    /// Session metadata containing creation time and other details
    metadata: SessionMetadata,
    /// List of messages in the session conversation
    messages: Vec<Message>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BranchSessionRequest {
    /// Index of the message to branch from (inclusive)
    message_index: usize,
    /// Optional description for the new branch
    description: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BranchSessionResponse {
    /// ID of the newly created branch session
    branch_session_id: String,
}

#[utoipa::path(
    get,
    path = "/sessions",
    responses(
        (status = 200, description = "List of available sessions retrieved successfully", body = SessionListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
// List all available sessions
async fn list_sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionListResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let sessions =
        get_session_info(SortOrder::Descending).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SessionListResponse { sessions }))
}

#[utoipa::path(
    get,
    path = "/sessions/{session_id}",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session history retrieved successfully", body = SessionHistoryResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
// Get a specific session's history
async fn get_session_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<SessionHistoryResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let session_path = session::get_path(session::Identifier::Name(session_id.clone()));

    // Read metadata
    let metadata = session::read_metadata(&session_path).map_err(|_| StatusCode::NOT_FOUND)?;

    let messages = match session::read_messages(&session_path) {
        Ok(messages) => messages,
        Err(e) => {
            tracing::error!("Failed to read session messages: {:?}", e);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    Ok(Json(SessionHistoryResponse {
        session_id,
        metadata,
        messages,
    }))
}

#[utoipa::path(
    post,
    path = "/sessions/{session_id}/branch",
    request_body = BranchSessionRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the source session")
    ),
    responses(
        (status = 200, description = "Session branch created successfully", body = BranchSessionResponse),
        (status = 400, description = "Invalid request - message index out of range"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Source session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
// Create a branch from an existing session
async fn branch_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(request): Json<BranchSessionRequest>,
) -> Result<Json<BranchSessionResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Branch the session
    let branch_session_id = match session::branch_session(
        &session_id,
        request.message_index,
        request.description,
    ) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to branch session: {:?}", e);
            if e.to_string().contains("out of range") {
                return Err(StatusCode::BAD_REQUEST);
            } else if e.to_string().contains("not found") {
                return Err(StatusCode::NOT_FOUND);
            } else {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    };

    Ok(Json(BranchSessionResponse { branch_session_id }))
}

// Configure routes for this module
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/{session_id}", get(get_session_history))
        .route("/sessions/{session_id}/branch", post(branch_session))
        .with_state(state)
}
