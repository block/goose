use anyhow::Result;
use mcp_client::client::{ClientCapabilities, ClientInfo, McpClient, McpClientTrait};
use mcp_client::transport::{SseTransport, Transport};
use mcp_client::McpService;
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

    // Create the base transport
    let transport = SseTransport::new("http://localhost:8000/sse", HashMap::new());

    // Start transport
    let handle = transport.start().await?;

    // Create the service with timeout middleware
    let service = McpService::with_timeout(handle, Duration::from_secs(3));

    // Create client
    let mut client = McpClient::new(service);
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
    tokio::time::sleep(Duration::from_millis(500)).await;

    // List tools
    let tools = client.list_tools(None).await?;
    println!("Available tools: {tools:?}\n");

    // Call tool
    let tool_result = client
        .call_tool(
            "echo_tool",
            serde_json::json!({ "message": "Client with SSE transport - calling a tool" }),
        )
        .await?;
    println!("Tool result: {tool_result:?}\n");

    // List resources
    let resources = client.list_resources(None).await?;
    println!("Resources: {resources:?}\n");

    // Read resource
    let resource = client.read_resource("echo://fixedresource").await?;
    println!("Resource: {resource:?}\n");

    Ok(())
}
