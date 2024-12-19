use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use mcp_client::{
    client::{ClientCapabilities, ClientInfo, McpClient, McpClientImpl},
    service::{ServiceError, TransportService},
    transport::{SseTransport, StdioTransport},
};
use mcp_core::protocol::JsonRpcMessage;
use tower::timeout::TimeoutLayer;
use tower::util::BoxService;
use tower::{ServiceBuilder, ServiceExt};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("mcp_client=debug".parse().unwrap())
                .add_directive("reqwest_eventsource=debug".parse().unwrap()),
        )
        .init();

    // Create two separate clients with stdio transport
    let client1 = create_client("client1", "1.0.0")?;
    let client2 = create_client("client2", "1.0.0")?;
    let client3 = create_sse_client("client3", "1.0.0")?;

    // Initialize both clients
    let mut clients: Vec<Box<dyn McpClient>> = Vec::new();
    clients.push(client1);
    clients.push(client2);
    clients.push(client3);

    // Initialize all clients
    for (i, client) in clients.iter_mut().enumerate() {
        let info = ClientInfo {
            name: format!("example-client-{}", i + 1),
            version: "1.0.0".to_string(),
        };
        let capabilities = ClientCapabilities::default();

        println!("\nInitializing client {}", i + 1);
        let init_result = client.initialize(info, capabilities).await?;
        println!("Client {} initialized: {:?}", i + 1, init_result);
    }

    // List tools for each client
    for (i, client) in clients.iter_mut().enumerate() {
        let tools = client.list_tools().await?;
        println!("\nClient {} tools: {:?}", i + 1, tools);
    }

    Ok(())
}

fn create_client(
    _name: &str,
    _version: &str,
) -> Result<Box<dyn McpClient>, Box<dyn std::error::Error>> {
    // Create the transport
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

    Ok(Box::new(McpClientImpl::new(service)))
}

fn create_sse_client(
    _name: &str,
    _version: &str,
) -> Result<Box<dyn McpClient>, Box<dyn std::error::Error>> {
    let transport = Arc::new(Mutex::new(
        SseTransport::new("http://localhost:8000/sse").unwrap(),
    ));

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

    Ok(Box::new(McpClientImpl::new(service)))
}
