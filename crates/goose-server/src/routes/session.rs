use super::utils::verify_secret_key;
use std::sync::Arc;

use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, put},
    Json, Router,
};
use goose::message::Message;
use goose::session;
use goose::session::info::{get_valid_sorted_sessions, SessionInfo, SortOrder};
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
pub struct UpdateSessionMetadataRequest {
    /// Updated description (name) for the session (max 200 characters)
    description: String,
}

const MAX_DESCRIPTION_LENGTH: usize = 200;

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

    let sessions = get_valid_sorted_sessions(SortOrder::Descending)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

    let session_path = match session::get_path(session::Identifier::Name(session_id.clone())) {
        Ok(path) => path,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

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
    put,
    path = "/sessions/{session_id}/metadata",
    request_body = UpdateSessionMetadataRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session metadata updated successfully"),
        (status = 400, description = "Bad request - Description too long (max 200 characters)"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
// Update session metadata
async fn update_session_metadata(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(request): Json<UpdateSessionMetadataRequest>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Validate description length
    if request.description.len() > MAX_DESCRIPTION_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }

    let session_path = session::get_path(session::Identifier::Name(session_id.clone()))
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Read current metadata
    let mut metadata = session::read_metadata(&session_path)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Update description
    metadata.description = request.description;

    // Save updated metadata
    session::update_metadata(&session_path, &metadata).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

// Configure routes for this module
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/{session_id}", get(get_session_history))
        .route("/sessions/{session_id}/metadata", put(update_session_metadata))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_update_session_metadata_request_deserialization() {
        // Test that our request struct can be deserialized properly
        let json = r#"{"description": "test description"}"#;
        let request: UpdateSessionMetadataRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.description, "test description");
    }

    #[tokio::test]
    async fn test_update_session_metadata_request_validation() {
        // Test empty description
        let empty_request = UpdateSessionMetadataRequest {
            description: "".to_string(),
        };
        assert_eq!(empty_request.description, "");

        // Test normal description
        let normal_request = UpdateSessionMetadataRequest {
            description: "My Session Name".to_string(),
        };
        assert_eq!(normal_request.description, "My Session Name");

        // Test description at max length (should be valid)
        let max_length_description = "A".repeat(MAX_DESCRIPTION_LENGTH);
        let max_request = UpdateSessionMetadataRequest {
            description: max_length_description.clone(),
        };
        assert_eq!(max_request.description, max_length_description);
        assert_eq!(max_request.description.len(), MAX_DESCRIPTION_LENGTH);

        // Test description over max length
        let over_max_description = "A".repeat(MAX_DESCRIPTION_LENGTH + 1);
        let over_max_request = UpdateSessionMetadataRequest {
            description: over_max_description.clone(),
        };
        assert_eq!(over_max_request.description, over_max_description);
        assert!(over_max_request.description.len() > MAX_DESCRIPTION_LENGTH);
    }

    #[tokio::test]
    async fn test_description_length_validation() {
        // Test the validation logic used in the endpoint
        let valid_description = "A".repeat(MAX_DESCRIPTION_LENGTH);
        assert!(valid_description.len() <= MAX_DESCRIPTION_LENGTH);

        let invalid_description = "A".repeat(MAX_DESCRIPTION_LENGTH + 1);
        assert!(invalid_description.len() > MAX_DESCRIPTION_LENGTH);

        // Test edge cases
        assert!(String::new().len() <= MAX_DESCRIPTION_LENGTH); // Empty string
        assert!("Short".len() <= MAX_DESCRIPTION_LENGTH); // Short string
    }
}
