use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use goose::message::Message;
use goose::session;
use serde::Serialize;

#[derive(Serialize)]
struct SessionInfo {
    id: String,
    path: String,
    modified: String,
    description: String,
}

#[derive(Serialize)]
struct SessionListResponse {
    sessions: Vec<SessionInfo>,
}

#[derive(Serialize)]
struct SessionHistoryResponse {
    session_id: String,
    description: String,
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

    let sessions = match session::list_sessions() {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!("Failed to list sessions: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let session_infos = sessions
        .into_iter()
        .map(|(id, path)| {
            // Get last modified time as string
            let modified = path
                .metadata()
                .and_then(|m| m.modified())
                .map(|time| {
                    chrono::DateTime::<chrono::Utc>::from(time)
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string()
                })
                .unwrap_or_else(|_| "Unknown".to_string());
            
            // Get session description
            let description = session::read_metadata(&path)
                .map(|meta| meta.description)
                .unwrap_or_else(|_| String::new());

            SessionInfo {
                id,
                path: path.to_string_lossy().to_string(),
                modified,
                description,
            }
        })
        .collect();

    Ok(Json(SessionListResponse {
        sessions: session_infos,
    }))
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
    let description = match session::read_metadata(&session_path) {
        Ok(metadata) => metadata.description,
        Err(e) => {
            tracing::error!("Failed to read session metadata: {:?}", e);
            String::new()
        }
    };

    let messages = match session::read_messages(&session_path) {
        Ok(messages) => messages,
        Err(e) => {
            tracing::error!("Failed to read session messages: {:?}", e);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    Ok(Json(SessionHistoryResponse {
        session_id,
        description,
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