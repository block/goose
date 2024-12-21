use async_trait::async_trait;
use eventsource_client::{Client, SSE};
use futures::TryStreamExt;
use reqwest::Client as HttpClient;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tracing::warn;

use super::{Error, Transport, TransportMessage};
use mcp_core::protocol::JsonRpcMessage;

/// A transport implementation that uses Server-Sent Events (SSE) for receiving messages
/// and HTTP POST for sending messages.
pub struct SseTransport {
    sse_url: String,
    http_client: HttpClient,
    post_endpoint: Arc<Mutex<Option<String>>>,
    sse_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    pending_requests: Arc<
        Mutex<std::collections::HashMap<String, oneshot::Sender<Result<JsonRpcMessage, Error>>>>,
    >,
}

impl SseTransport {
    /// Create a new SSE transport with the given SSE endpoint URL
    pub fn new<S: Into<String>>(sse_url: S) -> Self {
        Self {
            sse_url: sse_url.into(),
            http_client: HttpClient::new(),
            post_endpoint: Arc::new(Mutex::new(None)),
            sse_handle: Arc::new(Mutex::new(None)),
            pending_requests: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    async fn handle_message(
        message: JsonRpcMessage,
        pending_requests: Arc<
            Mutex<
                std::collections::HashMap<String, oneshot::Sender<Result<JsonRpcMessage, Error>>>,
            >,
        >,
    ) {
        if let JsonRpcMessage::Response(response) = &message {
            if let Some(id) = &response.id {
                if let Some(tx) = pending_requests.lock().await.remove(&id.to_string()) {
                    let _ = tx.send(Ok(message));
                }
            }
        }
    }

    async fn process_messages(
        mut message_rx: mpsc::Receiver<TransportMessage>,
        http_client: HttpClient,
        post_endpoint: Arc<Mutex<Option<String>>>,
        sse_url: String,
        pending_requests: Arc<
            Mutex<
                std::collections::HashMap<String, oneshot::Sender<Result<JsonRpcMessage, Error>>>,
            >,
        >,
    ) {
        // Set up SSE client
        let client = match eventsource_client::ClientBuilder::for_url(&sse_url) {
            Ok(builder) => builder.build(),
            Err(e) => {
                // Properly handle initial connection error
                let mut pending = pending_requests.lock().await;
                for (_, tx) in pending.drain() {
                    let _ = tx.send(Err(Error::SseConnection(e.to_string())));
                }
                return;
            }
        };

        let mut stream = client.stream();

        // First, wait for the endpoint event
        while let Ok(Some(event)) = stream.try_next().await {
            match event {
                SSE::Event(event) if event.event_type == "endpoint" => {
                    let base_url = sse_url.trim_end_matches('/').trim_end_matches("sse");
                    let endpoint_path = event.data.trim_start_matches('/');
                    let post_url = format!("{}{}", base_url, endpoint_path);
                    println!("Endpoint for POST requests: {}", post_url);
                    *post_endpoint.lock().await = Some(post_url);
                    break;
                }
                _ => continue,
            }
        }

        // Now handle all subsequent messages
        let message_handler = tokio::spawn({
            let pending_requests = pending_requests.clone();
            async move {
                while let Ok(Some(event)) = stream.try_next().await {
                    match event {
                        SSE::Event(event) if event.event_type == "message" => {
                            if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&event.data)
                            {
                                Self::handle_message(message, pending_requests.clone()).await;
                            }
                        }
                        _ => continue,
                    }
                }
            }
        });

        // Process outgoing messages
        while let Some(transport_msg) = message_rx.recv().await {
            let post_url = match post_endpoint.lock().await.as_ref() {
                Some(url) => url.clone(),
                None => {
                    if let Some(response_tx) = transport_msg.response_tx {
                        let _ = response_tx.send(Err(Error::NotConnected));
                    }
                    continue;
                }
            };

            // Serialize message first
            let message_str = match serde_json::to_string(&transport_msg.message) {
                Ok(s) => s,
                Err(e) => {
                    if let Some(response_tx) = transport_msg.response_tx {
                        let _ = response_tx.send(Err(Error::Serialization(e)));
                    }
                    continue;
                }
            };

            // Store response channel if this is a request
            if let Some(response_tx) = transport_msg.response_tx {
                if let JsonRpcMessage::Request(request) = &transport_msg.message {
                    if let Some(id) = &request.id {
                        pending_requests
                            .lock()
                            .await
                            .insert(id.to_string(), response_tx);
                    }
                }
            }

            // Send message via HTTP POST
            match http_client
                .post(&post_url)
                .header("Content-Type", "application/json")
                .body(message_str)
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        let error = Error::HttpError {
                            status: response.status().as_u16(),
                            message: response.status().to_string(),
                        };
                        // We don't handle the error directly as it will come through SSE,
                        // but we log it for debugging purposes
                        warn!("HTTP request failed with error: {}", error);
                    }
                }
                Err(e) => {
                    let error = Error::Other(format!("HTTP request failed: {}", e));
                    // Transport errors will also be communicated through the SSE channel
                    warn!("HTTP request failed with error: {}", error);
                }
            }
        }

        // Clean up
        message_handler.abort();
    }
}

#[async_trait]
impl Transport for SseTransport {
    async fn start(&self) -> Result<mpsc::Sender<TransportMessage>, Error> {
        let (message_tx, message_rx) = mpsc::channel(32);

        let http_client = self.http_client.clone();
        let post_endpoint = self.post_endpoint.clone();
        let sse_url = self.sse_url.clone();
        let pending_requests = self.pending_requests.clone();

        let handle = tokio::spawn(Self::process_messages(
            message_rx,
            http_client,
            post_endpoint,
            sse_url,
            pending_requests,
        ));

        *self.sse_handle.lock().await = Some(handle);

        Ok(message_tx)
    }

    async fn close(&self) -> Result<(), Error> {
        // Abort the SSE handler task
        if let Some(handle) = self.sse_handle.lock().await.take() {
            handle.abort();
        }

        // Clear any pending requests
        self.pending_requests.lock().await.clear();

        Ok(())
    }
}
