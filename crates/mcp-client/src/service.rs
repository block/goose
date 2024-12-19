use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;
use tower::Service;

use crate::transport::{Error as TransportError, Transport};
use mcp_core::protocol::JsonRpcMessage;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Request timed out")]
    Timeout(#[from] tower::timeout::error::Elapsed),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Unexpected server response")]
    UnexpectedResponse,
}

/// A Tower `Service` implementation that uses a `Transport` to send/receive JsonRpcMessages and JsonRpcMessages.
pub struct TransportService<T: Transport> {
    transport: Arc<Mutex<T>>,
    initialized: AtomicBool,
}

impl<T: Transport> TransportService<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport: Arc::new(Mutex::new(transport)),
            initialized: AtomicBool::new(false),
        }
    }

    /// Provides a clone of the transport handle for external access (e.g., for sending notifications).
    pub fn get_transport_handle(&self) -> Arc<Mutex<T>> {
        Arc::clone(&self.transport)
    }
}

impl<T: Transport> Service<JsonRpcMessage> for TransportService<T> {
    type Response = JsonRpcMessage;
    type Error = ServiceError;
    type Future = Pin<Box<dyn Future<Output = Result<JsonRpcMessage, ServiceError>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Always ready. We do on-demand initialization in call().
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, message: JsonRpcMessage) -> Self::Future {
        let transport = Arc::clone(&self.transport);
        let started = self.initialized.load(Ordering::SeqCst);

        Box::pin(async move {
            let transport = transport.lock().await;

            // Initialize (start) transport on the first call.
            if !started {
                transport.start().await?;
            }

            match message {
                JsonRpcMessage::Notification(notification) => {
                    // Serialize notification
                    let msg = serde_json::to_string(&notification)?;
                    transport.send(msg).await?;
                    // For notifications, the protocol does not require a response
                    // So we return an empty response here and this is not checked upstream
                    Ok(JsonRpcMessage::Nil)
                }
                JsonRpcMessage::Request(request) => {
                    // Serialize request & wait for response
                    let msg = serde_json::to_string(&request)?;
                    transport.send(msg).await?;
                    let line = transport.receive().await?;
                    let response: JsonRpcMessage = serde_json::from_str(&line)?;
                    Ok(response)
                }
                _ => return Err(ServiceError::Other("Invalid message type".to_string())),
            }
        })
    }
}

impl<T: Transport> Drop for TransportService<T> {
    fn drop(&mut self) {
        if self.initialized.load(Ordering::SeqCst) {
            // Create a new runtime for cleanup if needed
            let rt = tokio::runtime::Runtime::new().unwrap();
            let transport = rt.block_on(self.transport.lock());
            let _ = rt.block_on(transport.close());
        }
    }
}
