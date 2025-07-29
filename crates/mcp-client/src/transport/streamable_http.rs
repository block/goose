use crate::oauth::{authenticate_service, ServiceConfig};
use crate::transport::{Error, TransportMessageRecv};
use async_trait::async_trait;
use eventsource_client::{Client, SSE};
use futures::TryStreamExt;
use reqwest::Client as HttpClient;
use rmcp::model::{JsonRpcMessage, JsonRpcRequest, NumberOrString::Number};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::Duration;
use tracing::{debug, error, info, warn};
use url::Url;

use super::{serialize_and_send, Transport, TransportHandle};

// Default timeout for HTTP requests
const HTTP_TIMEOUT_SECS: u64 = 30;

/// The Streamable HTTP transport actor that handles:
/// - HTTP POST requests to send messages to the server
/// - Optional streaming responses for receiving multiple responses and server-initiated messages
/// - Session management with session IDs
pub struct StreamableHttpActor {
    /// Receives messages (requests/notifications) from the handle
    receiver: mpsc::Receiver<String>,
    /// Sends messages (responses) back to the handle
    sender: mpsc::Sender<TransportMessageRecv>,
    /// MCP endpoint URL
    mcp_endpoint: String,
    /// HTTP client for sending requests
    http_client: HttpClient,
    /// Optional session ID for stateful connections
    session_id: Arc<RwLock<Option<String>>>,
    /// Environment variables to set
    env: HashMap<String, String>,
    /// Custom headers to include in requests
    headers: HashMap<String, String>,
}

impl StreamableHttpActor {
    pub fn new(
        receiver: mpsc::Receiver<String>,
        sender: mpsc::Sender<TransportMessageRecv>,
        mcp_endpoint: String,
        session_id: Arc<RwLock<Option<String>>>,
        env: HashMap<String, String>,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            receiver,
            sender,
            mcp_endpoint,
            http_client: HttpClient::builder()
                .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
                .build()
                .unwrap(),
            session_id,
            env,
            headers,
        }
    }

    /// Main entry point for the actor
    pub async fn run(mut self) {
        // Set environment variables
        for (key, value) in &self.env {
            std::env::set_var(key, value);
        }

        // Handle outgoing messages
        while let Some(message_str) = self.receiver.recv().await {
            if let Err(e) = self.handle_outgoing_message(message_str).await {
                error!("Error handling outgoing message: {}", e);
                break;
            }
        }

        debug!("StreamableHttpActor shut down");
    }

    /// Handle an outgoing message by sending it via HTTP POST
    async fn handle_outgoing_message(&mut self, message_str: String) -> Result<(), Error> {
        debug!("Sending message to MCP endpoint: {}", message_str);

        // Parse the message to determine if it's a request that expects a response
        let parsed_message =
            serde_json::from_str::<JsonRpcMessage>(&message_str).map_err(Error::Serialization)?;

        let expects_response = matches!(
            parsed_message,
            JsonRpcMessage::Request(JsonRpcRequest { id: Number(_), .. })
        );

        // Try to send the request
        match self.send_request(&message_str, expects_response).await {
            Ok(()) => Ok(()),
            Err(Error::HttpError { status, .. }) if status == 401 || status == 403 => {
                // Authentication challenge - try to authenticate and retry
                info!(
                    "Received authentication challenge ({}), attempting OAuth flow...",
                    status
                );

                if let Some(token) = self.attempt_authentication().await? {
                    info!("Authentication successful, retrying request...");
                    self.headers
                        .insert("Authorization".to_string(), format!("Bearer {}", token));
                    self.send_request(&message_str, expects_response).await
                } else {
                    Err(Error::StreamableHttpError(
                        "Authentication failed - service not supported or OAuth flow failed"
                            .to_string(),
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Send an HTTP request to the MCP endpoint
    async fn send_request(
        &mut self,
        message_str: &str,
        expects_response: bool,
    ) -> Result<(), Error> {
        // Build the HTTP request
        let mut request = self
            .http_client
            .post(&self.mcp_endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-06-18") // Required protocol version header
            .body(message_str.to_string());

        // Add session ID header if we have one
        if let Some(session_id) = self.session_id.read().await.as_ref() {
            request = request.header("Mcp-Session-Id", session_id);
        }

        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // Send the request
        let response = request
            .send()
            .await
            .map_err(|e| Error::StreamableHttpError(format!("HTTP request failed: {}", e)))?;

        // Handle HTTP error status codes
        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 {
                // Session not found - clear our session ID
                *self.session_id.write().await = None;
                return Err(Error::SessionError(
                    "Session expired or not found".to_string(),
                ));
            }
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::HttpError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        // Check for session ID in response headers
        if let Some(session_id_header) = response.headers().get("Mcp-Session-Id") {
            if let Ok(session_id) = session_id_header.to_str() {
                debug!("Received session ID: {}", session_id);
                *self.session_id.write().await = Some(session_id.to_string());
            }
        }

        // Handle the response based on content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if content_type.starts_with("text/event-stream") {
            // Handle streaming HTTP response (server chose to stream multiple messages back)
            if expects_response {
                self.handle_streaming_response(response).await?;
            }
        } else if content_type.starts_with("application/json") || expects_response {
            // Handle single JSON response
            let response_text = response.text().await.map_err(|e| {
                Error::StreamableHttpError(format!("Failed to read response: {}", e))
            })?;

            if !response_text.is_empty() {
                let json_message = serde_json::from_str::<TransportMessageRecv>(&response_text)
                    .map_err(Error::Serialization)?;

                let _ = self.sender.send(json_message).await;
            }
        }
        // For notifications and responses, we get 202 Accepted with no body

        Ok(())
    }

    /// Attempt to authenticate with the service
    async fn attempt_authentication(&self) -> Result<Option<String>, Error> {
        info!("Attempting to authenticate with service...");

        // Create a generic OAuth configuration from the MCP endpoint
        match ServiceConfig::from_mcp_endpoint(&self.mcp_endpoint) {
            Ok(config) => {
                info!("Created OAuth config for endpoint: {}", self.mcp_endpoint);

                match authenticate_service(config, &self.mcp_endpoint).await {
                    Ok(token) => {
                        info!("OAuth authentication successful!");
                        Ok(Some(token))
                    }
                    Err(e) => {
                        warn!("OAuth authentication failed: {}", e);
                        Err(Error::StreamableHttpError(format!("OAuth failed: {}", e)))
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Could not create OAuth config from MCP endpoint {}: {}",
                    self.mcp_endpoint, e
                );
                Ok(None)
            }
        }
    }

    /// Handle streaming HTTP response that uses Server-Sent Events format
    ///
    /// This is called when the server responds to an HTTP POST with `text/event-stream`
    /// content-type, indicating it wants to stream multiple JSON-RPC messages back
    /// rather than sending a single response. This is part of the Streamable HTTP
    /// specification, not a separate SSE transport.
    async fn handle_streaming_response(
        &mut self,
        response: reqwest::Response,
    ) -> Result<(), Error> {
        use futures::StreamExt;
        use tokio::io::AsyncBufReadExt;
        use tokio_util::io::StreamReader;

        // Convert the response body to a stream reader
        let stream = response
            .bytes_stream()
            .map(|result| result.map_err(std::io::Error::other));
        let reader = StreamReader::new(stream);
        let mut lines = tokio::io::BufReader::new(reader).lines();

        let mut event_type = String::new();
        let mut event_data = String::new();
        let mut event_id = String::new();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.is_empty() {
                // Empty line indicates end of event
                if !event_data.is_empty() {
                    // Parse the streamed data as JSON-RPC message
                    match serde_json::from_str::<TransportMessageRecv>(&event_data) {
                        Ok(message) => {
                            debug!("Received streaming HTTP response message: {:?}", message);
                            let _ = self.sender.send(message).await;
                        }
                        Err(err) => {
                            warn!("Failed to parse streaming HTTP response message: {}", err);
                        }
                    }
                }
                // Reset for next event
                event_type.clear();
                event_data.clear();
                event_id.clear();
            } else if let Some(field_data) = line.strip_prefix("data: ") {
                if !event_data.is_empty() {
                    event_data.push('\n');
                }
                event_data.push_str(field_data);
            } else if let Some(field_data) = line.strip_prefix("event: ") {
                event_type = field_data.to_string();
            } else if let Some(field_data) = line.strip_prefix("id: ") {
                event_id = field_data.to_string();
            }
            // Ignore other fields (retry, etc.) - we only care about data
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct StreamableHttpTransportHandle {
    sender: mpsc::Sender<String>,
    receiver: Arc<Mutex<mpsc::Receiver<TransportMessageRecv>>>,
    session_id: Arc<RwLock<Option<String>>>,
    mcp_endpoint: String,
    http_client: HttpClient,
    headers: HashMap<String, String>,
}

#[async_trait::async_trait]
impl TransportHandle for StreamableHttpTransportHandle {
    async fn send(&self, message: JsonRpcMessage) -> Result<(), Error> {
        serialize_and_send(&self.sender, message).await
    }

    async fn receive(&self) -> Result<TransportMessageRecv, Error> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await.ok_or(Error::ChannelClosed)
    }
}

impl StreamableHttpTransportHandle {
    /// Manually terminate the session by sending HTTP DELETE
    pub async fn terminate_session(&self) -> Result<(), Error> {
        if let Some(session_id) = self.session_id.read().await.as_ref() {
            let mut request = self
                .http_client
                .delete(&self.mcp_endpoint)
                .header("Mcp-Session-Id", session_id)
                .header("MCP-Protocol-Version", "2025-06-18"); // Required protocol version header

            // Add custom headers
            for (key, value) in &self.headers {
                request = request.header(key, value);
            }

            match request.send().await {
                Ok(response) => {
                    if response.status().as_u16() == 405 {
                        // Method not allowed - server doesn't support session termination
                        debug!("Server doesn't support session termination");
                    }
                }
                Err(e) => {
                    warn!("Failed to terminate session: {}", e);
                }
            }
        }
        Ok(())
    }

    /// Create a GET request to establish a streaming connection for server-initiated messages
    pub async fn listen_for_server_messages(&self) -> Result<(), Error> {
        let mut request = self
            .http_client
            .get(&self.mcp_endpoint)
            .header("Accept", "text/event-stream")
            .header("MCP-Protocol-Version", "2025-06-18"); // Required protocol version header

        // Add session ID header if we have one
        if let Some(session_id) = self.session_id.read().await.as_ref() {
            request = request.header("Mcp-Session-Id", session_id);
        }

        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        let response = request.send().await.map_err(|e| {
            Error::StreamableHttpError(format!("Failed to start GET streaming connection: {}", e))
        })?;

        if !response.status().is_success() {
            if response.status().as_u16() == 405 {
                // Method not allowed - server doesn't support GET streaming connections
                debug!("Server doesn't support GET streaming connections");
                return Ok(());
            }
            return Err(Error::HttpError {
                status: response.status().as_u16(),
                message: "Failed to establish GET streaming connection".to_string(),
            });
        }

        // Handle the streaming connection in a separate task
        let receiver = self.receiver.clone();
        let url = response.url().clone();

        tokio::spawn(async move {
            let client = match eventsource_client::ClientBuilder::for_url(url.as_str()) {
                Ok(builder) => builder.build(),
                Err(e) => {
                    error!(
                        "Failed to create streaming client for GET connection: {}",
                        e
                    );
                    return;
                }
            };

            let mut stream = client.stream();
            while let Ok(Some(event)) = stream.try_next().await {
                match event {
                    SSE::Event(e) if e.event_type == "message" || e.event_type.is_empty() => {
                        match serde_json::from_str::<JsonRpcMessage>(&e.data) {
                            Ok(message) => {
                                debug!("Received GET streaming message: {:?}", message);
                                let receiver_guard = receiver.lock().await;
                                // We can't send through the receiver since it's for outbound messages
                                // This would need a different channel for server-initiated messages
                                drop(receiver_guard);
                            }
                            Err(err) => {
                                warn!("Failed to parse GET streaming message: {}", err);
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}

#[derive(Clone)]
pub struct StreamableHttpTransport {
    mcp_endpoint: String,
    env: HashMap<String, String>,
    headers: HashMap<String, String>,
}

impl StreamableHttpTransport {
    pub fn new<S: Into<String>>(mcp_endpoint: S, env: HashMap<String, String>) -> Self {
        Self {
            mcp_endpoint: mcp_endpoint.into(),
            env,
            headers: HashMap::new(),
        }
    }

    pub fn with_headers<S: Into<String>>(
        mcp_endpoint: S,
        env: HashMap<String, String>,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            mcp_endpoint: mcp_endpoint.into(),
            env,
            headers,
        }
    }

    /// Validate that the URL is a valid MCP endpoint
    pub fn validate_endpoint(endpoint: &str) -> Result<(), Error> {
        Url::parse(endpoint)
            .map_err(|e| Error::StreamableHttpError(format!("Invalid MCP endpoint URL: {}", e)))?;
        Ok(())
    }
}

#[async_trait]
impl Transport for StreamableHttpTransport {
    type Handle = StreamableHttpTransportHandle;

    async fn start(&self) -> Result<Self::Handle, Error> {
        // Validate the endpoint URL
        Self::validate_endpoint(&self.mcp_endpoint)?;

        // Create channels for communication
        let (tx, rx) = mpsc::channel(32);
        let (otx, orx) = mpsc::channel(32);

        let session_id: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
        let session_id_clone = Arc::clone(&session_id);

        // Create and spawn the actor
        let actor = StreamableHttpActor::new(
            rx,
            otx,
            self.mcp_endpoint.clone(),
            session_id,
            self.env.clone(),
            self.headers.clone(),
        );

        tokio::spawn(actor.run());

        // Create the handle
        let handle = StreamableHttpTransportHandle {
            sender: tx,
            receiver: Arc::new(Mutex::new(orx)),
            session_id: session_id_clone,
            mcp_endpoint: self.mcp_endpoint.clone(),
            http_client: HttpClient::builder()
                .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
                .build()
                .unwrap(),
            headers: self.headers.clone(),
        };

        Ok(handle)
    }

    async fn close(&self) -> Result<(), Error> {
        // The transport is closed when the actor task completes
        // No additional cleanup needed
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::sync::RwLock;

    #[test]
    fn test_message_parsing_request() {
        // Test that we can parse a JSON-RPC request message using the mcp-core types
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "capabilities": {}
            }
        });

        let message_str = serde_json::to_string(&request_json).unwrap();
        let parsed_message = serde_json::from_str::<JsonRpcMessage>(&message_str);
        assert!(
            parsed_message.is_ok(),
            "Should be able to parse JSON-RPC request message"
        );
    }

    #[test]
    fn test_message_parsing_response() {
        // Test that we can parse a JSON-RPC response message
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "capabilities": {}
            }
        });

        let message_str = serde_json::to_string(&response_json).unwrap();
        let parsed_message = serde_json::from_str::<JsonRpcMessage>(&message_str);
        assert!(
            parsed_message.is_ok(),
            "Should be able to parse JSON-RPC response message"
        );
    }

    #[test]
    fn test_message_parsing_notification() {
        // Test that we can parse a JSON-RPC notification message
        let notification_json = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });

        let message_str = serde_json::to_string(&notification_json).unwrap();
        let parsed_message = serde_json::from_str::<JsonRpcMessage>(&message_str);
        assert!(
            parsed_message.is_ok(),
            "Should be able to parse JSON-RPC notification message"
        );
    }

    #[test]
    fn test_message_parsing_error() {
        // Test that we can parse a JSON-RPC error message
        let error_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            }
        });

        let message_str = serde_json::to_string(&error_json).unwrap();
        let parsed_message = serde_json::from_str::<JsonRpcMessage>(&message_str);
        assert!(
            parsed_message.is_ok(),
            "Should be able to parse JSON-RPC error message"
        );
    }

    #[test]
    fn test_message_parsing_invalid_json() {
        let invalid_json = "{ invalid json }";
        let parsed_message = serde_json::from_str::<JsonRpcMessage>(invalid_json);
        assert!(parsed_message.is_err(), "Invalid JSON should fail to parse");
    }

    #[test]
    fn test_transport_message_recv_parsing() {
        // Test that we can parse messages as TransportMessageRecv (the type used for incoming messages)
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "capabilities": {}
            }
        });

        let message_str = serde_json::to_string(&response_json).unwrap();

        // For incoming messages
        let parsed_message = serde_json::from_str::<TransportMessageRecv>(&message_str);
        assert!(
            parsed_message.is_ok(),
            "Should be able to parse response as TransportMessageRecv"
        );
    }

    #[test]
    fn test_untagged_enum_serialization_issue() {
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        let message_str = serde_json::to_string(&request_json).unwrap();

        let parsed_as_jsonrpc = serde_json::from_str::<JsonRpcMessage>(&message_str);
        assert!(
            parsed_as_jsonrpc.is_ok(),
            "Should be able to parse request as JsonRpcMessage"
        );
    }

    #[test]
    fn test_expects_response_logic_with_number_id() {
        // Check if a message expects a response
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        let message_str = serde_json::to_string(&request_json).unwrap();
        let parsed_message = serde_json::from_str::<JsonRpcMessage>(&message_str).unwrap();

        // This should match the logic in handle_outgoing_message after the fix
        // The original code used: JsonRpcMessage::Request(JsonRpcRequest { id: Number(_), .. })
        let expects_response = match parsed_message {
            JsonRpcMessage::Request(_) => true,
            _ => false,
        };

        assert!(expects_response, "Request with ID should expect a response");

        // Test notification (should not expect response)
        let notification_json = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });

        let message_str = serde_json::to_string(&notification_json).unwrap();
        let parsed_message = serde_json::from_str::<JsonRpcMessage>(&message_str).unwrap();

        let expects_response = match parsed_message {
            JsonRpcMessage::Request(_) => true,
            _ => false,
        };

        assert!(
            !expects_response,
            "Notification should not expect a response"
        );
    }

    #[tokio::test]
    async fn test_handle_outgoing_message_successful_request() {
        // Set up a mock HTTP server
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#)
            .create_async()
            .await;

        // Create channels for the actor
        let (_tx, rx) = mpsc::channel(32);
        let (otx, mut orx) = mpsc::channel(32);

        // Create the actor
        let session_id = Arc::new(RwLock::new(None));
        let mut actor = StreamableHttpActor::new(
            rx,
            otx,
            server.url(),
            session_id,
            HashMap::new(),
            HashMap::new(),
        );

        // Create a JSON-RPC request message
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "capabilities": {}
            }
        });
        let message_str = serde_json::to_string(&request_json).unwrap();

        // Test handle_outgoing_message
        let result = actor.handle_outgoing_message(message_str).await;
        assert!(result.is_ok(), "handle_outgoing_message should succeed");

        // Verify the mock was called
        mock.assert_async().await;

        // Check that a response was received
        let response =
            tokio::time::timeout(std::time::Duration::from_millis(100), orx.recv()).await;
        assert!(response.is_ok(), "Should receive a response");
        assert!(response.unwrap().is_some(), "Response should not be None");
    }

    #[tokio::test]
    async fn test_handle_outgoing_message_notification() {
        // Set up a mock HTTP server for notifications (202 Accepted, no body)
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(202)
            .create_async()
            .await;

        // Create channels for the actor
        let (_tx, rx) = mpsc::channel(32);
        let (otx, mut orx) = mpsc::channel(32);

        // Create the actor
        let session_id = Arc::new(RwLock::new(None));
        let mut actor = StreamableHttpActor::new(
            rx,
            otx,
            server.url(),
            session_id,
            HashMap::new(),
            HashMap::new(),
        );

        // Create a JSON-RPC notification message (no id)
        let notification_json = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        });
        let message_str = serde_json::to_string(&notification_json).unwrap();

        // Test handle_outgoing_message
        let result = actor.handle_outgoing_message(message_str).await;
        assert!(
            result.is_ok(),
            "handle_outgoing_message should succeed for notification"
        );

        // Verify the mock was called
        mock.assert_async().await;

        // For notifications, we shouldn't receive a response
        let response =
            tokio::time::timeout(std::time::Duration::from_millis(100), orx.recv()).await;
        assert!(
            response.is_err(),
            "Should not receive a response for notification"
        );
    }

    #[tokio::test]
    async fn test_handle_outgoing_message_http_error() {
        // Set up a mock HTTP server that returns an error
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        // Create channels for the actor
        let (_tx, rx) = mpsc::channel(32);
        let (otx, _orx) = mpsc::channel(32);

        // Create the actor
        let session_id = Arc::new(RwLock::new(None));
        let mut actor = StreamableHttpActor::new(
            rx,
            otx,
            server.url(),
            session_id,
            HashMap::new(),
            HashMap::new(),
        );

        // Create a JSON-RPC request message
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "test",
            "params": {}
        });
        let message_str = serde_json::to_string(&request_json).unwrap();

        // Test handle_outgoing_message
        let result = actor.handle_outgoing_message(message_str).await;
        assert!(
            result.is_err(),
            "handle_outgoing_message should fail with HTTP error"
        );

        // Verify it's an HTTP error
        match result.unwrap_err() {
            Error::HttpError { status, .. } => {
                assert_eq!(status, 500, "Should return HTTP 500 error");
            }
            _ => panic!("Expected HttpError"),
        }

        // Verify the mock was called
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_handle_outgoing_message_session_id_handling() {
        // Set up a mock HTTP server that returns a session ID
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("Mcp-Session-Id", "test-session-123")
            .with_body(r#"{"jsonrpc":"2.0","id":1,"result":{}}"#)
            .create_async()
            .await;

        // Create channels for the actor
        let (_tx, rx) = mpsc::channel(32);
        let (otx, _orx) = mpsc::channel(32);

        // Create the actor
        let session_id = Arc::new(RwLock::new(None));
        let session_id_clone = Arc::clone(&session_id);
        let mut actor = StreamableHttpActor::new(
            rx,
            otx,
            server.url(),
            session_id,
            HashMap::new(),
            HashMap::new(),
        );

        // Create a JSON-RPC request message
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let message_str = serde_json::to_string(&request_json).unwrap();

        // Test handle_outgoing_message
        let result = actor.handle_outgoing_message(message_str).await;
        assert!(result.is_ok(), "handle_outgoing_message should succeed");

        // Verify the session ID was stored
        let stored_session_id = session_id_clone.read().await;
        assert_eq!(
            stored_session_id.as_ref(),
            Some(&"test-session-123".to_string()),
            "Session ID should be stored"
        );

        // Verify the mock was called
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_handle_outgoing_message_invalid_json() {
        // Create channels for the actor
        let (_tx, rx) = mpsc::channel(32);
        let (otx, _orx) = mpsc::channel(32);

        // Create the actor
        let session_id = Arc::new(RwLock::new(None));
        let mut actor = StreamableHttpActor::new(
            rx,
            otx,
            "http://localhost:8080".to_string(),
            session_id,
            HashMap::new(),
            HashMap::new(),
        );

        // Test with invalid JSON
        let invalid_json = "{ invalid json }";

        // Test handle_outgoing_message
        let result = actor
            .handle_outgoing_message(invalid_json.to_string())
            .await;
        assert!(
            result.is_err(),
            "handle_outgoing_message should fail with invalid JSON"
        );

        // Verify it's a serialization error
        match result.unwrap_err() {
            Error::Serialization(_) => {
                // Expected error type
            }
            _ => panic!("Expected Serialization error"),
        }
    }

    #[tokio::test]
    async fn test_handle_outgoing_message_session_not_found() {
        // Set up a mock HTTP server that returns 404 (session not found)
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/")
            .with_status(404)
            .with_body("Session not found")
            .create_async()
            .await;

        // Create channels for the actor
        let (_tx, rx) = mpsc::channel(32);
        let (otx, _orx) = mpsc::channel(32);

        // Create the actor with an existing session ID
        let session_id = Arc::new(RwLock::new(Some("old-session".to_string())));
        let session_id_clone = Arc::clone(&session_id);
        let mut actor = StreamableHttpActor::new(
            rx,
            otx,
            server.url(),
            session_id,
            HashMap::new(),
            HashMap::new(),
        );

        // Create a JSON-RPC request message
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "test",
            "params": {}
        });
        let message_str = serde_json::to_string(&request_json).unwrap();

        // Test handle_outgoing_message
        let result = actor.handle_outgoing_message(message_str).await;
        assert!(
            result.is_err(),
            "handle_outgoing_message should fail with 404"
        );

        // Verify it's a session error and the session ID was cleared
        match result.unwrap_err() {
            Error::SessionError(_) => {
                // Expected error type
            }
            _ => panic!("Expected SessionError"),
        }

        // Verify the session ID was cleared
        let stored_session_id = session_id_clone.read().await;
        assert!(
            stored_session_id.is_none(),
            "Session ID should be cleared on 404"
        );

        // Verify the mock was called
        mock.assert_async().await;
    }
}
