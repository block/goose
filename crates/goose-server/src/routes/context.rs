use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use goose::message::Message;
use serde::{Deserialize, Serialize};

// Direct message serialization for context mgmt request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextManageRequest {
    messages: Vec<Message>,
    manage_action: String,
}

// Direct message serialization for context mgmt request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextManageResponse {
    messages: Vec<Message>,
    token_counts: Vec<usize>,
}

async fn manage_context(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ContextManageRequest>,
) -> Result<Json<ContextManageResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Get a lock on the shared agent
    let agent = state.agent.read().await;
    let agent = agent.as_ref().ok_or(StatusCode::PRECONDITION_REQUIRED)?;

    let mut processed_messages: Vec<Message> = vec![];
    let mut token_counts: Vec<usize> = vec![];
    if request.manage_action == "trunction" {
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
        messages: processed_messages,
        token_counts,
    }))
}

// Configure routes for this module
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/context/manage", post(manage_context))
        .with_state(state)
}
