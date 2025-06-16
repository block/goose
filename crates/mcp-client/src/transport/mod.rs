use async_trait::async_trait;
use mcp_core::protocol::JsonRpcMessage;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

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

    #[error("Streamable HTTP error: {0}")]
    StreamableHttpError(String),

    #[error("Session error: {0}")]
    SessionError(String),
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
    async fn send(&self, message: JsonRpcMessage) -> Result<(), Error>;
    async fn receive(&self) -> Result<JsonRpcMessage, Error>;
}

pub async fn serialize_and_send(
    sender: &mpsc::Sender<String>,
    message: JsonRpcMessage,
) -> Result<(), Error> {
    match serde_json::to_string(&message).map_err(Error::Serialization) {
        Ok(msg) => sender.send(msg).await.map_err(|_| Error::ChannelClosed),
        Err(e) => {
            tracing::error!(error = ?e, "Error serializing message");
            Err(e)
        }
    }
}

pub mod stdio;
pub use stdio::StdioTransport;

pub mod sse;
pub use sse::SseTransport;

pub mod streamable_http;
pub use streamable_http::StreamableHttpTransport;
