// This example shows how to use the mcp-client crate to interact with a server that has a simple counter tool.
// The server is started by running `cargo run -p mcp-server` in the root of the mcp-server crate.
use anyhow::Result;
use mcp_client::client::{
    ClientCapabilities, ClientInfo, Error as ClientError, McpClient, McpClientImpl,
};
use mcp_client::{service::TransportService, transport::StdioTransport};
use tower::ServiceBuilder;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("mcp_client=debug".parse().unwrap())
                .add_directive("eventsource_client=debug".parse().unwrap()),
        )
        .init();

    // Create the transport
    let transport = StdioTransport::new(
        "cargo",
        vec!["run", "-p", "mcp-server"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
    );

    // Build service
    // TODO: Add timeout middleware
    let service = ServiceBuilder::new().service(TransportService::new(transport));

    // Create client
    let client = McpClientImpl::new(service);

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

    // List tools
    let tools = client.list_tools().await?;
    println!("Available tools: {tools:?}\n");

    // Call tool 'increment' tool 3 times
    for _ in 0..3 {
        let increment_result = client.call_tool("increment", serde_json::json!({})).await?;
        println!("Tool result for 'increment': {increment_result:?}\n");
    }

    // Call tool 'get_value'
    let get_value_result = client.call_tool("get_value", serde_json::json!({})).await?;
    println!("Tool result for 'get_value': {get_value_result:?}\n");

    // Call tool 'decrement' once
    let decrement_result = client.call_tool("decrement", serde_json::json!({})).await?;
    println!("Tool result for 'decrement': {decrement_result:?}\n");

    // Call tool 'get_value'
    let get_value_result = client.call_tool("get_value", serde_json::json!({})).await?;
    println!("Tool result for 'get_value': {get_value_result:?}\n");

    Ok(())
}