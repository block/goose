use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use rmcp::model::CallToolRequestParam;
use rmcp::object;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use goose::agents::extension::{Envs, ExtensionConfig};
use goose::agents::extension_manager::ExtensionManager;
use goose::agents::websocket_transport::{WebSocketTransport, WebSocketTransportConfig};

/// Mock WebSocket MCP server for testing
struct MockMcpServer {
    addr: SocketAddr,
    #[allow(dead_code)]
    messages: Arc<Mutex<Vec<String>>>,
}

impl MockMcpServer {
    async fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let messages = messages_clone.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, messages).await {
                        eprintln!("Error handling connection: {}", e);
                    }
                });
            }
        });

        Ok(Self { addr, messages })
    }

    fn url(&self) -> String {
        format!("ws://{}", self.addr)
    }

    #[allow(dead_code)]
    async fn get_messages(&self) -> Vec<String> {
        self.messages.lock().await.clone()
    }
}

async fn handle_connection(stream: TcpStream, messages: Arc<Mutex<Vec<String>>>) -> Result<()> {
    // Accept WebSocket connection with subprotocol validation
    let callback = |req: &Request, mut response: Response| {
        // Check for mcp subprotocol
        if let Some(protocols) = req.headers().get("Sec-WebSocket-Protocol") {
            if protocols.to_str().unwrap_or("").contains("mcp") {
                response
                    .headers_mut()
                    .insert("Sec-WebSocket-Protocol", "mcp".parse().unwrap());
            }
        }
        Ok(response)
    };

    let mut ws_stream = accept_hdr_async(stream, callback).await?;

    // Handle MCP protocol
    while let Some(msg) = ws_stream.next().await {
        match msg? {
            Message::Text(text) => {
                messages.lock().await.push(text.clone());

                // Parse the message to determine response
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                        let id = json.get("id");

                        let response = match method {
                            "initialize" => {
                                serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "protocolVersion": "2024-11-05",
                                        "capabilities": {
                                            "tools": {},
                                            "resources": {},
                                            "prompts": {}
                                        },
                                        "serverInfo": {
                                            "name": "mock-mcp-server",
                                            "version": "1.0.0"
                                        }
                                    }
                                })
                            }
                            "notifications/initialized" => {
                                // No response needed for notifications
                                continue;
                            }
                            "tools/list" => {
                                serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "tools": [
                                            {
                                                "name": "echo",
                                                "description": "Echo back the input",
                                                "inputSchema": {
                                                    "type": "object",
                                                    "properties": {
                                                        "message": {
                                                            "type": "string"
                                                        }
                                                    },
                                                    "required": ["message"]
                                                }
                                            }
                                        ]
                                    }
                                })
                            }
                            "tools/call" => {
                                let tool_name = json
                                    .get("params")
                                    .and_then(|p| p.get("name"))
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown");

                                if tool_name == "echo" {
                                    let message = json
                                        .get("params")
                                        .and_then(|p| p.get("arguments"))
                                        .and_then(|a| a.get("message"))
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("no message");

                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [
                                                {
                                                    "type": "text",
                                                    "text": format!("Echo: {}", message)
                                                }
                                            ]
                                        }
                                    })
                                } else {
                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "error": {
                                            "code": -32601,
                                            "message": "Tool not found"
                                        }
                                    })
                                }
                            }
                            _ => {
                                serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "error": {
                                        "code": -32601,
                                        "message": "Method not found"
                                    }
                                })
                            }
                        };

                        ws_stream.send(Message::Text(response.to_string())).await?;
                    }
                }
            }
            Message::Close(_) => break,
            Message::Ping(data) => {
                ws_stream.send(Message::Pong(data)).await?;
            }
            _ => {}
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_transport_connect() -> Result<()> {
    let server = MockMcpServer::start().await?;

    let config = WebSocketTransportConfig {
        url: server.url().parse()?,
        headers: HashMap::new(),
    };

    let transport = WebSocketTransport::new(config);
    let result = timeout(Duration::from_secs(5), transport.connect()).await;

    assert!(result.is_ok(), "Connection should succeed");
    assert!(result?.is_ok(), "Transport should be created successfully");

    Ok(())
}

#[tokio::test]
async fn test_websocket_transport_invalid_url() {
    let config = WebSocketTransportConfig {
        url: "ws://localhost:99999".parse().unwrap(),
        headers: HashMap::new(),
    };

    let transport = WebSocketTransport::new(config);
    let result = timeout(Duration::from_secs(2), transport.connect()).await;

    // Should timeout or error
    assert!(
        result.is_err() || result.unwrap().is_err(),
        "Connection to invalid URL should fail"
    );
}

#[tokio::test]
async fn test_websocket_tool_call() -> Result<()> {
    let server = MockMcpServer::start().await?;

    let extension_config = ExtensionConfig::WebSocket {
        name: "test-ws".to_string(),
        uri: server.url(),
        envs: Envs::new(HashMap::new()),
        env_keys: vec![],
        headers: HashMap::new(),
        description: "Test WebSocket extension".to_string(),
        timeout: Some(30),
        bundled: None,
        available_tools: vec![],
    };

    let extension_manager = ExtensionManager::new();
    extension_manager.add_extension(extension_config).await?;

    // Wait for initialization
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Call the echo tool
    let tool_call = CallToolRequestParam {
        name: "test-ws__echo".into(),
        arguments: Some(object!({"message": "Hello, WebSocket!"})),
    };

    let result = timeout(
        Duration::from_secs(5),
        extension_manager.dispatch_tool_call(tool_call, CancellationToken::default()),
    )
    .await;

    assert!(result.is_ok(), "Tool call should complete");
    let tool_result = result??;
    let content = tool_result.result.await?;

    // Verify the response
    assert!(!content.is_empty(), "Should have response content");

    // Serialize to check content
    let json = serde_json::to_string(&content)?;
    assert!(
        json.contains("Echo: Hello, WebSocket!"),
        "Response should contain echoed message"
    );

    Ok(())
}

#[tokio::test]
async fn test_websocket_multiple_connections() -> Result<()> {
    let server = MockMcpServer::start().await?;

    // Create multiple connections
    let mut handles = vec![];

    for i in 0..3 {
        let url = server.url();
        let handle = tokio::spawn(async move {
            let config = WebSocketTransportConfig {
                url: url.parse().unwrap(),
                headers: HashMap::new(),
            };

            let transport = WebSocketTransport::new(config);
            let result = transport.connect().await;

            assert!(result.is_ok(), "Connection {} should succeed", i);
        });
        handles.push(handle);
    }

    // Wait for all connections
    for handle in handles {
        timeout(Duration::from_secs(5), handle).await??;
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_subprotocol_negotiation() -> Result<()> {
    let server = MockMcpServer::start().await?;

    let config = WebSocketTransportConfig {
        url: server.url().parse()?,
        headers: HashMap::new(),
    };

    let transport = WebSocketTransport::new(config);
    let result = transport.connect().await;

    // Should succeed with subprotocol negotiation
    assert!(result.is_ok(), "Connection with subprotocol should succeed");

    Ok(())
}

#[tokio::test]
async fn test_websocket_message_ordering() -> Result<()> {
    let server = MockMcpServer::start().await?;

    let extension_config = ExtensionConfig::WebSocket {
        name: "test-ws".to_string(),
        uri: server.url(),
        envs: Envs::new(HashMap::new()),
        env_keys: vec![],
        headers: HashMap::new(),
        description: "Test WebSocket extension".to_string(),
        timeout: Some(30),
        bundled: None,
        available_tools: vec![],
    };

    let extension_manager = ExtensionManager::new();
    extension_manager.add_extension(extension_config).await?;

    // Wait for initialization
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send multiple tool calls in sequence
    for i in 0..5 {
        let tool_call = CallToolRequestParam {
            name: "test-ws__echo".into(),
            arguments: Some(object!({"message": format!("Message {}", i)})),
        };

        let result = timeout(
            Duration::from_secs(5),
            extension_manager.dispatch_tool_call(tool_call, CancellationToken::default()),
        )
        .await;

        assert!(result.is_ok(), "Tool call {} should complete", i);
        let tool_result = result??;
        let content = tool_result.result.await?;

        // Verify the response matches the request
        let json = serde_json::to_string(&content)?;
        assert!(
            json.contains(&format!("Message {}", i)),
            "Response should contain correct message number"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_connection_timeout() {
    // Try to connect to a non-existent server
    let config = WebSocketTransportConfig {
        url: "ws://192.0.2.1:9999".parse().unwrap(), // TEST-NET-1, should timeout
        headers: HashMap::new(),
    };

    let transport = WebSocketTransport::new(config);
    let result = timeout(Duration::from_secs(2), transport.connect()).await;

    // Should timeout
    assert!(result.is_err(), "Connection should timeout");
}

#[tokio::test]
async fn test_websocket_large_message() -> Result<()> {
    let server = MockMcpServer::start().await?;

    let extension_config = ExtensionConfig::WebSocket {
        name: "test-ws".to_string(),
        uri: server.url(),
        envs: Envs::new(HashMap::new()),
        env_keys: vec![],
        headers: HashMap::new(),
        description: "Test WebSocket extension".to_string(),
        timeout: Some(30),
        bundled: None,
        available_tools: vec![],
    };

    let extension_manager = ExtensionManager::new();
    extension_manager.add_extension(extension_config).await?;

    // Wait for initialization
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send a large message
    let large_message = "x".repeat(10000);
    let tool_call = CallToolRequestParam {
        name: "test-ws__echo".into(),
        arguments: Some(object!({"message": large_message.clone()})),
    };

    let result = timeout(
        Duration::from_secs(5),
        extension_manager.dispatch_tool_call(tool_call, CancellationToken::default()),
    )
    .await;

    assert!(result.is_ok(), "Large message should be handled");
    let tool_result = result??;
    let content = tool_result.result.await?;

    // Verify the response
    let json = serde_json::to_string(&content)?;
    assert!(
        json.contains(&large_message),
        "Response should contain large message"
    );

    Ok(())
}
