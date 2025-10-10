use crate::state::AppState;
use axum::routing::post;
use axum::{
    extract::{Path, State},
    http::{self, StatusCode},
    response::IntoResponse,
    routing::{delete, get, put},
    Json, Router,
};
use bytes::Bytes;
use futures::Stream;
use goose::session::session_manager::SessionInsights;
use goose::session::{Session, SessionManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResponse {
    /// List of available session information objects
    sessions: Vec<Session>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionDescriptionRequest {
    /// Updated description (name) for the session (max 200 characters)
    description: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionUserRecipeValuesRequest {
    /// Recipe parameter values entered by the user
    user_recipe_values: HashMap<String, String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportSessionRequest {
    json: String,
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
        (status = 200, description = "Session history retrieved successfully", body = Session),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn get_session(Path(session_id): Path<String>) -> Result<Json<Session>, StatusCode> {
    let session = SessionManager::get_session(&session_id, true)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(session))
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
    let insights = SessionManager::get_insights()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(insights))
}

#[utoipa::path(
    put,
    path = "/sessions/{session_id}/description",
    request_body = UpdateSessionDescriptionRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session description updated successfully"),
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
async fn update_session_description(
    Path(session_id): Path<String>,
    Json(request): Json<UpdateSessionDescriptionRequest>,
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
    put,
    path = "/sessions/{session_id}/user_recipe_values",
    request_body = UpdateSessionUserRecipeValuesRequest,
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session user recipe values updated successfully"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
// Update session user recipe parameter values
async fn update_session_user_recipe_values(
    Path(session_id): Path<String>,
    Json(request): Json<UpdateSessionUserRecipeValuesRequest>,
) -> Result<StatusCode, StatusCode> {
    SessionManager::update_session(&session_id)
        .user_recipe_values(Some(request.user_recipe_values))
        .apply()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/sessions/{session_id}",
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

#[utoipa::path(
    get,
    path = "/sessions/{session_id}/export",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session")
    ),
    responses(
        (status = 200, description = "Session exported successfully", body = String),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn export_session(Path(session_id): Path<String>) -> Result<Json<String>, StatusCode> {
    let exported = SessionManager::export_session(&session_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(exported))
}

#[utoipa::path(
    post,
    path = "/sessions/import",
    request_body = ImportSessionRequest,
    responses(
        (status = 200, description = "Session imported successfully", body = Session),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 400, description = "Bad request - Invalid JSON"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn import_session(
    Json(request): Json<ImportSessionRequest>,
) -> Result<Json<Session>, StatusCode> {
    let session = SessionManager::import_session(&request.json)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(session))
}

pub struct SessionSseResponse {
    rx: ReceiverStream<String>,
}

impl SessionSseResponse {
    fn new(rx: ReceiverStream<String>) -> Self {
        Self { rx }
    }
}

impl Stream for SessionSseResponse {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx)
            .poll_next(cx)
            .map(|opt| opt.map(|s| Ok(Bytes::from(s))))
    }
}

impl IntoResponse for SessionSseResponse {
    fn into_response(self) -> axum::response::Response {
        let stream = self;
        let body = axum::body::Body::from_stream(stream);

        http::Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("X-Accel-Buffering", "no") // Disable nginx buffering
            .body(body)
            .unwrap()
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum SessionEvent {
    Session { session: Box<Session> },
    Error { error: String },
    Ping,
}

async fn stream_session_event(event: SessionEvent, tx: &mpsc::Sender<String>) -> bool {
    let json = serde_json::to_string(&event).unwrap_or_else(|e| {
        format!(
            r#"{{"type":"Error","error":"Failed to serialize event: {}"}}"#,
            e
        )
    });
    tx.send(format!("data: {}\n\n", json)).await.is_ok()
}

#[utoipa::path(
    get,
    path = "/sessions/{session_id}/stream",
    params(
        ("session_id" = String, Path, description = "Unique identifier for the session to stream")
    ),
    responses(
        (status = 200, description = "Session stream initiated", content_type = "text/event-stream"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Session Management"
)]
async fn stream_session(
    State(_state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<SessionSseResponse, StatusCode> {
    // Verify session exists first
    let initial_session = SessionManager::get_session(&session_id, true)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);

    // Spawn background task to stream session updates
    tokio::spawn(async move {
        // Send initial session state immediately
        if !stream_session_event(
            SessionEvent::Session {
                session: Box::new(initial_session.clone()),
            },
            &tx,
        )
        .await
        {
            tracing::info!("Session stream client disconnected");
            return;
        }

        // If session is not in use, close the stream immediately
        if !initial_session.in_use {
            tracing::info!("Session {} not in use, closing stream", session_id);
            return;
        }

        // Session is in use, continue streaming until it's no longer in use
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut last_message_count: Option<usize> = Some(initial_session.message_count);
        let mut last_updated_at: Option<String> = Some(initial_session.updated_at.to_rfc3339());

        loop {
            interval.tick().await;

            // Fetch current session state
            match SessionManager::get_session(&session_id, true).await {
                Ok(session) => {
                    // Check if session has been updated since last check
                    let updated_at_str = session.updated_at.to_rfc3339();
                    let has_updates = last_message_count != Some(session.message_count)
                        || last_updated_at.as_ref() != Some(&updated_at_str);

                    if has_updates {
                        last_message_count = Some(session.message_count);
                        last_updated_at = Some(updated_at_str);

                        let session_in_use = session.in_use;

                        if !stream_session_event(
                            SessionEvent::Session {
                                session: Box::new(session),
                            },
                            &tx,
                        )
                        .await
                        {
                            tracing::info!("Session stream client disconnected");
                            break;
                        }

                        if !session_in_use {
                            tracing::info!(
                                "Session {} no longer in use, closing stream",
                                session_id
                            );
                            break;
                        }
                    } else {
                        if !session.in_use {
                            tracing::info!(
                                "Session {} no longer in use, closing stream",
                                session_id
                            );
                            break;
                        }

                        // Send heartbeat ping to keep connection alive
                        if !stream_session_event(SessionEvent::Ping, &tx).await {
                            tracing::info!("Session stream client disconnected");
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error fetching session {}: {}", session_id, e);
                    let _ = stream_session_event(
                        SessionEvent::Error {
                            error: format!("Failed to fetch session: {}", e),
                        },
                        &tx,
                    )
                    .await;
                    break;
                }
            }
        }
    });

    Ok(SessionSseResponse::new(stream))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/{session_id}", get(get_session))
        .route("/sessions/{session_id}/stream", get(stream_session))
        .route("/sessions/{session_id}", delete(delete_session))
        .route("/sessions/{session_id}/export", get(export_session))
        .route("/sessions/import", post(import_session))
        .route("/sessions/insights", get(get_session_insights))
        .route(
            "/sessions/{session_id}/description",
            put(update_session_description),
        )
        .route(
            "/sessions/{session_id}/user_recipe_values",
            put(update_session_user_recipe_values),
        )
        .with_state(state)
}
