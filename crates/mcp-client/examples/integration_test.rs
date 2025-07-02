use anyhow::Result;
use futures::lock::Mutex;
use mcp_client::client::{ClientCapabilities, ClientInfo, McpClient, McpClientTrait};
use mcp_client::transport::{SseTransport, StreamableHttpTransport, Transport};
use mcp_client::StdioTransport;
use std::collections::HashMap;
use std::sync::Arc;
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

    test_transport(sse_transport().await?).await?;
    test_transport(streamable_http_transport().await?).await?;
    test_transport(stdio_transport().await?).await?;

    // Test broken transport
    match test_transport(broken_stdio_transport().await?).await {
        Ok(_) => panic!("Expected an error but got success"),
        Err(e) => {
            assert!(e
                .to_string()
                .contains("error: package(s) `thispackagedoesnotexist` not found in workspace"));
            println!("Expected error occurred: {e}");
        }
    }

    Ok(())
}

async fn sse_transport() -> Result<SseTransport> {
    let port = "60053";

    tokio::process::Command::new("npx")
        .env("PORT", port)
        .arg("@modelcontextprotocol/server-everything")
        .arg("sse")
        .spawn()?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(SseTransport::new(
        format!("http://localhost:{}/sse", port),
        HashMap::new(),
    ))
}

async fn streamable_http_transport() -> Result<StreamableHttpTransport> {
    let port = "60054";

    tokio::process::Command::new("npx")
        .env("PORT", port)
        .arg("@modelcontextprotocol/server-everything")
        .arg("streamable-http")
        .spawn()?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(StreamableHttpTransport::new(
        format!("http://localhost:{}/mcp", port),
        HashMap::new(),
    ))
}

async fn stdio_transport() -> Result<StdioTransport> {
    Ok(StdioTransport::new(
        "npx",
        vec!["@modelcontextprotocol/server-everything"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        HashMap::new(),
    ))
}

async fn broken_stdio_transport() -> Result<StdioTransport> {
    Ok(StdioTransport::new(
        "cargo",
        vec!["run", "-p", "thispackagedoesnotexist"]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        HashMap::new(),
    ))
}

async fn test_transport<T>(transport: T) -> Result<()>
where
    T: Transport + Send + 'static,
{
    // Start transport
    let handle = transport.start().await?;

    // Create client
    let mut client = McpClient::connect(handle, Duration::from_secs(10)).await?;
    println!("Client created\n");

    let mut receiver = client.subscribe().await;
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            println!("Received event: {event:?}");
            events_clone.lock().await.push(event);
        }
    });

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
    println!("Available tools: {tools:#?}\n");

    // Call tool
    let tool_result = client
        .call_tool("echo", serde_json::json!({ "message": "honk" }))
        .await?;
    println!("Tool result: {tool_result:#?}\n");

    let collected_eventes_before = events.lock().await.len();
    let n_steps = 5;
    let long_op = client
        .call_tool(
            "longRunningOperation",
            serde_json::json!({ "duration": 3, "steps": n_steps }),
        )
        .await?;
    println!("Long op result: {long_op:#?}\n");
    let collected_events_after = events.lock().await.len();
    assert_eq!(collected_events_after - collected_eventes_before, n_steps);

    let error_result = client
        .call_tool("add", serde_json::json!({ "a": "foo", "b": "bar" }))
        .await;
    assert!(error_result.is_err());
    println!("Error result: {error_result:#?}\n");

    // List resources
    let resources = client.list_resources(None).await?;
    println!("Resources: {resources:#?}\n");

    // Read resource
    let resource = client.read_resource("test://static/resource/1").await?;
    println!("Resource: {resource:#?}\n");

    Ok(())
}
