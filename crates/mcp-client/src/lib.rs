pub mod client;
pub mod oauth;

#[cfg(test)]
mod oauth_tests;

pub use client::{Error, McpClient, McpClientTrait};
pub use oauth::{authenticate_service, ServiceConfig};
