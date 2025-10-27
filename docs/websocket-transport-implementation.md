# WebSocket Transport Implementation for Goose MCP Client

## Overview

This document describes the implementation of WebSocket transport for the Goose MCP client, compatible with the Kotlin SDK's WebSocket server implementation.

## Architecture

### Components

1. **WebSocket Transport Module** (`crates/goose/src/agents/websocket_transport.rs`)
   - Handles WebSocket connection lifecycle
   - Manages bidirectional JSON-RPC communication
   - Provides error handling and recovery

2. **Extension Configuration** (`crates/goose/src/agents/extension.rs`)
   - New `WebSocket` variant in `ExtensionConfig` enum
   - Configuration includes URI, headers, timeout, and environment variables

3. **Extension Manager Integration** (`crates/goose/src/agents/extension_manager.rs`)
   - Handles WebSocket client initialization
   - Routes tool calls to WebSocket-connected servers

## Protocol Details

### WebSocket Connection

- **URL Format**: `ws://` or `wss://` (converted from `http://` or `https://`)
- **Default Path**: `/mcp` (standard MCP WebSocket endpoint)
- **Message Format**: Text frames containing JSON-RPC messages
- **Communication**: Bidirectional (both client and server can initiate requests)

## Configuration Example

### YAML Configuration

```yaml
extensions:
  - type: websocket
    name: my-kotlin-server
    description: "MCP server running on Kotlin SDK"
    uri: "http://localhost:8080/mcp"
    timeout: 30
    headers:
      Authorization: "Bearer token123"
```

### JSON Configuration

```json
{
  "type": "websocket",
  "name": "my-kotlin-server",
  "description": "MCP server running on Kotlin SDK",
  "uri": "http://localhost:8080/mcp",
  "timeout": 30
}
```

## Dependencies

```toml
tokio-tungstenite = { version = "0.24", features = ["native-tls"] }
futures-util = "0.3"
```

## Next Steps

See TODO.md for remaining implementation tasks.
