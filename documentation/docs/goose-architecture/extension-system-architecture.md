---
title: Extension System Architecture
sidebar_position: 3
---

# Extension System Architecture

The extension system is a core component of Goose that enables modular addition of capabilities through tools. This document provides a detailed explanation of the extension system architecture, implementation, and best practices.

## Core Concepts

### Extension

An Extension represents a component that can be operated by an AI agent. Extensions expose their capabilities through Tools and maintain their own state. The core interface is defined by the `Extension` trait:

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

A tool must take a Value and return an `AgentResult<Value>` (it must also be async).

## Extension Types

Goose supports three types of extensions:

1. **Built-in Extensions**: Part of the Goose binary, implemented in the `goose-mcp` crate.
2. **Standard I/O Extensions**: External processes that communicate with Goose via stdin/stdout.
3. **Server-Sent Events (SSE) Extensions**: External servers that communicate with Goose via SSE.

## Extension Configuration

Extensions are configured through the `ExtensionConfig` enum:

```rust
pub enum ExtensionConfig {
    Sse {
        name: String,
        uri: String,
        envs: Envs,
        timeout: Option<u64>,
    },
    Stdio {
        name: String,
        cmd: String,
        args: Vec<String>,
        envs: Envs,
        timeout: Option<u64>,
    },
    Builtin {
        name: String,
        timeout: Option<u64>,
    },
}
```

## Extension Dependencies

Extensions can depend on each other, allowing for modular composition of capabilities. Dependencies are specified in the profile configuration:

```yaml
extensions:
  - developer
  - calendar
  - contacts
  - name: scheduling
    requires:
      assistant: assistant
      calendar: calendar
      contacts: contacts
```

## Extension Registration and Discovery

Extensions are registered with the agent through the `add_extension` method:

```rust
async fn add_extension(&mut self, extension: ExtensionConfig) -> ExtensionResult<()>;
```

The agent maintains a registry of extensions and their tools, which can be queried at runtime.

## Tool Execution Flow

When the LLM generates a tool call, the following sequence occurs:

1. The agent identifies the extension and tool from the tool name
2. The agent dispatches the tool call to the appropriate extension
3. The extension executes the tool and returns a result
4. The agent formats the result and sends it back to the LLM

## Error Handling

The extension system uses specialized error types for tool execution:

```rust
pub enum ExtensionError {
    Initialization(ExtensionConfig, ClientError),
    Client(ClientError),
    ContextLengthExceeded,
    Transport(TransportError),
}
```

Errors are propagated back to the LLM to enable self-correction.

## Built-in Extensions

Goose includes several built-in extensions:

1. **Developer**: Provides tools for software development tasks
2. **Memory**: Enables persistent memory across sessions
3. **Computer Controller**: Allows interaction with the local computer
4. **Google Drive**: Provides access to Google Drive
5. **JetBrains**: Integrates with JetBrains IDEs

## Custom Extensions

Custom extensions can be created by implementing the MCP protocol. Extensions can be written in any language that can communicate via stdin/stdout or HTTP.

## Best Practices

### Tool Design

1. **Clear Names**: Use clear, action-oriented names for tools
2. **Descriptive Parameters**: Each parameter should have a clear description
3. **Error Handling**: Return specific errors when possible
4. **State Management**: Be explicit about state modifications

### Extension Implementation

1. **State Encapsulation**: Keep extension state private and controlled
2. **Error Propagation**: Use `?` operator with `ToolError` for tool execution
3. **Status Clarity**: Provide clear, structured status information
4. **Documentation**: Document all tools and their effects
