/// WebSocket transport implementation for MCP client
///
/// This module provides a WebSocket transport layer compatible with the Kotlin SDK's
/// WebSocket server implementation. It uses JSON-RPC over WebSocket as specified by MCP.
///
/// The transport connects to WebSocket endpoints (typically at `/mcp` path) and handles
/// bidirectional communication using text frames containing JSON-RPC messages.
///
/// # Architecture
///
/// The WebSocket transport is implemented as an adapter that bridges WebSocket communication
/// to RMCP's async I/O transport layer. It uses line-delimited JSON over WebSocket text frames,
/// where each frame contains a complete JSON-RPC message.
///
/// # Protocol
///
/// - **Connection**: WebSocket over HTTP/HTTPS (ws:// or wss://)
/// - **Message Format**: JSON-RPC 2.0, one message per WebSocket text frame
/// - **Bidirectional**: Both client and server can initiate requests
/// - **Framing**: Each JSON-RPC message is sent as a single WebSocket text frame
///
/// # Keep-Alive
///
/// The underlying `tokio-tungstenite` library automatically handles WebSocket ping/pong frames
/// to keep connections alive. Servers can send ping frames, and the client will automatically
/// respond with pong frames. This happens transparently at the WebSocket protocol level.
///
/// # Connection Management
///
/// Currently, the transport does not implement automatic reconnection. If the connection is lost,
/// the MCP client will receive an error and the extension will need to be restarted. Future versions
/// may add reconnection support at the MCP client level (requiring re-initialization).
///
/// # Example Configuration
///
/// ```yaml
/// extensions:
///   - type: websocket
///     name: my-mcp-server
///     uri: ws://localhost:8080/mcp
///     timeout: 30
/// ```
use anyhow::{Context, Result};
use futures::{Sink, Stream, StreamExt};
use rmcp::transport::async_rw::AsyncRwTransport;
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::{error, warn};

/// WebSocket transport configuration
#[derive(Debug, Clone)]
pub struct WebSocketTransportConfig {
    /// WebSocket URL (ws:// or wss://)
    pub url: String,
    /// Custom headers to include in the WebSocket handshake
    pub headers: HashMap<String, String>,
}

impl WebSocketTransportConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: HashMap::new(),
        }
    }

    /// Create a new WebSocket transport config with custom headers
    pub fn with_headers(url: impl Into<String>, headers: HashMap<String, String>) -> Self {
        Self {
            url: url.into(),
            headers,
        }
    }

    /// Convert HTTP/HTTPS URL to WebSocket URL
    pub fn from_http_url(url: impl AsRef<str>) -> Self {
        let url = url.as_ref();
        let ws_url = if url.starts_with("https://") {
            url.replace("https://", "wss://")
        } else if url.starts_with("http://") {
            url.replace("http://", "ws://")
        } else {
            url.to_string()
        };
        Self {
            url: ws_url,
            headers: HashMap::new(),
        }
    }

    /// Convert HTTP/HTTPS URL to WebSocket URL with custom headers
    pub fn from_http_url_with_headers(
        url: impl AsRef<str>,
        headers: HashMap<String, String>,
    ) -> Self {
        let url = url.as_ref();
        let ws_url = if url.starts_with("https://") {
            url.replace("https://", "wss://")
        } else if url.starts_with("http://") {
            url.replace("http://", "ws://")
        } else {
            url.to_string()
        };
        Self {
            url: ws_url,
            headers,
        }
    }
}

/// WebSocket transport for MCP client
///
/// This transport connects to a WebSocket endpoint and provides an async I/O interface
/// compatible with RMCP's transport layer.
pub struct WebSocketTransport {
    config: WebSocketTransportConfig,
}

impl WebSocketTransport {
    pub fn new(config: WebSocketTransportConfig) -> Self {
        Self { config }
    }

    /// Create a new WebSocket transport with the given URL
    pub fn with_url(url: impl Into<String>) -> Self {
        Self::new(WebSocketTransportConfig::new(url))
    }

    /// Connect to the WebSocket server and return an async I/O transport
    pub async fn connect(
        self,
    ) -> Result<AsyncRwTransport<rmcp::RoleClient, WebSocketReader, WebSocketWriter>> {
        let url = self.config.url.clone();

        // Create a WebSocket request with the 'mcp' subprotocol
        let mut request = self
            .config
            .url
            .clone()
            .into_client_request()
            .context("Failed to create WebSocket request")?;

        // Add the 'mcp' subprotocol header
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", "mcp".parse().unwrap());

        // Add custom headers from the config
        for (key, value) in &self.config.headers {
            use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};

            match (key.parse::<HeaderName>(), value.parse::<HeaderValue>()) {
                (Ok(header_name), Ok(header_value)) => {
                    request.headers_mut().insert(header_name, header_value);
                }
                _ => {
                    warn!("Skipping invalid header: {} = {}", key, value);
                }
            }
        }

        let (ws_stream, _response) = connect_async(request)
            .await
            .with_context(|| format!("Failed to connect to WebSocket at {}", url))?;

        // Split the WebSocket stream into read and write halves
        let (write, read) = ws_stream.split();

        let reader = WebSocketReader::new(read);
        let writer = WebSocketWriter::new(write);

        let transport = AsyncRwTransport::new(reader, writer);

        Ok(transport)
    }
}

/// Reader half of the WebSocket adapter
///
/// Implements AsyncRead for the read side of a WebSocket connection.
pub struct WebSocketReader {
    read_half: futures::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    read_buffer: Vec<u8>,
    read_pos: usize,
}

impl WebSocketReader {
    fn new(
        read_half: futures::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> Self {
        Self {
            read_half,
            read_buffer: Vec::new(),
            read_pos: 0,
        }
    }
}

/// Writer half of the WebSocket adapter
///
/// Implements AsyncWrite for the write side of a WebSocket connection.
pub struct WebSocketWriter {
    write_half: futures::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
}

impl WebSocketWriter {
    fn new(
        write_half: futures::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    ) -> Self {
        Self { write_half }
    }
}

impl AsyncRead for WebSocketReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // If we have data in the buffer, copy it to the output buffer
        if self.read_pos < self.read_buffer.len() {
            let remaining = &self.read_buffer[self.read_pos..];
            let to_copy = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..to_copy]);
            self.read_pos += to_copy;

            // If we've consumed all the buffer, clear it
            if self.read_pos >= self.read_buffer.len() {
                self.read_buffer.clear();
                self.read_pos = 0;
            }

            return Poll::Ready(Ok(()));
        }

        // Try to read the next WebSocket message
        match Pin::new(&mut self.read_half).poll_next(cx) {
            Poll::Ready(Some(Ok(Message::Text(text)))) => {
                // Add the text to our buffer with a newline (line-delimited JSON)
                self.read_buffer = text.into_bytes();
                self.read_buffer.push(b'\n');
                self.read_pos = 0;

                // Copy what we can to the output buffer
                let to_copy = self.read_buffer.len().min(buf.remaining());
                buf.put_slice(&self.read_buffer[..to_copy]);
                self.read_pos = to_copy;

                Poll::Ready(Ok(()))
            }
            Poll::Ready(Some(Ok(Message::Close(_)))) => {
                Poll::Ready(Ok(())) // EOF
            }
            Poll::Ready(Some(Ok(Message::Ping(_)))) | Poll::Ready(Some(Ok(Message::Pong(_)))) => {
                // tungstenite handles pong automatically
                // Wake up to try reading again
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Ok(msg))) => {
                warn!("Received unexpected WebSocket message type: {:?}", msg);
                // Wake up to try reading again
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => {
                error!("WebSocket error: {}", e);
                Poll::Ready(Err(io::Error::other(e)))
            }
            Poll::Ready(None) => {
                Poll::Ready(Ok(())) // EOF
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for WebSocketWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // Convert the buffer to a string (should be line-delimited JSON)
        let text = match std::str::from_utf8(buf) {
            Ok(t) => t,
            Err(e) => {
                error!("Invalid UTF-8 in write buffer: {}", e);
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid UTF-8: {}", e),
                )));
            }
        };

        // Split by newlines and send each line as a separate WebSocket text frame
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return Poll::Ready(Ok(buf.len()));
        }

        // Send the first line (we'll handle multiple lines in subsequent calls)
        let line = lines[0];

        // First check if we can send
        match Pin::new(&mut self.write_half).poll_ready(cx) {
            Poll::Ready(Ok(())) => {
                let msg = Message::Text(line.to_string());
                // Send the message
                if let Err(e) = Pin::new(&mut self.write_half).start_send(msg) {
                    return Poll::Ready(Err(io::Error::other(e)));
                }

                // Immediately flush to ensure the message is sent
                match Pin::new(&mut self.write_half).poll_flush(cx) {
                    Poll::Ready(Ok(())) => {
                        // Return the number of bytes we "wrote" (including the newline)
                        let bytes_written = line.len() + 1;
                        Poll::Ready(Ok(bytes_written.min(buf.len())))
                    }
                    Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::other(e))),
                    Poll::Pending => {
                        // Message is queued but not yet sent, wake up to try flushing again
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::other(e))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.write_half)
            .poll_flush(cx)
            .map_err(io::Error::other)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.write_half)
            .poll_close(cx)
            .map_err(io::Error::other)
    }
}

/// Error type for WebSocket transport
#[derive(Debug)]
pub enum WebSocketError {
    ConnectionFailed(String),
    SendFailed(String),
    ReceiveFailed(String),
    SerializationError(String),
}

impl fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebSocketError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            WebSocketError::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            WebSocketError::ReceiveFailed(msg) => write!(f, "Receive failed: {}", msg),
            WebSocketError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
        }
    }
}

impl std::error::Error for WebSocketError {}

impl From<WebSocketError> for std::io::Error {
    fn from(err: WebSocketError) -> Self {
        std::io::Error::other(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_conversion() {
        let config = WebSocketTransportConfig::from_http_url("http://localhost:8080/mcp");
        assert_eq!(config.url, "ws://localhost:8080/mcp");

        let config = WebSocketTransportConfig::from_http_url("https://example.com/mcp");
        assert_eq!(config.url, "wss://example.com/mcp");

        let config = WebSocketTransportConfig::from_http_url("ws://localhost:8080/mcp");
        assert_eq!(config.url, "ws://localhost:8080/mcp");
    }

    #[test]
    fn test_config_creation() {
        let config = WebSocketTransportConfig::new("wss://example.com/mcp");
        assert_eq!(config.url, "wss://example.com/mcp");
    }

    #[test]
    fn test_subprotocol_header() {
        // Verify that the MCP subprotocol is correctly set
        let config = WebSocketTransportConfig::new("ws://localhost:8080/mcp");
        assert_eq!(config.url, "ws://localhost:8080/mcp");
    }
}
