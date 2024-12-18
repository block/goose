use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use mcp_client::{
    client::{ClientCapabilities, ClientInfo, McpClient},
    service::{ServiceError, TransportService},
    transport::StdioTransport,
};
use tower::{ServiceBuilder, ServiceExt};
use tower::timeout::TimeoutLayer;
use tracing_subscriber::EnvFilter;
use tower::util::BoxService;
use mcp_core::protocol::JsonRpcMessage;

// Define a type alias for the boxed service using BoxService
type BoxedService = BoxService<JsonRpcMessage, JsonRpcMessage, ServiceError>;

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

    // Initialize both clients
    let mut clients: Vec<McpClient<BoxedService>> = vec![client1, client2];

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
) -> Result<McpClient<BoxService<JsonRpcMessage, JsonRpcMessage, ServiceError>>, Box<dyn std::error::Error>> {
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
        })
        .boxed(); // Box the service to create a BoxService

    // Create the client
    Ok(McpClient::new(service))
}
