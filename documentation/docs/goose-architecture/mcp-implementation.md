---
title: Model Context Protocol Implementation
sidebar_position: 5
---

# Model Context Protocol Implementation

The Model Context Protocol (MCP) is a standardized protocol for connecting LLMs to tools and data sources. This document explains the MCP implementation in Goose, including message formats, tool definitions, and resource management.

## Protocol Overview

MCP uses JSON-RPC as its underlying protocol and defines several message types:

1. **Requests**: Messages sent from the client to the server
2. **Responses**: Messages sent from the server to the client in response to requests
3. **Notifications**: One-way messages that don't require a response
4. **Errors**: Error messages returned when a request fails

## Core Components

The MCP implementation consists of several crates:

1. **mcp-core**: Defines the core protocol types and interfaces
2. **mcp-client**: Implements the client side of the protocol
3. **mcp-server**: Implements the server side of the protocol
4. **mcp-macros**: Provides macros for simplifying MCP implementation

## Message Types

### Content

MCP defines several content types that can be exchanged:

```rust
pub enum Content {
    Text(TextContent),
    Image(ImageContent),
    Resource(EmbeddedResource),
}
```

### Tools

Tools are defined with a name, description, and input schema:

```rust
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
```

Tool calls include the tool name and arguments:

```rust
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}
```

### Resources

Resources allow extensions to share data with LLMs:

```rust
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub metadata: Value,
}

pub enum ResourceContents {
    TextResourceContents {
        uri: String,
        mime_type: Option<String>,
        text: String,
    },
    BlobResourceContents {
        uri: String,
        mime_type: Option<String>,
        blob: String,
    },
}
```

## Protocol Implementation

The protocol is implemented using JSON-RPC with custom message types:

```rust
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
    Error(ErrorResponse),
}
```

### Server Capabilities

Servers advertise their capabilities during initialization:

```rust
pub struct ServerCapabilities {
    pub prompts: Option<PromptsCapability>,
    pub resources: Option<ResourcesCapability>,
    pub tools: Option<ToolsCapability>,
}
```

### Client Capabilities

Clients advertise their capabilities during initialization:

```rust
pub struct ClientCapabilities {
    pub supports_tool_confirmation: bool,
}
```

## Transport Mechanisms

MCP supports multiple transport mechanisms:

1. **Standard I/O**: Communication via stdin/stdout
2. **Server-Sent Events (SSE)**: Communication via HTTP SSE

```rust
pub trait Transport: Send + Sync {
    fn start(&self) -> Result<TransportHandle>;
}

pub enum TransportHandle {
    Stdio(StdioHandle),
    Sse(SseHandle),
}
```

## Error Handling

MCP defines a comprehensive error handling system:

```rust
pub struct ErrorData {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}
```

Error codes are standardized to allow consistent error handling across implementations.

## Tool Execution Flow

The tool execution flow in MCP:

1. Client sends a request to list available tools
2. Server responds with a list of tools
3. Client sends a request to call a tool with arguments
4. Server executes the tool and returns the result

## Resource Management

The resource management flow in MCP:

1. Client sends a request to list available resources
2. Server responds with a list of resources
3. Client sends a request to read a resource
4. Server returns the resource contents

## Prompt Management

MCP supports prompt management:

```rust
pub struct Prompt {
    pub name: String,
    pub description: String,
    pub argument_schema: Value,
}
```

Prompts can be listed and retrieved with arguments to generate dynamic prompt text.

## Implementation Details

### Message Deserialization

MCP implements flexible deserialization to handle different JSON-RPC message formats:

```rust
impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Implementation details
    }
}
```

### Tool Registration

Tools are registered with the server and can be discovered by clients:

```rust
pub trait McpServerTrait: Send + Sync {
    async fn register_tool(&self, tool: Tool) -> Result<(), Error>;
}
```

## Best Practices

1. **Clear Tool Definitions**: Define tools with clear names, descriptions, and schemas
2. **Efficient Resource Management**: Use resources for sharing large data
3. **Proper Error Handling**: Return specific error codes and messages
4. **Transport Independence**: Design extensions to work with any transport mechanism
