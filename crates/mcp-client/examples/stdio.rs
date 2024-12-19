use anyhow::Result;
use mcp_client::client::{
    ClientCapabilities, ClientInfo, Error as ClientError, McpClient, McpClientImpl,
};
use mcp_client::{
    service::{ServiceError, TransportService},
    transport::StdioTransport,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tower::timeout::TimeoutLayer;
use tower::{ServiceBuilder, ServiceExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("mcp_client=debug".parse().unwrap())
                .add_directive("reqwest_eventsource=debug".parse().unwrap()),
        )
        .init();

    // Create the base transport as Arc<Mutex<StdioTransport>>
    let transport = Arc::new(Mutex::new(StdioTransport::new("uvx", ["mcp-server-git"])));

    // Build service with middleware including timeout
    let service = ServiceBuilder::new()
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .service(TransportService::new(Arc::clone(&transport)))
        .map_err(|e: Box<dyn std::error::Error + Send + Sync>| {
            if e.is::<tower::timeout::error::Elapsed>() {
                ServiceError::Timeout(tower::timeout::error::Elapsed::new())
            } else {
                ServiceError::Other(e.to_string())
            }
        });

    // Create client
    let mut client = McpClientImpl::new(service);

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

    // Call tool 'git_status' wtih arguments = {"repo_path": "."}
    let tool_result = client
        .call_tool("git_status", serde_json::json!({ "repo_path": "." }))
        .await?;
    println!("Tool result: {tool_result:?}");

    Ok(())
}
