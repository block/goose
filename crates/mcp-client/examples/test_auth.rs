use anyhow::Result;
use mcp_client::client::{ClientCapabilities, ClientInfo, McpClient, McpClientTrait};
use mcp_client::transport::{StreamableHttpTransport, Transport};
use std::collections::HashMap;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("mcp_client=debug".parse().unwrap())
                .add_directive("eventsource_client=info".parse().unwrap()),
        )
        .init();

    println!("Testing Streamable HTTP transport with auto-authentication...");

    // Create the Streamable HTTP transport for any MCP service that supports OAuth
    // This example uses a hypothetical MCP endpoint - replace with actual service
    let mcp_endpoint =
        std::env::var("MCP_ENDPOINT").unwrap_or_else(|_| "https://example.com/mcp".to_string());

    println!("Using MCP endpoint: {}", mcp_endpoint);

    let transport = StreamableHttpTransport::new(&mcp_endpoint, HashMap::new());

    // Start transport
    let handle = transport.start().await?;

    // Create client
    let mut client = McpClient::connect(handle, Duration::from_secs(30)).await?;
    println!("Client created with Streamable HTTP transport\n");

    // Initialize - this should trigger the OAuth flow if authentication is needed
    let server_info = client
        .initialize(
            ClientInfo {
                name: "streamable-http-auth-test".into(),
                version: "1.0.0".into(),
            },
            ClientCapabilities::default(),
        )
        .await?;

    println!("Connected to server: {server_info:?}\n");
    println!("Authentication test completed successfully!");

    Ok(())
}
