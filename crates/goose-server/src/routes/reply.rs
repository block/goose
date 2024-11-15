use axum::{
    extract::State,
    http::{self, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use bytes::Bytes;
use futures::{stream::StreamExt, Stream};
use goose::{
    agent::Agent,
    developer::DeveloperSystem,
    providers::{
        configs::ProviderConfig,
        factory::{self, ProviderType},
        types::{
            content::Content,
            message::{Message, Role},
        },
    },
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::state::AppState;

// Request type matching the Python implementation
#[derive(Debug, Deserialize)]
struct ChatRequest {
    messages: Vec<Value>,
}

// Custom SSE response type that implements the Vercel AI SDK protocol
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
            .header("x-vercel-ai-data-stream", "v1")
            .body(body)
            .unwrap()
    }
}

// Convert JSON message to our Message type
fn convert_message(value: &Value) -> Option<Message> {
    let role = value.get("role")?.as_str()?;
    let role = match role {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        _ => return None,
    };

    let content = if let Some(content) = value.get("content") {
        if let Some(text) = content.as_str() {
            vec![Content::text(text)]
        } else if let Some(array) = content.as_array() {
            array
                .iter()
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()).map(Content::text))
                .collect()
        } else {
            return None;
        }
    } else {
        Vec::new()
    };

    Message::new(role, content).ok()
}

async fn chat_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<SseResponse, StatusCode> {
    // Check protocol header (optional in our case)
    if let Some(protocol) = headers.get("x-protocol") {
        if protocol.to_str().map(|p| p != "data").unwrap_or(true) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Create channel for streaming
    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);

    // Setup agent with developer system
    let system = Box::new(DeveloperSystem::new());

    // Determine provider type based on config
    let provider_type = match &state.provider_config {
        ProviderConfig::OpenAi(_) => ProviderType::OpenAi,
        ProviderConfig::Databricks(_) => ProviderType::Databricks,
    };

    let provider = factory::get_provider(provider_type, state.provider_config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut agent = Agent::new(provider);
    agent.add_system(system);

    // Convert incoming messages
    let messages: Vec<Message> = request
        .messages
        .iter()
        .filter_map(convert_message)
        .collect();

    // Spawn task to handle streaming
    tokio::spawn(async move {
        let mut stream = agent.reply(&messages);

        while let Some(response) = stream.next().await {
            match response {
                Ok(message) => {
                    match message.role {
                        Role::User => {
                            // Handle tool results if present
                            if let Some(Content::ToolResponse(tool_data)) = message.content.first() {
                                // Introduce a small delay to ensure tool calls and results are separated
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                
                                let result = json!({
                                    "toolCallId": tool_data.request_id,
                                    "result": tool_data.output.as_ref().unwrap(),
                                });
                                let _ = tx.send(format!("a:{}\n", result)).await;
                            }
                        }
                        Role::Assistant => {
                            for content in message.content {
                                match content {
                                    Content::ToolRequest(request) => {
                                        if let Ok(call) = request.call {
                                            let tool_call = json!({
                                                "toolCallId": request.id,
                                                "toolName": call.name,
                                                "args": call.parameters
                                            });
                                            let _ = tx.send(format!("9:{}\n", tool_call)).await;
                                        }
                                    }
                                    _ => {
                                        let text = content.summary();
                                        // Split text by newlines and send each line separately
                                        for line in text.lines() {
                                            let escaped_line = line.replace('\"', "\\\"");
                                            if let Err(e) = tx.send(format!("0:\"{}\"\n", escaped_line)).await {
                                                tracing::error!("Error sending line through channel: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error processing message: {}", e);
                    break;
                }
            }
        }

        // Send finish message
        let finish = json!({
            "finishReason": "stop",
            "usage": {
                "promptTokens": 0,
                "completionTokens": 0
            }
        });
        let _ = tx.send(format!("d:{}\n", finish)).await;
    });

    Ok(SseResponse::new(stream))
}

// Configure routes for this module
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/reply", post(chat_handler))
        .with_state(state)
}