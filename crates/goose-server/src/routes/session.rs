use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch},
    Json, Router,
};
use goose::session;
use goose::{message::Message, session::SessionMetadata};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize)]
struct UpdateMetadataRequest {
    description: Option<String>,
    working_dir: Option<PathBuf>,
}

#[derive(Serialize)]
struct SessionInfo {
    id: String,
    path: String,
    modified: String,
    metadata: session::SessionMetadata,
}

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
            let metadata = session::read_metadata(&path).expect("Failed to read session metadata");

            SessionInfo {
                id,
                path: path.to_string_lossy().to_string(),
                modified,
                metadata,
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

// Update session metadata
async fn update_session_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(update): Json<UpdateMetadataRequest>,
) -> Result<Json<SessionMetadata>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let session_path = session::get_path(session::Identifier::Name(session_id));

    // Check if the session exists
    if !session_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Read current metadata
    let mut metadata = session::read_metadata(&session_path).map_err(|_| StatusCode::NOT_FOUND)?;

    // Update fields if provided
    if let Some(description) = update.description {
        metadata.description = description;
    }
    if let Some(working_dir) = update.working_dir {
        metadata.working_dir = working_dir;
    }

    // Save updated metadata
    session::update_metadata(&session_path, &metadata)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update session metadata: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(metadata))
}

// Configure routes for this module
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/:session_id", get(get_session_history))
        .route(
            "/sessions/:session_id/metadata",
            patch(update_session_metadata),
        )
        .with_state(state)
}
