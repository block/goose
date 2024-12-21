use async_trait::async_trait;
use mcp_core::protocol::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

/// A generic error type for transport operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Transport was not connected or is already closed")]
    NotConnected,

    #[error("Invalid URL provided")]
    InvalidUrl,

    #[error("Connection timeout")]
    Timeout,

    #[error("Failed to send message")]
    SendFailed,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {status} - {message}")]
    HttpError { status: u16, message: String },

    #[error("SSE connection error: {0}")]
    SseConnection(String),

    #[error("Connection closed by server")]
    ConnectionClosed,

    #[error("Unexpected transport error: {0}")]
    Other(String),
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
pub trait Transport: Send + Sync + 'static {
    /// Start the transport and establish the underlying connection.
    /// Returns channels for sending messages and receiving errors.
    async fn start(&self) -> Result<mpsc::Sender<TransportMessage>, Error>;

    /// Close the transport and free any resources.
    async fn close(&self) -> Result<(), Error>;
}

pub mod stdio;
pub use stdio::StdioTransport;

pub mod sse;
pub use sse::SseTransport;

/// A router that handles message distribution for a transport
#[derive(Clone)]
pub struct MessageRouter {
    transport_tx: mpsc::Sender<TransportMessage>,
    // shutdown_tx is unused, but we'll probably need it for shutdown
    #[allow(dead_code)]
    shutdown_tx: mpsc::Sender<()>,
}

impl MessageRouter {
    pub fn new(
        transport_tx: mpsc::Sender<TransportMessage>,
        shutdown_tx: mpsc::Sender<()>,
    ) -> Self {
        Self {
            transport_tx,
            shutdown_tx,
        }
    }

    /// Send a message and wait for a response
    pub async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcMessage, Error> {
        let (response_tx, response_rx) = oneshot::channel();

        self.transport_tx
            .send(TransportMessage {
                message: JsonRpcMessage::Request(request),
                response_tx: Some(response_tx),
            })
            .await
            .map_err(|_| Error::ChannelClosed)?;

        response_rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Send a notification (no response expected)
    pub async fn send_notification(&self, notification: JsonRpcNotification) -> Result<(), Error> {
        self.transport_tx
            .send(TransportMessage {
                message: JsonRpcMessage::Notification(notification),
                response_tx: None,
            })
            .await
            .map_err(|_| Error::ChannelClosed)
    }
}
