//! Axum router for A2A protocol endpoints.
//!
//! Provides HTTP routes for JSON-RPC and agent card discovery.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use futures::stream::StreamExt;
use serde_json::Value;

use crate::server::executor::AgentExecutor;
use crate::server::request_handler::DefaultRequestHandler;
use crate::server::store::TaskStore;
use crate::server::transport::JsonRpcHandler;

type SharedHandler<S, E> = Arc<JsonRpcHandler<S, E>>;
type SharedRequestHandler<S, E> = Arc<DefaultRequestHandler<S, E>>;

/// Application state for Axum routes.
pub struct A2AAppState<S: TaskStore, E: AgentExecutor> {
    rpc_handler: SharedHandler<S, E>,
    request_handler: SharedRequestHandler<S, E>,
}

// Manual Clone impl — Arc<T> is Clone regardless of T: Clone
impl<S: TaskStore, E: AgentExecutor> Clone for A2AAppState<S, E> {
    fn clone(&self) -> Self {
        Self {
            rpc_handler: Arc::clone(&self.rpc_handler),
            request_handler: Arc::clone(&self.request_handler),
        }
    }
}

/// Create an Axum router with A2A protocol endpoints.
///
/// Routes:
/// - `GET /.well-known/agent-card.json` — Agent card discovery
/// - `POST /` — JSON-RPC endpoint (message/send, tasks/get, etc.)
/// - `POST /stream` — Streaming JSON-RPC endpoint (message/sendStream via SSE)
pub fn create_a2a_router<S, E>(handler: DefaultRequestHandler<S, E>) -> Router
where
    S: TaskStore + Clone + Send + Sync + 'static,
    E: AgentExecutor + Send + Sync + 'static,
{
    let handler = Arc::new(handler);
    let rpc_handler = Arc::new(JsonRpcHandler::new(Arc::clone(&handler)));

    let state = A2AAppState {
        rpc_handler,
        request_handler: handler,
    };

    Router::new()
        .route(
            "/.well-known/agent-card.json",
            get(agent_card_handler::<S, E>),
        )
        .route("/", post(jsonrpc_handler::<S, E>))
        .route("/stream", post(stream_handler::<S, E>))
        .with_state(state)
}

async fn agent_card_handler<S, E>(State(state): State<A2AAppState<S, E>>) -> impl IntoResponse
where
    S: TaskStore + Clone + Send + Sync + 'static,
    E: AgentExecutor + Send + Sync + 'static,
{
    let card = state.request_handler.get_agent_card();
    Json(card)
}

async fn jsonrpc_handler<S, E>(
    State(state): State<A2AAppState<S, E>>,
    Json(body): Json<Value>,
) -> impl IntoResponse
where
    S: TaskStore + Clone + Send + Sync + 'static,
    E: AgentExecutor + Send + Sync + 'static,
{
    let response = state.rpc_handler.handle_request(&body).await;
    Json(response)
}

async fn stream_handler<S, E>(
    State(state): State<A2AAppState<S, E>>,
    Json(body): Json<Value>,
) -> impl IntoResponse
where
    S: TaskStore + Clone + Send + Sync + 'static,
    E: AgentExecutor + Send + Sync + 'static,
{
    match state.rpc_handler.handle_stream_request(&body).await {
        Ok(stream) => {
            let sse_stream = stream.map(|result| match result {
                Ok(data) => Ok::<_, std::convert::Infallible>(Event::default().data(data)),
                Err(e) => {
                    let error_json = serde_json::json!({
                        "error": e.to_string()
                    });
                    Ok(Event::default().event("error").data(error_json.to_string()))
                }
            });

            Sse::new(sse_stream)
                .keep_alive(KeepAlive::default())
                .into_response()
        }
        Err(error_response) => (StatusCode::BAD_REQUEST, Json(error_response)).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::context::RequestContext;
    use crate::server::store::InMemoryTaskStore;
    use crate::types::agent_card::AgentCard;
    use crate::types::core::{Artifact, TaskState, TaskStatus};
    use crate::types::events::{
        AgentExecutionEvent, TaskArtifactUpdateEvent, TaskStatusUpdateEvent,
    };
    use axum::body::Body;
    use axum::http::Request;
    use tokio::sync::mpsc;
    use tower::ServiceExt;

    struct EchoExecutor;

    #[async_trait::async_trait]
    impl AgentExecutor for EchoExecutor {
        async fn execute(
            &self,
            context: RequestContext,
            tx: mpsc::Sender<AgentExecutionEvent>,
        ) -> Result<(), crate::error::A2AError> {
            let _ = tx
                .send(AgentExecutionEvent::ArtifactUpdate(
                    TaskArtifactUpdateEvent {
                        task_id: context.task_id.clone(),
                        context_id: context.context_id.clone(),
                        artifact: Artifact {
                            artifact_id: "art-1".to_string(),
                            name: Some("echo".to_string()),
                            description: None,
                            parts: context.user_message.parts.clone(),
                            metadata: None,
                            extensions: vec![],
                        },
                        append: false,
                        last_chunk: true,
                        metadata: None,
                    },
                ))
                .await;
            let _ = tx
                .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                    task_id: context.task_id,
                    context_id: context.context_id,
                    status: TaskStatus {
                        state: TaskState::Completed,
                        message: None,
                        timestamp: Some(chrono::Utc::now().to_rfc3339()),
                    },
                    metadata: None,
                }))
                .await;
            Ok(())
        }

        async fn cancel(
            &self,
            _task_id: &str,
            _tx: mpsc::Sender<AgentExecutionEvent>,
        ) -> Result<(), crate::error::A2AError> {
            Ok(())
        }
    }

    fn test_app() -> Router {
        let card = AgentCard {
            name: "Test Agent".to_string(),
            description: "Test".to_string(),
            ..Default::default()
        };
        let handler = DefaultRequestHandler::new(card, InMemoryTaskStore::new(), EchoExecutor);
        create_a2a_router(handler)
    }

    #[tokio::test]
    async fn test_agent_card_endpoint() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/agent-card.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let card: AgentCard = serde_json::from_slice(&body).unwrap();
        assert_eq!(card.name, "Test Agent");
    }

    #[tokio::test]
    async fn test_jsonrpc_endpoint() {
        let app = test_app();

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "message/send",
            "params": {
                "message": {
                    "messageId": "msg-1",
                    "role": "user",
                    "parts": [{"type": "text", "text": "Hello!"}]
                }
            }
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let rpc_response: Value = serde_json::from_slice(&body).unwrap();
        assert!(rpc_response.get("result").is_some());
    }

    #[tokio::test]
    async fn test_sse_streaming() {
        let app = test_app();

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "message/stream",
            "params": {
                "message": {
                    "messageId": "msg-stream-1",
                    "role": "user",
                    "parts": [{"type": "text", "text": "Stream test"}]
                }
            }
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/stream")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/event-stream"
        );

        // Read the SSE body and parse events
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // SSE events are delimited by double newlines, each starting with "data: "
        let events: Vec<Value> = body_str
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .map(|data| serde_json::from_str(data).unwrap())
            .collect();

        // EchoExecutor sends: ArtifactUpdate + StatusUpdate(Completed)
        // DefaultRequestHandler adds initial StatusUpdate(Working)
        // So we expect at least 2 events
        assert!(
            events.len() >= 2,
            "Expected at least 2 SSE events, got {}: {:?}",
            events.len(),
            events
        );

        // Check that we got artifact and status events
        // Events are wrapped in JSON-RPC response: {id, jsonrpc, result: {kind: ..., ...}}
        let has_artifact = events.iter().any(|e| {
            e.get("result")
                .and_then(|r| r.get("kind"))
                .map(|k| k == "artifact-update")
                .unwrap_or(false)
        });
        let has_status_completed = events.iter().any(|e| {
            let result = e.get("result");
            result
                .and_then(|r| r.get("kind"))
                .map(|k| k == "status-update")
                .unwrap_or(false)
                && result
                    .and_then(|r| r.get("status"))
                    .and_then(|s| s.get("state"))
                    .map(|s| s == "completed")
                    .unwrap_or(false)
        });

        assert!(
            has_artifact,
            "Expected artifact-update event in: {:?}",
            events
        );
        assert!(
            has_status_completed,
            "Expected status-update with completed state in: {:?}",
            events
        );
    }

    #[tokio::test]
    async fn test_jsonrpc_method_not_found() {
        let app = test_app();

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "nonexistent/method",
            "params": {}
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let rpc_response: Value = serde_json::from_slice(&body).unwrap();
        assert!(rpc_response.get("error").is_some());
    }
}
