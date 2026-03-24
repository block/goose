---
sidebar_position: 2
---

# Extensions Design

This document describes the design and implementation of the [Extensions framework](/docs/getting-started/using-extensions) in goose, which enables AI agents to interact with different extensions through a unified tool-based interface.

## Core Concepts

### Extension
An Extension represents any component that can be operated by an AI agent. Extensions expose their capabilities through Tools and maintain their own state. The core interface is defined by the `Extension` trait:

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn instructions(&self) -> &str;
    fn tools(&self) -> &[Tool];
    async fn status(&self) -> AnyhowResult<HashMap<String, Value>>;
    async fn call_tool(&self, tool_name: &str, parameters: HashMap<String, Value>) -> ToolResult<Value>;
}
```

### Tools
Tools are the primary way Extensions expose functionality to agents. Each tool has:
- A name
- A description
- A set of parameters
- An implementation that executes the tool's functionality

A tool must take a Value and return an `AgentResult<Value>` (it must also be async). This
is what makes it compatible with the tool calling framework from the agent. 

```rust
async fn echo(&self, params: Value) -> AgentResult<Value>
```

## Architecture

### Component Overview

1. **Extension Trait**: The core interface that all extensions must implement
2. **Error Handling**: Specialized error types for tool execution
3. **Proc Macros**: Simplify tool definition and registration [*not yet implemented*]

### Error Handling

The system uses two main error types:
- `ErrorData`: Specific errors related to tool execution
- `anyhow::Error`: General purpose errors for extension status and other operations

This split allows precise error handling for tool execution while maintaining flexibility for general extension operations.

## Best Practices

### Tool Design

1. **Clear Names**: Use clear, action-oriented names for tools (e.g., "create_user" not "user")
2. **Descriptive Parameters**: Each parameter should have a clear description
3. **Error Handling**: Return specific errors when possible, the errors become "prompts"
4. **State Management**: Be explicit about state modifications

### Extension Implementation

1. **State Encapsulation**: Keep extension state private and controlled
2. **Error Propagation**: Use `?` operator with `ErrorData` for tool execution
3. **Status Clarity**: Provide clear, structured status information
4. **Documentation**: Document all tools and their effects

### Example Implementation

Here's a complete example of a simple extension:

```rust
use goose_macros::tool;

struct FileSystem {
    registry: ToolRegistry,
    root_path: PathBuf,
}

impl FileSystem {
    #[tool(
        name = "read_file",
        description = "Read contents of a file"
    )]
    async fn read_file(&self, path: String) -> ToolResult<Value> {
        let full_path = self.root_path.join(path);
        let content = tokio::fs::read_to_string(full_path)
            .await
            .map_err(|e| ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from(e.to_string(),
                data: None,
            }))?;
            
        Ok(json!({ "content": content }))
    }
}

#[async_trait]
impl Extension for FileSystem {
    // ... implement trait methods ...
}
```

## Testing

Extensions should be tested at multiple levels:
1. Unit tests for individual tools
2. Integration tests for extension behavior
3. Property tests for tool invariants

Example test:
```rust
#[tokio::test]
async fn test_echo_tool() {
    let extension = TestExtension::new();
    let result = extension.call_tool(
        "echo",
        hashmap!{ "message" => json!("hello") }
    ).await;
    
    assert_eq!(result.unwrap(), json!({ "response": "hello" }));
}
```