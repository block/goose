use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

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

    #[error("Unexpected transport error: {0}")]
    Other(String),
}

/// A generic asynchronous transport trait.
///
/// Implementations are expected to handle:
/// - starting the underlying communication channel (e.g., launching a child process, connecting a socket)
/// - sending JSON-RPC messages as strings
/// - receiving JSON-RPC messages as strings
/// - closing the transport cleanly
#[async_trait]
pub trait Transport: Send + 'static {
    /// Start the transport and establish the underlying connection.
    async fn start(&self) -> Result<(), Error>;

    /// Send a raw JSON-encoded message through the transport.
    async fn send(&self, msg: String) -> Result<(), Error>;

    /// Receive a raw JSON-encoded message from the transport.
    ///
    /// This should return a single line representing one JSON message.
    async fn receive(&self) -> Result<String, Error>;

    /// Close the transport and free any resources.
    async fn close(&self) -> Result<(), Error>;
}

#[async_trait]
impl<T: Transport> Transport for Arc<Mutex<T>> {
    async fn start(&self) -> Result<(), Error> {
        let transport = self.lock().await;
        transport.start().await
    }

    async fn send(&self, msg: String) -> Result<(), Error> {
        let transport = self.lock().await;
        transport.send(msg).await
    }

    async fn receive(&self) -> Result<String, Error> {
        let transport = self.lock().await;
        transport.receive().await
    }

    async fn close(&self) -> Result<(), Error> {
        let transport = self.lock().await;
        transport.close().await
    }
}

pub mod stdio;
pub use stdio::StdioTransport;

pub mod sse;
pub use sse::SseTransport;
