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

    // Create example headers
    let mut headers = HashMap::new();
    headers.insert("X-Custom-Header".to_string(), "example-value".to_string());
    headers.insert(
        "User-Agent".to_string(),
        "MCP-StreamableHttp-Client/1.0".to_string(),
    );

    // Create the Streamable HTTP transport with headers
    let transport =
        StreamableHttpTransport::with_headers("http://localhost:8000/mcp", HashMap::new(), headers);

    // Start transport
    let handle = transport.start().await?;

    // Create client
    let mut client = McpClient::connect(handle, Duration::from_secs(10)).await?;
    println!("Client created with Streamable HTTP transport\n");

    // Initialize
    let server_info = client
        .initialize(
            ClientInfo {
                name: "streamable-http-client".into(),
                version: "1.0.0".into(),
            },
            ClientCapabilities::default(),
        )
        .await?;
    println!("Connected to server: {server_info:?}\n");

    // Give the server a moment to fully initialize
    tokio::time::sleep(Duration::from_millis(500)).await;

    // List tools
    let tools = client.list_tools(None).await?;
    println!("Available tools: {tools:?}\n");

    // Call tool if available
    if !tools.tools.is_empty() {
        let tool_result = client
            .call_tool(
                &tools.tools[0].name,
                serde_json::json!({ "message": "Hello from Streamable HTTP transport!" }),
            )
            .await?;
        println!("Tool result: {tool_result:?}\n");
    }

    // List resources
    let resources = client.list_resources(None).await?;
    println!("Resources: {resources:?}\n");

    // Read resource if available
    if !resources.resources.is_empty() {
        let resource = client.read_resource(&resources.resources[0].uri).await?;
        println!("Resource content: {resource:?}\n");
    }

    // List prompts
    let prompts = client.list_prompts(None).await?;
    println!("Available prompts: {prompts:?}\n");

    // Get prompt if available
    if !prompts.prompts.is_empty() {
        let prompt_result = client
            .get_prompt(&prompts.prompts[0].name, serde_json::json!({}))
            .await?;
        println!("Prompt result: {prompt_result:?}\n");
    }

    println!("Streamable HTTP transport example completed successfully!");

    Ok(())
}
