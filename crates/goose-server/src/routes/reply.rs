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

use mcp_core::{content::Content, role::Role};
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
                        let tool_call = mcp_core::tool::ToolCall {
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

    fn format_error(error: &str) -> String {
        // Error messages start with "3:" in the new protocol.
        let encoded_error = serde_json::to_string(error).unwrap_or_else(|_| String::new());
        format!("3:{}\n", encoded_error)
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
    match message.role {
        Role::User => {
            // Handle tool responses
            for content in message.content {
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
                            // Send an error message first
                            tx.send(ProtocolFormatter::format_error(&err.to_string()))
                                .await?;
                            // Then send an empty tool response to maintain the protocol
                            let result =
                                vec![Content::text(format!("Error: {}", err)).with_priority(0.0)];
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
                        match request.tool_call {
                            Ok(tool_call) => {
                                tx.send(ProtocolFormatter::format_tool_call(
                                    &request.id,
                                    &tool_call.name,
                                    &tool_call.arguments,
                                ))
                                .await?;
                            }
                            Err(err) => {
                                // Send a placeholder tool call to maintain protocol
                                tx.send(ProtocolFormatter::format_tool_call(
                                    &request.id,
                                    "invalid_tool",
                                    &json!({"error": err.to_string()}),
                                ))
                                .await?;
                            }
                        }
                    }
                    MessageContent::Text(text) => {
                        for line in text.text.lines() {
                            let modified_line = format!("{}\n", line);
                            tx.send(ProtocolFormatter::format_text(&modified_line))
                                .await?;
                        }
                    }
                    MessageContent::ToolConfirmationRequest(_) => {
                        // skip tool confirmation requests
                    }
                    MessageContent::Image(_) => {
                        // skip images
                    }
                    MessageContent::ToolResponse(_) => {
                        // skip tool responses
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
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

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
        let agent = match agent.as_ref() {
            Some(agent) => agent,
            None => {
                let _ = tx
                    .send(ProtocolFormatter::format_error("No agent configured"))
                    .await;
                let _ = tx.send(ProtocolFormatter::format_finish("error")).await;
                return;
            }
        };

        let mut stream = match agent.reply(&messages).await {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to start reply stream: {:?}", e);
                let _ = tx
                    .send(ProtocolFormatter::format_error(&e.to_string()))
                    .await;
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
                                let _ = tx.send(ProtocolFormatter::format_error(&e.to_string())).await;
                                break;
                            }
                        }
                        Ok(Some(Err(e))) => {
                            tracing::error!("Error processing message: {}", e);
                            let _ = tx.send(ProtocolFormatter::format_error(&e.to_string())).await;
                            break;
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(_) => { // Heartbeat, used to detect disconnected clients and then end running tools.
                            if tx.is_closed() {
                                // Kill any running processes when the client disconnects
                                // TODO is this used? I suspect post MCP this is on the server instead
                                // goose::process_store::kill_processes();
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

#[derive(Debug, Deserialize, serde::Serialize)]
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

    #[test]
    fn test_convert_messages_user_only() {
        let incoming = vec![IncomingMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            tool_invocations: vec![],
        }];

        let messages = convert_messages(incoming);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, Role::User);
        assert!(
            matches!(&messages[0].content[0], MessageContent::Text(text) if text.text == "Hello")
        );
    }

    #[test]
    fn test_convert_messages_with_tool_invocation() {
        let tool_result = vec![Content::text("tool response").with_priority(0.0)];
        let incoming = vec![IncomingMessage {
            role: "assistant".to_string(),
            content: "".to_string(),
            tool_invocations: vec![ToolInvocation {
                state: "result".to_string(),
                tool_call_id: "123".to_string(),
                tool_name: "test_tool".to_string(),
                args: json!({"key": "value"}),
                result: Some(tool_result.clone()),
            }],
        }];

        let messages = convert_messages(incoming);
        assert_eq!(messages.len(), 2); // Tool request and response

        // Check tool request
        assert_eq!(messages[0].role, Role::Assistant);
        assert!(
            matches!(&messages[0].content[0], MessageContent::ToolRequest(req) if req.id == "123")
        );

        // Check tool response
        assert_eq!(messages[1].role, Role::User);
        assert!(
            matches!(&messages[1].content[0], MessageContent::ToolResponse(resp) if resp.id == "123")
        );
    }

    #[test]
    fn test_protocol_formatter() {
        // Test text formatting
        let text = "Hello world";
        let formatted = ProtocolFormatter::format_text(text);
        assert_eq!(formatted, "0:\"Hello world\"\n");

        // Test tool call formatting
        let formatted =
            ProtocolFormatter::format_tool_call("123", "test_tool", &json!({"key": "value"}));
        assert!(formatted.starts_with("9:"));
        assert!(formatted.contains("\"toolCallId\":\"123\""));
        assert!(formatted.contains("\"toolName\":\"test_tool\""));

        // Test tool response formatting
        let result = vec![Content::text("response").with_priority(0.0)];
        let formatted = ProtocolFormatter::format_tool_response("123", &result);
        assert!(formatted.starts_with("a:"));
        assert!(formatted.contains("\"toolCallId\":\"123\""));

        // Test error formatting
        let formatted = ProtocolFormatter::format_error("Test error");
        println!("Formatted error: {}", formatted);
        assert!(formatted.starts_with("3:"));
        assert!(formatted.contains("Test error"));

        // Test finish formatting
        let formatted = ProtocolFormatter::format_finish("stop");
        assert!(formatted.starts_with("d:"));
        assert!(formatted.contains("\"finishReason\":\"stop\""));
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
                config: Arc::new(Mutex::new(HashMap::new())), // Add this line
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
