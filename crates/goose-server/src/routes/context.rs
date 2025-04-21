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
struct ContextRequest {
    messages: Vec<Message>,
}

// Direct message serialization for context mgmt request
#[derive(Debug, Serialize)]
struct ContextResponse {
    messages: Vec<Message>,
    token_counts: Vec<usize>,
}

async fn truncate_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ContextRequest>,
) -> Result<Json<ContextResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Get a lock on the shared agent
    let agent = state.agent.read().await;
    let agent = agent.as_ref().ok_or(StatusCode::PRECONDITION_REQUIRED)?;
    let (truncated_messages, token_counts) = agent
        .truncate_context(&payload.messages)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ContextResponse {
        messages: truncated_messages,
        token_counts,
    }))
}

async fn summarize_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ContextRequest>,
) -> Result<Json<ContextResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Get a lock on the shared agent
    let agent = state.agent.read().await;
    let agent = agent.as_ref().ok_or(StatusCode::PRECONDITION_REQUIRED)?;
    let (summarized_messages, token_counts) = agent
        .summarize_context(&payload.messages)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ContextResponse {
        messages: summarized_messages,
        token_counts,
    }))
}

// Configure routes for this module
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/context/truncate", post(truncate_handler))
        .route("/context/summarize", post(summarize_handler))
        .with_state(state)
}
