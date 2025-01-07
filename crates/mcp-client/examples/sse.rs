use anyhow::Result;
use mcp_client::client::{ClientCapabilities, ClientInfo, McpClient, McpClientImpl};
use mcp_client::{service::TransportService, transport::SseTransport};
use std::time::Duration;
use tower::ServiceBuilder;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("mcp_client=debug".parse().unwrap())
                .add_directive("eventsource_client=debug".parse().unwrap()),
        )
        .init();

    // Create the base transport
    let transport = SseTransport::new("http://localhost:8000/sse");

    // Build service
    // TODO: Add timeout middleware
    let service = ServiceBuilder::new().service(TransportService::new(transport));

    // Create client
    let client = McpClientImpl::new(service);
    println!("Client created\n");

    // Initialize
    let server_info = client
        .initialize(
            ClientInfo {
                name: "test-client".into(),
                version: "1.0.0".into(),
            },
            ClientCapabilities::default(),
        )
        .await?;
    println!("Connected to server: {server_info:?}\n");

    // Sleep for 100ms to allow the server to start - surprisingly this is required!
    tokio::time::sleep(Duration::from_millis(100)).await;

    // List tools
    let tools = client.list_tools().await?;
    println!("Available tools: {tools:?}\n");

    // Call tool
    let tool_result = client
        .call_tool(
            "echo_tool",
            serde_json::json!({ "message": "Client with SSE transport - calling a tool" }),
        )
        .await?;
    println!("Tool result: {tool_result:?}");

    Ok(())
}
