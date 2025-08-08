use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
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
    /// Operation to perform: "truncation" or "summarize"
    pub manage_action: String,
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
    headers: HeaderMap,
    Json(request): Json<ContextManageRequest>,
) -> Result<Json<ContextManageResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;

    let mut processed_messages = Conversation::new_unvalidated(vec![]);
    let mut token_counts: Vec<usize> = vec![];

    if request.manage_action == "truncation" {
        (processed_messages, token_counts) = agent
            .truncate_context(&request.messages)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else if request.manage_action == "summarize" {
        (processed_messages, token_counts) = agent
            .summarize_context(&request.messages)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(ContextManageResponse {
        messages: processed_messages.messages().clone(),
        token_counts,
    }))
}

// Configure routes for this module
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/context/manage", post(manage_context))
        .with_state(state)
}
