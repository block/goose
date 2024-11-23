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
    models::content::Content,
    models::message::{Message, MessageContent},
    models::role::Role,
    providers::factory,
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

// Types matching the incoming JSON structure
#[derive(Debug, Deserialize)]
struct ChatRequest {
    messages: Vec<IncomingMessage>,
}

#[derive(Debug, Deserialize)]
struct IncomingMessage {
    role: String,
    content: String,
    #[serde(default)]
    #[serde(rename = "toolInvocations")]
    tool_invocations: Vec<ToolInvocation>,
}

#[derive(Debug, Deserialize)]
struct ToolInvocation {
    state: String,
    #[serde(rename = "toolCallId")]
    tool_call_id: String,
    #[serde(rename = "toolName")]
    tool_name: String,
    args: Value,
    result: Option<Vec<Content>>,
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

// Convert incoming messages to our internal Message type
fn convert_messages(incoming: Vec<IncomingMessage>) -> Vec<Message> {
    let mut messages = Vec::new();

    for msg in incoming {
        match msg.role.as_str() {
            "user" => {
                messages.push(Message::user().with_text(msg.content));
            }
            "assistant" => {
                // First handle any tool invocations - each represents a complete request/response cycle
                for tool in msg.tool_invocations {
                    if tool.state == "result" {
                        // Add the original tool request from assistant
                        let tool_call = goose::models::tool::ToolCall {
                            name: tool.tool_name,
                            arguments: tool.args,
                        };
                        messages.push(
                            Message::assistant()
                                .with_tool_request(tool.tool_call_id.clone(), Ok(tool_call)),
                        );

                        // Add the tool response from user
                        if let Some(result) = &tool.result {
                            messages.push(
                                Message::user()
                                    .with_tool_response(tool.tool_call_id, Ok(result.clone())),
                            );
                        }
                    }
                }

                // Then add the assistant's text response after tool interactions
                if !msg.content.is_empty() {
                    messages.push(Message::assistant().with_text(msg.content));
                }
            }
            _ => {
                tracing::warn!("Unknown role: {}", msg.role);
            }
        }
    }

    messages
}

// Protocol-specific message formatting
struct ProtocolFormatter;

impl ProtocolFormatter {
    fn format_text(text: &str) -> String {
        // Text messages start with "0:"
        format!("0:\"{}\"\n", text.replace('\"', "\\\""))
    }

    fn format_tool_call(id: &str, name: &str, args: &Value) -> String {
        // Tool calls start with "9:"
        let tool_call = json!({
            "toolCallId": id,
            "toolName": name,
            "args": args
        });
        format!("9:{}\n", tool_call)
    }

    fn format_tool_response(id: &str, result: &Vec<Content>) -> String {
        // Tool responses start with "a:"
        let response = json!({
            "toolCallId": id,
            "result": result,
        });
        format!("a:{}\n", response)
    }

    fn format_finish(reason: &str) -> String {
        // Finish messages start with "d:"
        let finish = json!({
            "finishReason": reason,
            "usage": {
                "promptTokens": 0,
                "completionTokens": 0
            }
        });
        format!("d:{}\n", finish)
    }
}

async fn stream_message(
    message: Message,
    tx: &mpsc::Sender<String>,
) -> Result<(), mpsc::error::SendError<String>> {
    dbg!(&message);
    match message.role {
        Role::User => {
            // Handle tool responses
            for content in message.content {
                dbg!(&content);
                // I believe with the protocol we aren't intended to pass back user messages, so we only deal with
                // the tool responses here
                if let MessageContent::ToolResponse(response) = content {
                    // We should return a result for either an error or a success
                    match response.tool_result {
                        Ok(result) => {
                            tx.send(ProtocolFormatter::format_tool_response(
                                &response.id,
                                &result,
                            ))
                            .await?;
                        }
                        Err(err) => {
                            let result = vec![Content::text(format!("Error {}", err))];
                            tx.send(ProtocolFormatter::format_tool_response(
                                &response.id,
                                &result,
                            ))
                            .await?;
                        }
                    }
                }
            }
        }
        Role::Assistant => {
            for content in message.content {
                match content {
                    MessageContent::ToolRequest(request) => {
                        if let Ok(tool_call) = request.tool_call {
                            tx.send(ProtocolFormatter::format_tool_call(
                                &request.id,
                                &tool_call.name,
                                &tool_call.arguments,
                            ))
                            .await?;
                        } else {
                            // if the llm generates an invalid object tool call, we still have
                            // to include it in the history. It always comes with a response indicating the error
                            tx.send(ProtocolFormatter::format_tool_call(
                                &request.id,
                                "invalid name",
                                &json!({}),
                            ))
                            .await?;
                        }
                    }
                    MessageContent::Text(text) => {
                        for line in text.text.lines() {
                            tx.send(ProtocolFormatter::format_text(&format!("{}\\n", line))).await?;
                        }
                    }
                    MessageContent::Image(_) => {
                        // TODO
                        continue;
                    }
                    MessageContent::ToolResponse(_) => {
                        // Tool responses should only come from the user
                        continue;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handler(
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
    let provider = factory::get_provider(state.provider_config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut agent = Agent::new(provider);
    agent.add_system(system);

    // Convert incoming messages
    let messages = convert_messages(request.messages);

    // Spawn task to handle streaming
    tokio::spawn(async move {
        let mut stream = match agent.reply(&messages).await {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to start reply stream: {}", e);
                // Send a finish message with error as the reason
                let _ = tx.send(ProtocolFormatter::format_finish("error")).await;
                return;
            }
        };

        while let Some(response) = stream.next().await {
            match response {
                Ok(message) => {
                    if let Err(e) = stream_message(message, &tx).await {
                        tracing::error!("Error sending message through channel: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Error processing message: {}", e);
                    break;
                }
            }
        }

        // Send finish message
        let _ = tx.send(ProtocolFormatter::format_finish("stop")).await;
    });

    Ok(SseResponse::new(stream))
}

// Configure routes for this module
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/reply", post(handler))
        .with_state(state)
}
