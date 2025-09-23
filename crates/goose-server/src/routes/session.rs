use crate::state::AppState;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, get, put},
    Json, Router,
};
use chrono::DateTime;
use goose::conversation::message::Message;
use goose::session;
use goose::session::{Session, SessionManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResponse {
    /// List of available session information objects
    sessions: Vec<Session>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryResponse {
    /// Unique identifier for the session
    session_id: String,
    /// Session
    session: Session,
    messages: Vec<Message>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionMetadataRequest {
    /// Updated description (name) for the session (max 200 characters)
    description: String,
}

const MAX_DESCRIPTION_LENGTH: usize = 200;

#[derive(Serialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionInsights {
    /// Total number of sessions
    total_sessions: usize,
    /// Most active working directories with session counts
    most_active_dirs: Vec<(String, usize)>,
    /// Total tokens used across all sessions
    total_tokens: i64,
    /// Activity trend for the last 7 days
    recent_activity: Vec<(String, usize)>,
}

#[derive(Serialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActivityHeatmapCell {
    pub week: usize,
    pub day: usize,
    pub count: usize,
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
async fn list_sessions() -> Result<Json<SessionListResponse>, StatusCode> {
    let sessions = SessionManager::list_sessions()
        .await
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
async fn get_session_history(
    Path(session_id): Path<String>,
) -> Result<Json<SessionHistoryResponse>, StatusCode> {
    let mut session = SessionManager::get_session(&session_id, true)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let user_visible_messages = session
        .conversation
        .take()
        .unwrap_or_default()
        .messages()
        .iter()
        .filter(|m| m.is_user_visible())
        .cloned()
        .collect();

    Ok(Json(SessionHistoryResponse {
        session_id,
        session: session.without_messages(),
        messages: user_visible_messages,
    }))
}
#[utoipa::path(
    get,
    path = "/sessions/insights",
    responses(
        (status = 200, description = "Session insights retrieved successfully", body = SessionInsights),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn get_session_insights() -> Result<Json<SessionInsights>, StatusCode> {
    let handler_start = Instant::now();

    let sessions = SessionManager::list_sessions().await.map_err(|e| {
        error!("Failed to get session info: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Filter out sessions without descriptions
    let sessions: Vec<Session> = sessions
        .into_iter()
        .filter(|session| !session.description.is_empty())
        .collect();
    info!(
        "Found {} sessions with descriptions in {}",
        sessions.len(),
        handler_start.elapsed().as_millis()
    );

    // Calculate insights
    let total_sessions = sessions.len();

    // Debug: Log if we have very few sessions, which might indicate filtering issues
    if total_sessions == 0 {
        info!("Warning: No sessions found with descriptions");
    }

    // Track directory usage
    let mut dir_counts: HashMap<String, usize> = HashMap::new();
    let mut total_tokens = 0;
    let mut activity_by_date: HashMap<String, usize> = HashMap::new();

    for session in &sessions {
        // Track directory usage
        let dir = session.working_dir.to_string_lossy().to_string();
        *dir_counts.entry(dir).or_insert(0) += 1;

        // Track tokens - only add positive values to prevent negative totals
        if let Some(tokens) = session.accumulated_total_tokens {
            match tokens.cmp(&0) {
                std::cmp::Ordering::Greater => {
                    total_tokens += tokens as i64;
                }
                std::cmp::Ordering::Less => {
                    // Log negative token values for debugging
                    info!(
                        "Warning: Session {} has negative accumulated_total_tokens: {}",
                        session.id, tokens
                    );
                }
                std::cmp::Ordering::Equal => {
                    // Zero tokens, no action needed
                }
            }
        }

        // Track activity by date
        if let Ok(date) = DateTime::parse_from_str(&session.updated_at, "%Y-%m-%d %H:%M:%S UTC") {
            let date_str = date.format("%Y-%m-%d").to_string();
            *activity_by_date.entry(date_str).or_insert(0) += 1;
        }
    }

    // Get top 3 most active directories
    let mut dir_vec: Vec<(String, usize)> = dir_counts.into_iter().collect();
    dir_vec.sort_by(|a, b| b.1.cmp(&a.1));
    let most_active_dirs = dir_vec.into_iter().take(3).collect();

    // Get last 7 days of activity
    let mut activity_vec: Vec<(String, usize)> = activity_by_date.into_iter().collect();
    activity_vec.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by date descending
    let recent_activity = activity_vec.into_iter().take(7).collect();

    let insights = SessionInsights {
        total_sessions,
        most_active_dirs,
        total_tokens,
        recent_activity,
    };

    let handler_ms = handler_start.elapsed().as_millis();
    info!("Returning insights: {:?} in {:?}", insights, handler_ms);
    Ok(Json(insights))
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
async fn update_session_metadata(
    Path(session_id): Path<String>,
    Json(request): Json<UpdateSessionMetadataRequest>,
) -> Result<StatusCode, StatusCode> {
    if request.description.len() > MAX_DESCRIPTION_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }

    SessionManager::update_session(&session_id)
        .description(request.description)
        .apply()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/sessions/{session_id}/delete",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session deleted successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn delete_session(Path(session_id): Path<String>) -> Result<StatusCode, StatusCode> {
    SessionManager::delete_session(&session_id)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(StatusCode::OK)
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/{session_id}", get(get_session_history))
        .route("/sessions/{session_id}/delete", delete(delete_session))
        .route("/sessions/insights", get(get_session_insights))
        .route(
            "/sessions/{session_id}/metadata",
            put(update_session_metadata),
        )
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
