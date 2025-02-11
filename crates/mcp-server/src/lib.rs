use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, Stream};
use mcp_core::protocol::{JsonRpcError, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse};
use pin_project::pin_project;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tower_service::Service;

mod errors;
pub use errors::{BoxError, RouterError, ServerError, TransportError};

pub mod router;
pub use router::Router;

/// A transport layer that handles JSON-RPC messages over byte
#[pin_project]
pub struct ByteTransport<R, W> {
    // Reader is a BufReader on the underlying stream (stdin or similar) buffering
    // the underlying data across poll calls, we clear one line (\n) during each
    // iteration of poll_next from this buffer
    #[pin]
    reader: BufReader<R>,
    #[pin]
    writer: W,
}

impl<R, W> ByteTransport<R, W>
where
    R: AsyncRead,
    W: AsyncWrite,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            // Default BufReader capacity is 8 * 1024, increase this to 2MB to the file size limit
            // allows the buffer to have the capacity to read very large calls
            reader: BufReader::with_capacity(2 * 1024 * 1024, reader),
            writer,
        }
    }
}

impl<R, W> Stream for ByteTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    type Item = Result<JsonRpcMessage, TransportError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let mut buf = Vec::new();

        let mut reader = this.reader.as_mut();
        let mut read_future = Box::pin(reader.read_until(b'\n', &mut buf));
        match read_future.as_mut().poll(cx) {
            Poll::Ready(Ok(0)) => Poll::Ready(None), // EOF
            Poll::Ready(Ok(_)) => {
                // Convert to UTF-8 string
                let line = match String::from_utf8(buf) {
                    Ok(s) => s,
                    Err(e) => return Poll::Ready(Some(Err(TransportError::Utf8(e)))),
                };
                // Log incoming message here before serde conversion to
                // track incomplete chunks which are not valid JSON
                tracing::info!(json = %line, "incoming message");

                // Parse JSON and validate message format
                match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(value) => {
                        // Validate basic JSON-RPC structure
                        if !value.is_object() {
                            return Poll::Ready(Some(Err(TransportError::InvalidMessage(
                                "Message must be a JSON object".into(),
                            ))));
                        }
                        let obj = value.as_object().unwrap(); // Safe due to check above

                        // Check jsonrpc version field
                        if !obj.contains_key("jsonrpc") || obj["jsonrpc"] != "2.0" {
                            return Poll::Ready(Some(Err(TransportError::InvalidMessage(
                                "Missing or invalid jsonrpc version".into(),
                            ))));
                        }

                        // Now try to parse as proper message
                        match serde_json::from_value::<JsonRpcMessage>(value) {
                            Ok(msg) => Poll::Ready(Some(Ok(msg))),
                            Err(e) => Poll::Ready(Some(Err(TransportError::Json(e)))),
                        }
                    }
                    Err(e) => Poll::Ready(Some(Err(TransportError::Json(e)))),
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(TransportError::Io(e)))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<R, W> ByteTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub async fn write_message(&mut self, msg: JsonRpcMessage) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(&msg)?;
        Pin::new(&mut self.writer)
            .write_all(json.as_bytes())
            .await?;
        Pin::new(&mut self.writer).write_all(b"\n").await?;
        Pin::new(&mut self.writer).flush().await?;
        Ok(())
    }
}

/// The main server type that processes incoming requests
pub struct Server<S> {
    service: S,
}

impl<S> Server<S>
where
    S: Service<JsonRpcRequest, Response = JsonRpcResponse> + Send,
    S::Error: Into<BoxError>,
    S::Future: Send,
{
    pub fn new(service: S) -> Self {
        Self { service }
    }

    // TODO transport trait instead of byte transport if we implement others
    pub async fn run<R, W>(self, mut transport: ByteTransport<R, W>) -> Result<(), ServerError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        use futures::StreamExt;
        let mut service = self.service;

        tracing::info!("Server started");
        while let Some(msg_result) = transport.next().await {
            let _span = tracing::span!(tracing::Level::INFO, "message_processing").entered();
            match msg_result {
                Ok(msg) => {
                    match msg {
                        JsonRpcMessage::Request(request) => {
                            // Serialize request for logging
                            let id = request.id;
                            let request_json = serde_json::to_string(&request)
                                .unwrap_or_else(|_| "Failed to serialize request".to_string());

                            tracing::info!(
                                request_id = ?id,
                                method = ?request.method,
                                json = %request_json,
                                "Received request"
                            );

                            // Process the request using our service
                            let response = match service.call(request).await {
                                Ok(resp) => resp,
                                Err(e) => {
                                    let error_msg = e.into().to_string();
                                    tracing::error!(error = %error_msg, "Request processing failed");
                                    JsonRpcResponse {
                                        jsonrpc: "2.0".to_string(),
                                        id,
                                        result: None,
                                        error: Some(mcp_core::protocol::ErrorData {
                                            code: mcp_core::protocol::INTERNAL_ERROR,
                                            message: error_msg,
                                            data: None,
                                        }),
                                    }
                                }
                            };

                            // Serialize response for logging
                            let response_json = serde_json::to_string(&response)
                                .unwrap_or_else(|_| "Failed to serialize response".to_string());

                            tracing::info!(
                                response_id = ?response.id,
                                json = %response_json,
                                "Sending response"
                            );
                            // Send the response back
                            if let Err(e) = transport
                                .write_message(JsonRpcMessage::Response(response))
                                .await
                            {
                                return Err(ServerError::Transport(TransportError::Io(e)));
                            }
                        }
                        JsonRpcMessage::Response(_)
                        | JsonRpcMessage::Notification(_)
                        | JsonRpcMessage::Nil
                        | JsonRpcMessage::Error(_) => {
                            // Ignore responses, notifications and nil messages for now
                            continue;
                        }
                    }
                }
                Err(e) => {
                    // Convert transport error to JSON-RPC error response
                    let error = match e {
                        TransportError::Json(_) | TransportError::InvalidMessage(_) => {
                            mcp_core::protocol::ErrorData {
                                code: mcp_core::protocol::PARSE_ERROR,
                                message: e.to_string(),
                                data: None,
                            }
                        }
                        TransportError::Protocol(_) => mcp_core::protocol::ErrorData {
                            code: mcp_core::protocol::INVALID_REQUEST,
                            message: e.to_string(),
                            data: None,
                        },
                        _ => mcp_core::protocol::ErrorData {
                            code: mcp_core::protocol::INTERNAL_ERROR,
                            message: e.to_string(),
                            data: None,
                        },
                    };

                    let error_response = JsonRpcMessage::Error(JsonRpcError {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        error,
                    });

                    if let Err(e) = transport.write_message(error_response).await {
                        return Err(ServerError::Transport(TransportError::Io(e)));
                    }
                }
            }
        }

        Ok(())
    }
}

// Define a specific service implementation that we need for any
// Any router implements this
pub trait BoundedService:
    Service<
        JsonRpcRequest,
        Response = JsonRpcResponse,
        Error = BoxError,
        Future = Pin<Box<dyn Future<Output = Result<JsonRpcResponse, BoxError>> + Send>>,
    > + Send
    + 'static
{
}

// Implement it for any type that meets the bounds
impl<T> BoundedService for T where
    T: Service<
            JsonRpcRequest,
            Response = JsonRpcResponse,
            Error = BoxError,
            Future = Pin<Box<dyn Future<Output = Result<JsonRpcResponse, BoxError>> + Send>>,
        > + Send
        + 'static
{
}
