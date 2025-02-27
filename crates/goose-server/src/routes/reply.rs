use crate::state::AppState;
use axum::{
    extract::State,
    http::{self, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use bytes::Bytes;
use futures::{stream::StreamExt, Stream};
use goose::message::{Message, MessageContent};

use mcp_core::role::Role;
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;

// Direct message serialization for the chat request
#[derive(Debug, Deserialize)]
struct ChatRequest {
    messages: Vec<Message>,
}

// Custom SSE response type for streaming messages
pub struct SseResponse {
    rx: ReceiverStream<String>,
}

impl SseResponse {
    fn new(rx: ReceiverStream<String>) -> Self {
        Self { rx }
    }
}

impl Stream for SseResponse {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx)
            .poll_next(cx)
            .map(|opt| opt.map(|s| Ok(Bytes::from(s))))
    }
}

impl IntoResponse for SseResponse {
    fn into_response(self) -> axum::response::Response {
        let stream = self;
        let body = axum::body::Body::from_stream(stream);

        http::Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(body)
            .unwrap()
    }
}

// Message event types for SSE streaming
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum MessageEvent {
    Message { message: Message },
    Error { error: String },
    Finish { reason: String },
}

// Stream a message as an SSE event
async fn stream_event(
    event: MessageEvent,
    tx: &mpsc::Sender<String>,
) -> Result<(), mpsc::error::SendError<String>> {
    let json = serde_json::to_string(&event).unwrap_or_else(|e| {
        format!(
            r#"{{"type":"Error","error":"Failed to serialize event: {}"}}"#,
            e
        )
    });
    tx.send(format!("data: {}\n\n", json)).await
}

async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<SseResponse, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Create channel for streaming
    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);

    // Get messages directly from the request
    let messages = request.messages;

    // Get a lock on the shared agent
    let agent = state.agent.clone();

    // Spawn task to handle streaming
    tokio::spawn(async move {
        let agent = agent.lock().await;
        let agent = match agent.as_ref() {
            Some(agent) => agent,
            None => {
                let _ = stream_event(
                    MessageEvent::Error {
                        error: "No agent configured".to_string(),
                    },
                    &tx,
                )
                .await;
                let _ = stream_event(
                    MessageEvent::Finish {
                        reason: "error".to_string(),
                    },
                    &tx,
                )
                .await;
                return;
            }
        };

        let mut stream = match agent.reply(&messages).await {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to start reply stream: {:?}", e);
                let _ = stream_event(
                    MessageEvent::Error {
                        error: e.to_string(),
                    },
                    &tx,
                )
                .await;
                let _ = stream_event(
                    MessageEvent::Finish {
                        reason: "error".to_string(),
                    },
                    &tx,
                )
                .await;
                return;
            }
        };

        loop {
            tokio::select! {
                response = timeout(Duration::from_millis(500), stream.next()) => {
                    match response {
                        Ok(Some(Ok(message))) => {
                            if let Err(e) = stream_event(MessageEvent::Message { message }, &tx).await {
                                tracing::error!("Error sending message through channel: {}", e);
                                let _ = stream_event(
                                    MessageEvent::Error {
                                        error: e.to_string(),
                                    },
                                    &tx,
                                ).await;
                                break;
                            }
                        }
                        Ok(Some(Err(e))) => {
                            tracing::error!("Error processing message: {}", e);
                            let _ = stream_event(
                                MessageEvent::Error {
                                    error: e.to_string(),
                                },
                                &tx,
                            ).await;
                            break;
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(_) => { // Heartbeat, used to detect disconnected clients
                            if tx.is_closed() {
                                break;
                            }
                            continue;
                        }
                    }
                }
            }
        }

        // Send finish event
        let _ = stream_event(
            MessageEvent::Finish {
                reason: "stop".to_string(),
            },
            &tx,
        )
        .await;
    });

    Ok(SseResponse::new(stream))
}

#[derive(Debug, Deserialize, Serialize)]
struct AskRequest {
    prompt: String,
}

#[derive(Debug, Serialize)]
struct AskResponse {
    response: String,
}

// Simple ask an AI for a response, non streaming
async fn ask_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AskRequest>,
) -> Result<Json<AskResponse>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let agent = state.agent.clone();
    let agent = agent.lock().await;
    let agent = agent.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    // Create a single message for the prompt
    let messages = vec![Message::user().with_text(request.prompt)];

    // Get response from agent
    let mut response_text = String::new();
    let mut stream = match agent.reply(&messages).await {
        Ok(stream) => stream,
        Err(e) => {
            tracing::error!("Failed to start reply stream: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    while let Some(response) = stream.next().await {
        match response {
            Ok(message) => {
                if message.role == Role::Assistant {
                    for content in message.content {
                        if let MessageContent::Text(text) = content {
                            response_text.push_str(&text.text);
                            response_text.push('\n');
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error processing as_ai message: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok(Json(AskResponse {
        response: response_text.trim().to_string(),
    }))
}

// Configure routes for this module
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/reply", post(handler))
        .route("/ask", post(ask_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::{
        agents::AgentFactory,
        model::ModelConfig,
        providers::{
            base::{Provider, ProviderUsage, Usage},
            errors::ProviderError,
        },
    };
    use mcp_core::tool::Tool;

    // Mock Provider implementation for testing
    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn metadata() -> goose::providers::base::ProviderMetadata {
            goose::providers::base::ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    mod integration_tests {
        use super::*;
        use axum::{body::Body, http::Request};
        use std::collections::HashMap;
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use tower::ServiceExt;

        // This test requires tokio runtime
        #[tokio::test]
        async fn test_ask_endpoint() {
            // Create a mock app state with mock provider
            let mock_model_config = ModelConfig::new("test-model".to_string());
            let mock_provider = Box::new(MockProvider {
                model_config: mock_model_config,
            });
            let agent = AgentFactory::create("reference", mock_provider).unwrap();
            let state = AppState {
                config: Arc::new(Mutex::new(HashMap::new())),
                agent: Arc::new(Mutex::new(Some(agent))),
                secret_key: "test-secret".to_string(),
            };

            // Build router
            let app = routes(state);

            // Create request
            let request = Request::builder()
                .uri("/ask")
                .method("POST")
                .header("content-type", "application/json")
                .header("x-secret-key", "test-secret")
                .body(Body::from(
                    serde_json::to_string(&AskRequest {
                        prompt: "test prompt".to_string(),
                    })
                    .unwrap(),
                ))
                .unwrap();

            // Send request
            let response = app.oneshot(request).await.unwrap();

            // Assert response status
            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
