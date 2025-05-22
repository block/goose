use futures::future::BoxFuture;
use mcp_core::protocol::{JsonRpcMessage, JsonRpcRequest};
use std::collections::HashMap;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{oneshot, RwLock};
use tower::{timeout::Timeout, Service, ServiceBuilder};

use crate::transport::{Error, TransportHandle};

/// A wrapper service that implements Tower's Service trait for MCP transport
#[derive(Clone)]
pub struct McpService<T: TransportHandle> {
    inner: Arc<T>,
    pending_requests: Arc<PendingRequests>,
}

impl<T: TransportHandle> McpService<T> {
    pub fn new(transport: T) -> Self {
        Self {
            inner: Arc::new(transport),
            pending_requests: Arc::new(PendingRequests::default()),
        }
    }

    pub async fn respond(&self, id: &str, response: Result<JsonRpcMessage, Error>) {
        self.pending_requests.respond(id, response).await
    }
}

impl<T> Service<JsonRpcMessage> for McpService<T>
where
    T: TransportHandle + Send + Sync + 'static,
{
    type Response = JsonRpcMessage;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Most transports are always ready, but this could be customized if needed
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: JsonRpcMessage) -> Self::Future {
        let transport = self.inner.clone();
        let pending_requests = self.pending_requests.clone();

        Box::pin(async move {
            match request {
                JsonRpcMessage::Request(JsonRpcRequest { id: Some(id), .. }) => {
                    // Create a channel to receive the response
                    let (sender, receiver) = oneshot::channel();
                    pending_requests.insert(id.to_string(), sender).await;

                    transport.send(request).await?;
                    receiver.await.map_err(|_| Error::ChannelClosed)?
                }
                JsonRpcMessage::Request(_) => {
                    // Handle notifications without waiting for a response
                    transport.send(request).await?;
                    Ok(JsonRpcMessage::Nil)
                }
                JsonRpcMessage::Notification(_) => {
                    // Handle notifications without waiting for a response
                    transport.send(request).await?;
                    Ok(JsonRpcMessage::Nil)
                }
                _ => Err(Error::UnsupportedMessage),
            }
        })
    }
}

// Add a convenience constructor for creating a service with timeout
impl<T> McpService<T>
where
    T: TransportHandle,
{
    pub fn with_timeout(transport: T, timeout: std::time::Duration) -> Timeout<McpService<T>> {
        ServiceBuilder::new()
            .timeout(timeout)
            .service(McpService::new(transport))
    }
}

// A data structure to store pending requests and their response channels
pub struct PendingRequests {
    requests: RwLock<HashMap<String, oneshot::Sender<Result<JsonRpcMessage, Error>>>>,
}

impl Default for PendingRequests {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingRequests {
    pub fn new() -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
        }
    }

    pub async fn insert(&self, id: String, sender: oneshot::Sender<Result<JsonRpcMessage, Error>>) {
        self.requests.write().await.insert(id, sender);
    }

    pub async fn respond(&self, id: &str, response: Result<JsonRpcMessage, Error>) {
        if let Some(tx) = self.requests.write().await.remove(id) {
            let _ = tx.send(response);
        }
    }

    pub async fn clear(&self) {
        self.requests.write().await.clear();
    }

    pub async fn len(&self) -> usize {
        self.requests.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}
