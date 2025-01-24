use async_trait::async_trait;
use mcp_core::protocol::JsonRpcMessage;
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, RwLock};

pub type BoxError = Box<dyn std::error::Error + Sync + Send>;
/// A generic error type for transport operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Transport was not connected or is already closed")]
    NotConnected,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unsupported message type. JsonRpcMessage can only be Request or Notification.")]
    UnsupportedMessage,

    #[error("Stdio process error: {0}")]
    StdioProcessError(String),

    #[error("SSE connection error: {0}")]
    SseConnection(String),

    #[error("HTTP error: {status} - {message}")]
    HttpError { status: u16, message: String },
}

/// A message that can be sent through the transport
#[derive(Debug)]
pub struct TransportMessage {
    /// The JSON-RPC message to send
    pub message: JsonRpcMessage,
    /// Channel to receive the response on (None for notifications)
    pub response_tx: Option<oneshot::Sender<Result<JsonRpcMessage, Error>>>,
}

/// A generic asynchronous transport trait with channel-based communication
#[async_trait]
pub trait Transport {
    type Handle: TransportHandle;

    /// Start the transport and establish the underlying connection.
    /// Returns the transport handle for sending messages.
    async fn start(&self) -> Result<Self::Handle, Error>;

    /// Close the transport and free any resources.
    async fn close(&self) -> Result<(), Error>;
}

#[async_trait]
pub trait TransportHandle: Send + Sync + Clone + 'static {
    async fn send(&self, message: JsonRpcMessage) -> Result<JsonRpcMessage, Error>;
}

// Helper function that contains the common send implementation
pub async fn send_message(
    sender: &mpsc::Sender<TransportMessage>,
    message: JsonRpcMessage,
) -> Result<JsonRpcMessage, Error> {
    match message {
        JsonRpcMessage::Request(request) => {
            let (respond_to, response) = oneshot::channel();
            let msg = TransportMessage {
                message: JsonRpcMessage::Request(request),
                response_tx: Some(respond_to),
            };
            sender.send(msg).await.map_err(|_| Error::ChannelClosed)?;
            Ok(response.await.map_err(|_| Error::ChannelClosed)??)
        }
        JsonRpcMessage::Notification(notification) => {
            let msg = TransportMessage {
                message: JsonRpcMessage::Notification(notification),
                response_tx: None,
            };
            sender.send(msg).await.map_err(|_| Error::ChannelClosed)?;
            Ok(JsonRpcMessage::Nil)
        }
        _ => Err(Error::UnsupportedMessage),
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
}

pub mod stdio;
pub use stdio::StdioTransport;

pub mod sse;
pub use sse::SseTransport;
