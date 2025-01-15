use futures::future::BoxFuture;
use mcp_core::protocol::JsonRpcMessage;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{timeout::Timeout, Service, ServiceBuilder};

use crate::transport::{Error, TransportHandle};

/// A wrapper service that implements Tower's Service trait for MCP transport
#[derive(Clone)]
pub struct McpService<T: TransportHandle> {
    inner: Arc<T>,
}

impl<T: TransportHandle> McpService<T> {
    pub fn new(transport: T) -> Self {
        Self {
            inner: Arc::new(transport),
        }
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
        Box::pin(async move { transport.send(request).await })
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
