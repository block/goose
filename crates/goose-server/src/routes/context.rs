use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::conversation::{message::Message, Conversation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

/// Request payload for context management operations
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextManageRequest {
    /// Collection of messages to be managed
    pub messages: Vec<Message>,
    /// Optional session ID for session-specific agent
    pub session_id: String,
}

/// Response from context management operations
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextManageResponse {
    /// Processed messages after the operation
    pub messages: Vec<Message>,
    /// Token counts for each processed message
    pub token_counts: Vec<usize>,
}

#[utoipa::path(
    post,
    path = "/context/manage",
    request_body = ContextManageRequest,
    responses(
        (status = 200, description = "Context managed successfully", body = ContextManageResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 412, description = "Precondition failed - Agent not available"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Context Management"
)]
async fn manage_context(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ContextManageRequest>,
) -> Result<Json<ContextManageResponse>, StatusCode> {
    let agent = state.get_agent_for_route(request.session_id).await?;

    let conversation = Conversation::new_unvalidated(request.messages);

    let (processed_messages, token_counts, _) =
        goose::context_mgmt::compact_messages(&agent, &conversation, false)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // TODO(Douwe): store into db

    Ok(Json(ContextManageResponse {
        messages: processed_messages.messages().to_vec(),
        token_counts,
    }))
}

// Configure routes for this module
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/context/manage", post(manage_context))
        .with_state(state)
}
