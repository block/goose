use crate::transport::{Error, PendingRequests, TransportMessage};
use async_trait::async_trait;
use eventsource_client::{Client, SSE};
use futures::TryStreamExt;
use mcp_core::protocol::{JsonRpcMessage, JsonRpcRequest};
use reqwest::Client as HttpClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{timeout, Duration};
use tracing::warn;
use url::Url;

use super::{send_message, Transport, TransportHandle};

// Timeout for the endpoint discovery
const ENDPOINT_TIMEOUT_SECS: u64 = 5;

/// The SSE-based actor that continuously:
/// - Reads incoming events from the SSE stream.
/// - Sends outgoing messages via HTTP POST (once the post endpoint is known).
pub struct SseActor {
    /// Receives messages (requests/notifications) from the handle
    receiver: mpsc::Receiver<TransportMessage>,
    /// Map of request-id -> oneshot sender
    pending_requests: Arc<PendingRequests>,
    /// Base SSE URL
    sse_url: String,
    /// For sending HTTP POST requests
    http_client: HttpClient,
    /// The discovered endpoint for POST requests (once "endpoint" SSE event arrives)
    post_endpoint: Arc<RwLock<Option<String>>>,
}

impl SseActor {
    pub fn new(
        receiver: mpsc::Receiver<TransportMessage>,
        pending_requests: Arc<PendingRequests>,
        sse_url: String,
        post_endpoint: Arc<RwLock<Option<String>>>,
    ) -> Self {
        Self {
            receiver,
            pending_requests,
            sse_url,
            post_endpoint,
            http_client: HttpClient::new(),
        }
    }

    /// The main entry point for the actor. Spawns two concurrent loops:
    /// 1) handle_incoming_messages (SSE events)
    /// 2) handle_outgoing_messages (sending messages via POST)
    pub async fn run(self) {
        tokio::join!(
            Self::handle_incoming_messages(
                self.sse_url.clone(),
                Arc::clone(&self.pending_requests),
                Arc::clone(&self.post_endpoint)
            ),
            Self::handle_outgoing_messages(
                self.receiver,
                self.http_client.clone(),
                Arc::clone(&self.post_endpoint),
                Arc::clone(&self.pending_requests),
            )
        );
    }

    /// Continuously reads SSE events from `sse_url`.
    /// - If an `endpoint` event is received, store it in `post_endpoint`.
    /// - If a `message` event is received, parse it as `JsonRpcMessage`
    ///   and respond to pending requests if it's a `Response`.
    async fn handle_incoming_messages(
        sse_url: String,
        pending_requests: Arc<PendingRequests>,
        post_endpoint: Arc<RwLock<Option<String>>>,
    ) {
        let client = match eventsource_client::ClientBuilder::for_url(&sse_url) {
            Ok(builder) => builder.build(),
            Err(e) => {
                pending_requests.clear().await;
                warn!("Failed to connect SSE client: {}", e);
                return;
            }
        };
        let mut stream = client.stream();

        // First, wait for the "endpoint" event
        while let Ok(Some(event)) = stream.try_next().await {
            match event {
                SSE::Event(e) if e.event_type == "endpoint" => {
                    // SSE server uses the "endpoint" event to tell us the POST URL
                    let base_url = Url::parse(&sse_url).expect("Invalid base URL");
                    let post_url = base_url
                        .join(&e.data)
                        .expect("Failed to resolve endpoint URL");

                    tracing::debug!("Discovered SSE POST endpoint: {}", post_url);
                    *post_endpoint.write().await = Some(post_url.to_string());
                    break;
                }
                _ => continue,
            }
        }

        // Now handle subsequent events
        while let Ok(Some(event)) = stream.try_next().await {
            match event {
                SSE::Event(e) if e.event_type == "message" => {
                    // Attempt to parse the SSE data as a JsonRpcMessage
                    match serde_json::from_str::<JsonRpcMessage>(&e.data) {
                        Ok(message) => {
                            // If it's a response, complete the pending request
                            if let JsonRpcMessage::Response(resp) = &message {
                                if let Some(id) = &resp.id {
                                    pending_requests.respond(&id.to_string(), Ok(message)).await;
                                }
                            }
                            // If it's something else (notification, etc.), handle as needed
                        }
                        Err(err) => {
                            warn!("Failed to parse SSE message: {err}");
                        }
                    }
                }
                _ => { /* ignore other events */ }
            }
        }

        // SSE stream ended or errored; signal any pending requests
        tracing::error!("SSE stream ended or encountered an error; clearing pending requests.");
        pending_requests.clear().await;
    }

    /// Continuously receives messages from the `mpsc::Receiver`.
    /// - If it's a request, store the oneshot in `pending_requests`.
    /// - POST the message to the discovered endpoint (once known).
    async fn handle_outgoing_messages(
        mut receiver: mpsc::Receiver<TransportMessage>,
        http_client: HttpClient,
        post_endpoint: Arc<RwLock<Option<String>>>,
        pending_requests: Arc<PendingRequests>,
    ) {
        while let Some(transport_msg) = receiver.recv().await {
            let post_url = match post_endpoint.read().await.as_ref() {
                Some(url) => url.clone(),
                None => {
                    if let Some(response_tx) = transport_msg.response_tx {
                        let _ = response_tx.send(Err(Error::NotConnected));
                    }
                    continue;
                }
            };

            // Serialize the JSON-RPC message
            let message_str = match serde_json::to_string(&transport_msg.message) {
                Ok(s) => s,
                Err(e) => {
                    if let Some(tx) = transport_msg.response_tx {
                        let _ = tx.send(Err(Error::Serialization(e)));
                    }
                    continue;
                }
            };

            // If it's a request, store the channel so we can respond later
            if let Some(response_tx) = transport_msg.response_tx {
                if let JsonRpcMessage::Request(JsonRpcRequest { id: Some(id), .. }) =
                    &transport_msg.message
                {
                    pending_requests.insert(id.to_string(), response_tx).await;
                }
            }

            // Perform the HTTP POST
            match http_client
                .post(&post_url)
                .header("Content-Type", "application/json")
                .body(message_str)
                .send()
                .await
            {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let err = Error::HttpError {
                            status: resp.status().as_u16(),
                            message: resp.status().to_string(),
                        };
                        warn!("HTTP request returned error: {err}");
                        // This doesn't directly fail the request,
                        // because we rely on SSE to deliver the error response
                    }
                }
                Err(e) => {
                    warn!("HTTP POST failed: {e}");
                    // Similarly, SSE might eventually reveal the error
                }
            }
        }

        // mpsc channel closed => no more outgoing messages
        tracing::error!("SseActor: outgoing message loop ended. Clearing pending requests.");
        pending_requests.clear().await;
    }
}

#[derive(Clone)]
pub struct SseTransportHandle {
    sender: mpsc::Sender<TransportMessage>,
}

#[async_trait::async_trait]
impl TransportHandle for SseTransportHandle {
    async fn send(&self, message: JsonRpcMessage) -> Result<JsonRpcMessage, Error> {
        send_message(&self.sender, message).await
    }
}

#[derive(Clone)]
pub struct SseTransport {
    sse_url: String,
    env: HashMap<String, String>,
}

/// The SSE transport spawns an `SseActor` on `start()`.
impl SseTransport {
    pub fn new<S: Into<String>>(sse_url: S, env: HashMap<String, String>) -> Self {
        Self {
            sse_url: sse_url.into(),
            env,
        }
    }

    /// Waits for the endpoint to be set, up to 10 attempts.
    async fn wait_for_endpoint(
        post_endpoint: Arc<RwLock<Option<String>>>,
    ) -> Result<String, Error> {
        // Check every 100ms for the endpoint, for up to 10 attempts
        let check_interval = Duration::from_millis(100);
        let mut attempts = 0;
        let max_attempts = 10;

        while attempts < max_attempts {
            if let Some(url) = post_endpoint.read().await.clone() {
                return Ok(url);
            }
            tokio::time::sleep(check_interval).await;
            attempts += 1;
        }
        Err(Error::SseConnection("No endpoint discovered".to_string()))
    }
}

#[async_trait]
impl Transport for SseTransport {
    type Handle = SseTransportHandle;

    async fn start(&self) -> Result<Self::Handle, Error> {
        // Set environment variables
        for (key, value) in &self.env {
            std::env::set_var(key, value);
        }

        // Create a channel for outgoing TransportMessages
        let (tx, rx) = mpsc::channel(32);

        let post_endpoint: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
        let post_endpoint_clone = Arc::clone(&post_endpoint);

        // Build the actor
        let actor = SseActor::new(
            rx,
            Arc::new(PendingRequests::new()),
            self.sse_url.clone(),
            post_endpoint,
        );

        // Spawn the actor task
        tokio::spawn(actor.run());

        // Wait for the endpoint to be discovered before returning the handle
        match timeout(
            Duration::from_secs(ENDPOINT_TIMEOUT_SECS),
            Self::wait_for_endpoint(post_endpoint_clone),
        )
        .await
        {
            Ok(_) => Ok(SseTransportHandle { sender: tx }),
            Err(e) => Err(Error::SseConnection(e.to_string())),
        }
    }

    async fn close(&self) -> Result<(), Error> {
        // For SSE, you might close the stream or send a shutdown signal to the actor.
        // Here, we do nothing special.
        Ok(())
    }
}
