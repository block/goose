use super::utils::verify_secret_key;
use chrono::DateTime;
use std::collections::HashMap;
use std::sync::Arc;

use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use goose::conversation::message::{BranchReference, BranchSource, BranchingMetadata, Message};
use goose::conversation::Conversation;
use goose::session;
use goose::session::info::{get_valid_sorted_sessions, SessionInfo, SortOrder};
use goose::session::SessionMetadata;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
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

const MAX_DESCRIPTION_LENGTH: usize = 200;

#[derive(Serialize, ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionInsights {
    /// Total number of sessions
    total_sessions: usize,
    /// Most active working directories with session counts
    most_active_dirs: Vec<(String, usize)>,
    /// Average session duration in minutes
    avg_session_duration: f64,
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
            error!("Failed to read session messages: {:?}", e);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    Ok(Json(SessionHistoryResponse {
        session_id,
        metadata,
        messages: messages.messages().clone(),
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
async fn get_session_insights(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionInsights>, StatusCode> {
    info!("Received request for session insights");

    verify_secret_key(&headers, &state)?;

    let sessions = get_valid_sorted_sessions(SortOrder::Descending).map_err(|e| {
        error!("Failed to get session info: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Filter out sessions without descriptions
    let sessions: Vec<SessionInfo> = sessions
        .into_iter()
        .filter(|session| !session.metadata.description.is_empty())
        .collect();

    info!("Found {} sessions with descriptions", sessions.len());

    // Calculate insights
    let total_sessions = sessions.len();

    // Debug: Log if we have very few sessions, which might indicate filtering issues
    if total_sessions == 0 {
        info!("Warning: No sessions found with descriptions");
    }

    // Track directory usage
    let mut dir_counts: HashMap<String, usize> = HashMap::new();
    let mut total_duration = 0.0;
    let mut total_tokens = 0;
    let mut activity_by_date: HashMap<String, usize> = HashMap::new();

    for session in &sessions {
        // Track directory usage
        let dir = session.metadata.working_dir.to_string_lossy().to_string();
        *dir_counts.entry(dir).or_insert(0) += 1;

        // Track tokens - only add positive values to prevent negative totals
        if let Some(tokens) = session.metadata.accumulated_total_tokens {
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
        if let Ok(date) = DateTime::parse_from_str(&session.modified, "%Y-%m-%d %H:%M:%S UTC") {
            let date_str = date.format("%Y-%m-%d").to_string();
            *activity_by_date.entry(date_str).or_insert(0) += 1;
        }

        // Calculate session duration from messages
        let session_path = session::get_path(session::Identifier::Name(session.id.clone()));
        if let Ok(session_path) = session_path {
            if let Ok(messages) = session::read_messages(&session_path) {
                if let (Some(first), Some(last)) = (messages.first(), messages.last()) {
                    let duration = (last.created - first.created) as f64 / 60.0; // Convert to minutes
                    total_duration += duration;
                }
            }
        }
    }

    // Get top 3 most active directories
    let mut dir_vec: Vec<(String, usize)> = dir_counts.into_iter().collect();
    dir_vec.sort_by(|a, b| b.1.cmp(&a.1));
    let most_active_dirs = dir_vec.into_iter().take(3).collect();

    // Calculate average session duration
    let avg_session_duration = if total_sessions > 0 {
        total_duration / total_sessions as f64
    } else {
        0.0
    };

    // Get last 7 days of activity
    let mut activity_vec: Vec<(String, usize)> = activity_by_date.into_iter().collect();
    activity_vec.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by date descending
    let recent_activity = activity_vec.into_iter().take(7).collect();

    let insights = SessionInsights {
        total_sessions,
        most_active_dirs,
        avg_session_duration,
        total_tokens,
        recent_activity,
    };

    info!("Returning insights: {:?}", insights);
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
    let mut metadata = session::read_metadata(&session_path).map_err(|_| StatusCode::NOT_FOUND)?;

    // Update description
    metadata.description = request.description;

    // Save updated metadata
    session::update_metadata(&session_path, &metadata)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Clean up branching metadata references to a deleted session from all other sessions
async fn cleanup_branching_references(
    deleted_session_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get all valid sessions
    let sessions = get_valid_sorted_sessions(SortOrder::Descending)?;

    for session_info in sessions {
        // Skip if this is the deleted session (shouldn't happen since it's already deleted, but safety check)
        if session_info.id == deleted_session_id {
            continue;
        }

        let session_path = session::get_path(session::Identifier::Name(session_info.id.clone()))?;

        // Read the session messages
        let messages_result = session::read_messages(&session_path);
        let messages = match messages_result {
            Ok(messages) => messages,
            Err(_) => {
                // If we can't read the session, skip it
                continue;
            }
        };

        let mut messages_vec = messages.messages().clone();
        let mut session_modified = false;

        // Check each message for branching metadata that references the deleted session
        for message in &mut messages_vec {
            if let Some(ref mut metadata) = message.branching_metadata {
                // Remove references from branches_created
                let original_count = metadata.branches_created.len();
                metadata
                    .branches_created
                    .retain(|branch_ref| branch_ref.session_id != deleted_session_id);
                if metadata.branches_created.len() != original_count {
                    session_modified = true;
                }

                // Remove reference from branched_from if it matches the deleted session
                if let Some(ref branch_source) = metadata.branched_from {
                    if branch_source.session_id == deleted_session_id {
                        metadata.branched_from = None;
                        session_modified = true;
                    }
                }

                // If metadata is now empty, remove it entirely
                if metadata.branches_created.is_empty() && metadata.branched_from.is_none() {
                    message.branching_metadata = None;
                    session_modified = true;
                }
            }
        }

        // If we modified the session, save it back
        if session_modified {
            let updated_messages_container = Conversation::new_unvalidated(messages_vec);
            session::persist_messages(&session_path, &updated_messages_container, None, None)
                .await?;
            info!(
                "Cleaned up branching references to session {} from session {}",
                deleted_session_id, session_info.id
            );
        }
    }

    Ok(())
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
// Delete a session
async fn delete_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Get the session path
    let session_path = match session::get_path(session::Identifier::Name(session_id.clone())) {
        Ok(path) => path,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    // Check if session file exists
    if !session_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Delete the session file
    std::fs::remove_file(&session_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Clean up branching metadata references to the deleted session
    if let Err(e) = cleanup_branching_references(&session_id).await {
        error!(
            "Failed to cleanup branching references for session {}: {:?}",
            session_id, e
        );
        // Don't fail the deletion if cleanup fails - the session is already deleted
        // This is a best-effort cleanup operation
    }

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/sessions/{session_id}/branch",
    request_body = BranchSessionRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session to branch from")
    ),
    responses(
        (status = 200, description = "Session branched successfully", body = BranchSessionResponse),
        (status = 400, description = "Bad request - Invalid message index or description too long"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
// Branch a session from a specific message
async fn branch_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(request): Json<BranchSessionRequest>,
) -> Result<Json<BranchSessionResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Validate description length if provided
    if let Some(ref description) = request.description {
        if description.len() > MAX_DESCRIPTION_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Get the source session path
    let source_session_path = session::get_path(session::Identifier::Name(session_id.clone()))
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Read source session data
    let source_metadata =
        session::read_metadata(&source_session_path).map_err(|_| StatusCode::NOT_FOUND)?;

    let source_messages =
        session::read_messages(&source_session_path).map_err(|_| StatusCode::NOT_FOUND)?;

    // Validate message index
    if request.message_index >= source_messages.len() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create new session ID
    let branch_session_id = session::generate_session_id();

    // Copy messages up to and including the specified index
    let mut branch_messages: Vec<Message> = source_messages
        .messages()
        .iter()
        .take(request.message_index + 1)
        .cloned()
        .collect();

    // Add branching metadata to the original session's messages
    let mut updated_source_messages = source_messages.messages().clone();
    if let Some(original_message) = updated_source_messages.get_mut(request.message_index) {
        // Initialize branching metadata if it doesn't exist
        if original_message.branching_metadata.is_none() {
            original_message.branching_metadata = Some(BranchingMetadata::default());
        }

        // Add the new branch reference
        if let Some(ref mut metadata) = original_message.branching_metadata {
            metadata.branches_created.push(BranchReference {
                session_id: branch_session_id.clone(),
                description: request.description.clone(),
            });
        }
    }

    // Add branching metadata to the branch session's message at the branch point
    if let Some(branch_message) = branch_messages.get_mut(request.message_index) {
        branch_message.branching_metadata = Some(BranchingMetadata {
            branches_created: Vec::new(),
            branched_from: Some(BranchSource {
                session_id: session_id.clone(),
                message_index: request.message_index,
                description: request.description.clone(),
            }),
        });
    }

    // Create branch session metadata
    let branch_description = request
        .description
        .unwrap_or_else(|| format!("Branch from {}", session_id));

    let mut branch_metadata = source_metadata.clone();
    branch_metadata.description = branch_description;

    // Create the branch session
    let branch_session_path =
        session::get_path(session::Identifier::Name(branch_session_id.clone()))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Save branch session
    let branch_messages_container = Conversation::new_unvalidated(branch_messages);
    session::persist_messages(&branch_session_path, &branch_messages_container, None, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    session::update_metadata(&branch_session_path, &branch_metadata)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update the original session with branching metadata
    let updated_source_messages_container = Conversation::new_unvalidated(updated_source_messages);
    session::persist_messages(
        &source_session_path,
        &updated_source_messages_container,
        None,
        None,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(BranchSessionResponse { branch_session_id }))
}

// Configure routes for this module
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
        .route("/sessions/{session_id}/branch", post(branch_session))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

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

    #[tokio::test]
    async fn test_branch_session_request_deserialization() {
        // Test basic request
        let json = r#"{"messageIndex": 5}"#;
        let request: BranchSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.message_index, 5);
        assert_eq!(request.description, None);

        // Test request with description
        let json = r#"{"messageIndex": 3, "description": "My branch"}"#;
        let request: BranchSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.message_index, 3);
        assert_eq!(request.description, Some("My branch".to_string()));
    }

    #[tokio::test]
    async fn test_branch_session_response_serialization() {
        let response = BranchSessionResponse {
            branch_session_id: "test-session-123".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("branchSessionId"));
        assert!(json.contains("test-session-123"));
    }

    #[tokio::test]
    async fn test_branch_session_with_metadata() {
        use goose::conversation::message::Message;
        use goose::conversation::Conversation;

        // Create test messages
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
            Message::user().with_text("How are you?"),
            Message::assistant().with_text("I'm doing well, thanks!"),
        ];

        // Test the branching logic without file operations
        let message_index = 2; // Branch from the second user message
        let branch_description = Some("Test branch".to_string());

        // Create branch session ID
        let source_session_id = goose::session::generate_session_id();
        let branch_session_id = goose::session::generate_session_id();

        // Copy messages up to and including the specified index
        let mut branch_messages: Vec<Message> =
            messages.iter().take(message_index + 1).cloned().collect();

        // Add branching metadata to the original session's messages
        let mut updated_source_messages = messages.clone();
        if let Some(original_message) = updated_source_messages.get_mut(message_index) {
            original_message.branching_metadata = Some(BranchingMetadata {
                branches_created: vec![BranchReference {
                    session_id: branch_session_id.clone(),
                    description: branch_description.clone(),
                }],
                branched_from: None,
            });
        }

        // Add branching metadata to the branch session's message at the branch point
        if let Some(branch_message) = branch_messages.get_mut(message_index) {
            branch_message.branching_metadata = Some(BranchingMetadata {
                branches_created: Vec::new(),
                branched_from: Some(BranchSource {
                    session_id: source_session_id.clone(),
                    message_index,
                    description: branch_description.clone(),
                }),
            });
        }

        // Verify the branch messages
        assert_eq!(branch_messages.len(), 3); // Should have 3 messages (up to and including index 2)

        // Verify the last message has branching metadata indicating it was branched from source
        let last_branch_message = branch_messages.last().unwrap();
        assert!(last_branch_message.branching_metadata.is_some());

        let branch_metadata = last_branch_message.branching_metadata.as_ref().unwrap();
        assert!(branch_metadata.branched_from.is_some());

        let branch_source = branch_metadata.branched_from.as_ref().unwrap();
        assert_eq!(branch_source.session_id, source_session_id);
        assert_eq!(branch_source.message_index, 2);
        assert_eq!(branch_source.description, Some("Test branch".to_string()));

        // Verify the original session has branching metadata
        let original_message = &updated_source_messages[message_index];
        assert!(original_message.branching_metadata.is_some());

        let original_metadata = original_message.branching_metadata.as_ref().unwrap();
        let branches = &original_metadata.branches_created;
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].session_id, branch_session_id);
        assert_eq!(branches[0].description, Some("Test branch".to_string()));

        // Test serialization/deserialization of branching metadata
        let serialized = serde_json::to_string(&last_branch_message).unwrap();
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();

        assert!(deserialized.branching_metadata.is_some());
        let deserialized_metadata = deserialized.branching_metadata.as_ref().unwrap();
        assert!(deserialized_metadata.branched_from.is_some());

        let deserialized_source = deserialized_metadata.branched_from.as_ref().unwrap();
        assert_eq!(deserialized_source.session_id, source_session_id);
        assert_eq!(deserialized_source.message_index, 2);
        assert_eq!(
            deserialized_source.description,
            Some("Test branch".to_string())
        );
    }

    #[tokio::test]
    async fn test_cleanup_branching_references_function() {
        use goose::conversation::message::Message;
        use tokio::time::{sleep, Duration};
        use uuid::Uuid;

        // Add small delay to prevent file locking conflicts
        sleep(Duration::from_millis(10)).await;

        let deleted_session_id = format!("deleted-session-{}", Uuid::new_v4());
        let keep_session_id = format!("session-to-keep-{}", Uuid::new_v4());
        let test_session_id = Uuid::new_v4().to_string();

        // Create a test session with branching metadata that references the deleted session
        let mut messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
        ];

        messages[0].branching_metadata = Some(BranchingMetadata {
            branches_created: vec![
                BranchReference {
                    session_id: deleted_session_id.to_string(),
                    description: Some("Branch to be cleaned up".to_string()),
                },
                BranchReference {
                    session_id: keep_session_id.to_string(),
                    description: Some("Branch to keep".to_string()),
                },
            ],
            branched_from: None,
        });

        messages[1].branching_metadata = Some(BranchingMetadata {
            branches_created: Vec::new(),
            branched_from: Some(BranchSource {
                session_id: deleted_session_id.to_string(),
                message_index: 0,
                description: Some("Branched from deleted session".to_string()),
            }),
        });

        // Create the test session using the proper session creation method
        let conversation = Conversation::new_unvalidated(messages);
        let session_path = session::get_path(session::Identifier::Name(test_session_id)).unwrap();
        session::persist_messages(&session_path, &conversation, None, None)
            .await
            .unwrap();

        // Call the actual cleanup function
        cleanup_branching_references(&deleted_session_id)
            .await
            .unwrap();

        // Read back the session and verify cleanup worked
        let updated_messages = session::read_messages(&session_path).unwrap();
        let messages_vec = updated_messages.messages();

        // Check first message - should have one branch reference remaining
        if let Some(ref metadata) = messages_vec[0].branching_metadata {
            assert_eq!(metadata.branches_created.len(), 1);
            assert_eq!(metadata.branches_created[0].session_id, keep_session_id);
            assert_eq!(
                metadata.branches_created[0].description,
                Some("Branch to keep".to_string())
            );
            assert!(metadata.branched_from.is_none());
        } else {
            panic!("Expected branching metadata on first message");
        }

        // Check second message - should have no metadata since branched_from was removed and branches_created was empty
        assert!(
            messages_vec[1].branching_metadata.is_none(),
            "Second message should have no metadata after cleanup"
        );

        // Clean up the test session
        let _ = std::fs::remove_file(&session_path);
    }

    #[tokio::test]
    async fn test_cleanup_branching_references_with_empty_meta_data() {
        use goose::conversation::message::Message;
        use tokio::time::{sleep, Duration};
        use uuid::Uuid;

        // Add small delay to prevent file locking conflicts
        sleep(Duration::from_millis(20)).await;

        let deleted_session_id = format!("deleted-session-empty-{}", Uuid::new_v4());
        let test_session_id = Uuid::new_v4().to_string();

        // Test message with empty branching metadata
        let mut message_with_empty_metadata = Message::user().with_text("Test");
        message_with_empty_metadata.branching_metadata = Some(BranchingMetadata {
            branches_created: Vec::new(),
            branched_from: None,
        });

        // Create a test with empty branching metadata
        let messages = vec![message_with_empty_metadata];

        // Create the test session using the proper session creation method
        let conversation = Conversation::new_unvalidated(messages);
        let session_path = session::get_path(session::Identifier::Name(test_session_id)).unwrap();
        session::persist_messages(&session_path, &conversation, None, None)
            .await
            .unwrap();

        // Call the function under test
        cleanup_branching_references(&deleted_session_id)
            .await
            .unwrap();

        // Read back the session and verify cleanup worked
        let updated_messages = session::read_messages(&session_path).unwrap();
        let messages_vec = updated_messages.messages();

        assert!(messages_vec[0].branching_metadata.is_none());

        // Clean up the test session
        let _ = std::fs::remove_file(&session_path);
    }

    #[tokio::test]
    async fn test_cleanup_branching_references_with_no_meta_data() {
        use goose::conversation::message::Message;

        let deleted_session_id = "deleted-session-no-meta";
        let test_session_id = Uuid::new_v4().to_string();

        // Create a test with empty branching metadata
        let messages = vec![
            // Message with no meta-data
            Message::user().with_text("Hello"),
        ];

        // Create the test session using the proper session creation method
        let conversation = Conversation::new_unvalidated(messages);
        let session_path = session::get_path(session::Identifier::Name(test_session_id)).unwrap();
        session::persist_messages(&session_path, &conversation, None, None)
            .await
            .unwrap();

        // Call the function under test
        cleanup_branching_references(&deleted_session_id)
            .await
            .unwrap();

        // Read back the session and verify cleanup worked
        let updated_messages = session::read_messages(&session_path).unwrap();
        let messages_vec = updated_messages.messages();

        assert!(messages_vec[0].branching_metadata.is_none());

        // Clean up the test session
        let _ = std::fs::remove_file(&session_path);
    }
}
