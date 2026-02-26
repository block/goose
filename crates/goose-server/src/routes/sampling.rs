use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use goose::conversation::message::Message;
use rmcp::model::{
    CreateMessageRequestParams, CreateMessageResult, Role, SamplingContent, SamplingMessage,
    SamplingMessageContent,
};
use std::sync::Arc;

use crate::state::AppState;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/sessions/{session_id}/sampling/message",
            post(create_message),
        )
        .with_state(state)
}

async fn create_message(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<CreateMessageRequestParams>,
) -> Result<Json<CreateMessageResult>, StatusCode> {
    let agent = state.get_agent_for_route(session_id.clone()).await?;

    let provider = agent.provider().await.map_err(|e| {
        tracing::error!("Failed to get provider: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let messages: Vec<Message> = request
        .messages
        .iter()
        .map(|msg| {
            let base = match msg.role {
                Role::User => Message::user(),
                Role::Assistant => Message::assistant(),
                _ => Message::user(),
            };
            content_to_message(base, &msg.content)
        })
        .collect();

    let system = request
        .system_prompt
        .as_deref()
        .unwrap_or("You are a helpful AI assistant.");

    let model_config = provider.get_model_config();
    let (response, usage) = provider
        .complete(&model_config, &session_id, system, &messages, &[])
        .await
        .map_err(|e| {
            tracing::error!("Sampling completion failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let text = response.as_concat_text();

    Ok(Json(CreateMessageResult {
        model: usage.model,
        stop_reason: Some(CreateMessageResult::STOP_REASON_END_TURN.to_string()),
        message: SamplingMessage::new(Role::Assistant, SamplingMessageContent::text(&text)),
    }))
}

fn content_to_message(base: Message, content: &SamplingContent<SamplingMessageContent>) -> Message {
    let items = match content {
        SamplingContent::Single(item) => vec![item],
        SamplingContent::Multiple(items) => items.iter().collect(),
        _ => vec![],
    };

    let mut msg = base;
    for item in items {
        msg = match item {
            SamplingMessageContent::Text(text) => msg.with_text(&text.text),
            SamplingMessageContent::Image(image) => msg.with_image(&image.data, &image.mime_type),
            _ => msg,
        };
    }
    msg
}
