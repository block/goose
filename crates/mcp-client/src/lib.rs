pub mod client;
pub mod oauth;
pub mod service;
pub mod transport;

#[cfg(test)]
mod oauth_tests;

pub use client::{ClientCapabilities, ClientInfo, Error, McpClient, McpClientTrait};
pub use oauth::{authenticate_service, ServiceConfig};
pub use service::McpService;
pub use transport::{
    SseTransport, StdioTransport, StreamableHttpTransport, Transport, TransportHandle,
};
