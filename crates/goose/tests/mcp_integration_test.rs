use std::path::PathBuf;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use serde_json::json;

use goose::agents::extension_manager::ExtensionManager;
use goose::agents::extension::{ExtensionConfig, Envs};
use mcp_core::ToolCall;
use rmcp::model::Content;

#[tokio::test]
async fn test_extension_manager_with_python_mcp_server() {
    // Get the path to the test MCP server
    let mut test_server_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_server_path.push("tests");
    test_server_path.push("test_mcp_server.py");
    
    // Ensure the test server exists
    assert!(test_server_path.exists(), "Test MCP server not found at {:?}", test_server_path);
    
    // Create extension manager
    let mut extension_manager = ExtensionManager::new();
    
    // Create a stdio extension config for the Python MCP server
    let extension_config = ExtensionConfig::Stdio {
        name: "test_python_mcp".to_string(),
        description: Some("Test Python MCP Server".to_string()),
        cmd: "python3".to_string(),
        args: vec![test_server_path.to_string_lossy().to_string()],
        envs: Envs::default(),
        env_keys: vec![],
        timeout: Some(30), // 30 seconds timeout
        bundled: Some(false),
    };
    
    // Add the extension
    let result = extension_manager.add_extension(extension_config).await;
    assert!(result.is_ok(), "Failed to add extension: {:?}", result);
    
    // Wait a moment for the connection to stabilize
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Get the list of tools
    let tools = extension_manager.get_prefixed_tools(None).await;
    assert!(tools.is_ok(), "Failed to get tools: {:?}", tools);
    
    let tools = tools.unwrap();
    assert!(!tools.is_empty(), "No tools found");
    
    // Verify we have the expected tools with proper prefixes
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    println!("Available tools: {:?}", tool_names);
    
    assert!(tool_names.iter().any(|name| name.contains("echo")), 
            "Echo tool not found in: {:?}", tool_names);
    assert!(tool_names.iter().any(|name| name.contains("add_numbers")), 
            "Add numbers tool not found in: {:?}", tool_names);
}

#[tokio::test]
async fn test_tool_call_dispatch_with_python_mcp_server() {
    // Get the path to the test MCP server
    let mut test_server_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_server_path.push("tests");
    test_server_path.push("test_mcp_server.py");
    
    // Create extension manager
    let mut extension_manager = ExtensionManager::new();
    
    // Create a stdio extension config for the Python MCP server
    let extension_config = ExtensionConfig::Stdio {
        name: "test_python_mcp".to_string(),
        description: Some("Test Python MCP Server".to_string()),
        cmd: "python3".to_string(),
        args: vec![test_server_path.to_string_lossy().to_string()],
        envs: Envs::default(),
        env_keys: vec![],
        timeout: Some(30), // 30 seconds timeout
        bundled: Some(false),
    };
    
    // Add the extension
    let result = extension_manager.add_extension(extension_config).await;
    assert!(result.is_ok(), "Failed to add extension: {:?}", result);
    
    // Wait a moment for the connection to stabilize
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Test echo tool call
    let echo_tool_call = ToolCall {
        name: "test_python_mcp__echo".to_string(),
        arguments: json!({
            "message": "Hello, MCP World!"
        }),
    };
    
    let result = extension_manager
        .dispatch_tool_call(echo_tool_call, CancellationToken::default())
        .await;
    
    assert!(result.is_ok(), "Failed to dispatch echo tool call");
    
    let tool_result = result.unwrap();
    let content = tool_result.result.await;
    assert!(content.is_ok(), "Tool execution failed: {:?}", content);
    
    let content = content.unwrap();
    assert!(!content.is_empty(), "Tool result is empty");
    
    // Check that the echo response contains our message
    let content_text = match &content[0] {
        Content::Text { text } => text,
        _ => panic!("Expected text content"),
    };
    assert!(content_text.contains("Hello, MCP World!"), 
            "Echo response doesn't contain expected message: {}", content_text);
    
    // Test add_numbers tool call
    let add_tool_call = ToolCall {
        name: "test_python_mcp__add_numbers".to_string(),
        arguments: json!({
            "a": 5,
            "b": 3
        }),
    };
    
    let result = extension_manager
        .dispatch_tool_call(add_tool_call, CancellationToken::default())
        .await;
    
    assert!(result.is_ok(), "Failed to dispatch add_numbers tool call");
    
    let tool_result = result.unwrap();
    let content = tool_result.result.await;
    assert!(content.is_ok(), "Add numbers tool execution failed: {:?}", content);
    
    let content = content.unwrap();
    assert!(!content.is_empty(), "Add numbers tool result is empty");
    
    // Check that the addition result is correct
    let content_text = match &content[0] {
        Content::Text { text } => text,
        _ => panic!("Expected text content"),
    };
    assert!(content_text.contains("8"), 
            "Add numbers response doesn't contain expected result: {}", content_text);
}

#[tokio::test]
async fn test_invalid_tool_call_with_python_mcp_server() {
    // Get the path to the test MCP server
    let mut test_server_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_server_path.push("tests");
    test_server_path.push("test_mcp_server.py");
    
    // Create extension manager
    let mut extension_manager = ExtensionManager::new();
    
    // Create a stdio extension config for the Python MCP server
    let extension_config = ExtensionConfig::Stdio {
        name: "test_python_mcp".to_string(),
        description: Some("Test Python MCP Server".to_string()),
        cmd: "python3".to_string(),
        args: vec![test_server_path.to_string_lossy().to_string()],
        envs: Envs::default(),
        env_keys: vec![],
        timeout: Some(30), // 30 seconds timeout
        bundled: Some(false),
    };
    
    // Add the extension
    let result = extension_manager.add_extension(extension_config).await;
    assert!(result.is_ok(), "Failed to add extension: {:?}", result);
    
    // Wait a moment for the connection to stabilize
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Test calling a tool that doesn't exist
    let invalid_tool_call = ToolCall {
        name: "test_python_mcp__nonexistent_tool".to_string(),
        arguments: json!({}),
    };
    
    let result = extension_manager
        .dispatch_tool_call(invalid_tool_call, CancellationToken::default())
        .await;
    
    assert!(result.is_ok(), "Tool dispatch should succeed even if tool execution fails");
    
    let tool_result = result.unwrap();
    let content = tool_result.result.await;
    assert!(content.is_err(), "Expected tool execution to fail for nonexistent tool");
    
    // Test calling echo tool with missing parameters
    let invalid_params_tool_call = ToolCall {
        name: "test_python_mcp__echo".to_string(),
        arguments: json!({}), // Missing required 'message' parameter
    };
    
    let result = extension_manager
        .dispatch_tool_call(invalid_params_tool_call, CancellationToken::default())
        .await;
    
    assert!(result.is_ok(), "Tool dispatch should succeed");
    
    let tool_result = result.unwrap();
    let content = tool_result.result.await;
    // This might succeed or fail depending on how the Python server handles missing params
    // The important thing is that the dispatch mechanism works
    println!("Result with missing params: {:?}", content);
}