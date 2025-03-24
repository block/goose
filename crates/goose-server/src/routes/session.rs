use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use goose::message::Message;
use goose::session;
use goose::session::info::{get_session_info, SessionInfo};
use serde::Serialize;

#[derive(Serialize)]
struct SessionListResponse {
    sessions: Vec<SessionInfo>,
}

#[derive(Serialize)]
struct SessionHistoryResponse {
    session_id: String,
    metadata: session::SessionMetadata,
    messages: Vec<Message>,
}

// List all available sessions
async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionListResponse>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let sessions = get_session_info().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SessionListResponse { sessions }))
}

// Get a specific session's history
async fn get_session_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<SessionHistoryResponse>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

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

// Configure routes for this module
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/:session_id", get(get_session_history))
        .with_state(state)
}
