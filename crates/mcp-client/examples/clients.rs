use mcp_client::{
    client::{ClientCapabilities, ClientInfo, McpClient, McpClientImpl},
    service::TransportService,
    transport::{SseTransport, StdioTransport},
};
use rand::Rng;
use rand::SeedableRng;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("mcp_client=debug".parse().unwrap()),
        )
        .init();

    // Create two separate clients with stdio transport
    let client1 = create_stdio_client("client1", "1.0.0")?;
    let client2 = create_stdio_client("client2", "1.0.0")?;
    let client3 = create_sse_client("client3", "1.0.0")?;

    // Initialize both clients
    let mut clients: Vec<Box<dyn McpClient + Send + Sync>> = Vec::new();
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

    // List tools for all clients
    for (i, client) in clients.iter_mut().enumerate() {
        let tools = client.list_tools().await?;
        println!("\nClient {} tools: {:?}", i + 1, tools);
    }

    println!("\n\n----------------------------------\n\n");

    // Wrap clients in Arc before spawning tasks
    let clients = Arc::new(clients);
    let mut handles = vec![];

    for i in 0..20 {
        let clients = Arc::clone(&clients);
        let handle = tokio::spawn(async move {
            // let mut rng = rand::thread_rng();
            let mut rng = rand::rngs::StdRng::from_entropy();
            tokio::time::sleep(Duration::from_millis(rng.gen_range(5..50))).await;

            // Randomly select an operation
            match rng.gen_range(0..4) {
                0 => {
                    println!("\n{i}: Listing tools for client 1 (stdio)");
                    match clients[0].list_tools().await {
                        Ok(tools) => {
                            println!("  {i}: -> Got tools, first one: {:?}", tools.tools.first())
                        }
                        Err(e) => println!("  {i}: -> Error: {}", e),
                    }
                }
                1 => {
                    println!("\n{i}: Listing tools for client 3 (sse)");
                    match clients[2].list_tools().await {
                        Ok(tools) => {
                            println!("  {i}: -> Got tools, first one: {:?}", tools.tools.first())
                        }
                        Err(e) => println!("  {i}: -> Error: {}", e),
                    }
                }
                2 => {
                    println!("\n{i}: Calling tool for client 2 (stdio)");
                    match clients[1]
                        .call_tool("git_status", serde_json::json!({ "repo_path": "." }))
                        .await
                    {
                        Ok(result) => println!(
                            "  {i}: -> Tool execution result, is_error: {:?}",
                            result.is_error
                        ),
                        Err(e) => println!("  {i}: -> Error: {}", e),
                    }
                }
                3 => {
                    println!("\n{i}: Calling tool for client 3 (sse)");
                    match clients[2]
                            .call_tool(
                                "echo_tool",
                                serde_json::json!({ "message": "Client with SSE transport - calling a tool" }),
                            )
                            .await
                        {
                        Ok(result) => println!("  {i}: -> Tool execution result, is_error: {:?}", result.is_error),
                        Err(e) => println!("  {i}: -> Error: {}", e),
                    }
                }
                _ => unreachable!(),
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    Ok(())
}

fn create_stdio_client(
    _name: &str,
    _version: &str,
) -> Result<Box<dyn McpClient + Send + Sync>, Box<dyn std::error::Error>> {
    let transport = StdioTransport::new("uvx", vec!["mcp-server-git".to_string()]);
    // TODO: Add timeout middleware
    let service = ServiceBuilder::new().service(TransportService::new(transport));
    Ok(Box::new(McpClientImpl::new(service)))
}

fn create_sse_client(
    _name: &str,
    _version: &str,
) -> Result<Box<dyn McpClient + Send + Sync>, Box<dyn std::error::Error>> {
    let transport = SseTransport::new("http://localhost:8000/sse");
    // TODO: Add timeout middleware
    let service = ServiceBuilder::new().service(TransportService::new(transport));
    Ok(Box::new(McpClientImpl::new(service)))
}
