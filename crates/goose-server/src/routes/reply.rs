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
use goose::models::{content::Content, message::{Message, MessageContent}, role::Role};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;

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
        let encoded_text = serde_json::to_string(text).unwrap_or_else(|_| String::new());
        format!("0:{}\n", encoded_text)
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
                            let result =
                                vec![Content::text(format!("Error {}", err)).with_priority(0.0)];
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
                            let modified_line = format!("{}\n", line);
                            tx.send(ProtocolFormatter::format_text(&modified_line))
                                .await?;
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

    // Convert incoming messages
    let messages = convert_messages(request.messages);

    // Get a lock on the shared agent
    let agent = state.agent.clone();
        

    // Spawn task to handle streaming
    tokio::spawn(async move {
        let agent = agent.lock().await;
        let mut stream = match agent.reply(&messages).await {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to start reply stream: {}", e);
                // Send a finish message with error as the reason
                let _ = tx.send(ProtocolFormatter::format_finish("error")).await;
                return;
            }
        };

        loop {
            tokio::select! {
                response = timeout(Duration::from_millis(500), stream.next()) => {
                    match response {
                        Ok(Some(Ok(message))) => {
                            if let Err(e) = stream_message(message, &tx).await {
                                tracing::error!("Error sending message through channel: {}", e);
                                break;
                            }
                        }
                        Ok(Some(Err(e))) => {
                            tracing::error!("Error processing message: {}", e);
                            break;
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(_) => { // Heartbeat, used to detect disconnected clients and then end running tools.
                            if tx.is_closed() {
                                break;
                            }
                            continue;
                        }
                    }
                }
            }
        }

        // Send finish message
        let _ = tx.send(ProtocolFormatter::format_finish("stop")).await;
    });

    Ok(SseResponse::new(stream))
}

#[derive(Debug, Deserialize)]
struct AskRequest {
    prompt: String,
}

#[derive(Debug, serde::Serialize)]
struct AskResponse {
    response: String,
}

// simple ask an AI for a response, non streaming
async fn ask_handler(
    State(state): State<AppState>,
    Json(request): Json<AskRequest>,
) -> Result<Json<AskResponse>, StatusCode> {
    let agent = state.agent.clone();
    let agent = agent.lock().await;

    // Create a single message for the prompt
    let messages = vec![Message::user().with_text(request.prompt)];

    // Get response from agent
    let mut response_text = String::new();
    let mut stream = match agent.reply(&messages).await {
        Ok(stream) => stream,
        Err(e) => {
            tracing::error!("Failed to start reply stream: {}", e);
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
