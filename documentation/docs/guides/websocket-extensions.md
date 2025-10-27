---
sidebar_position: 90
title: WebSocket Extensions
sidebar_label: WebSocket Extensions
---

# WebSocket Extensions

WebSocket extensions provide a high-performance, bidirectional communication channel between Goose and MCP servers. This guide covers everything you need to know about using and developing WebSocket-based extensions.

## Overview

WebSocket is a protocol that provides full-duplex communication over a single TCP connection. Unlike traditional HTTP-based transports (SSE, Streaming HTTP), WebSocket enables:

- **Bidirectional communication**: Both client and server can initiate requests
- **Low latency**: Persistent connections eliminate handshake overhead
- **Real-time updates**: Server can push notifications and updates instantly
- **Better performance**: Ideal for high-frequency tool calls and long-running sessions

## When to Use WebSocket

Consider using WebSocket extensions when:

- You need low-latency, real-time communication
- Your server needs to initiate requests (sampling, notifications)
- You're making frequent tool calls that benefit from persistent connections
- You're integrating with existing WebSocket-based services
- You need better performance than HTTP-based transports

## Quick Start

### Adding a WebSocket Extension

#### Via CLI

```bash
goose configure
```

Then select:
1. `Add Extension`
2. `Remote Extension (WebSocket)`
3. Enter the extension details:
   - **Name**: A descriptive name for your extension
   - **URI**: The WebSocket endpoint (e.g., `ws://localhost:8080/mcp`)
   - **Timeout**: Maximum wait time in seconds (default: 300)
   - **Description**: What the extension does
   - **Environment Variables**: Any required credentials or configuration

#### Via Desktop UI

1. Click the sidebar menu button
2. Select `Extensions`
3. Click `Add custom extension`
4. Choose `WebSocket` as the type
5. Fill in the extension details
6. Click `Add`

#### Via Configuration File

Edit `~/.config/goose/config.yaml`:

```yaml
extensions:
  my_websocket_extension:
    enabled: true
    name: "My WebSocket Extension"
    type: websocket
    uri: "ws://localhost:8080/mcp"
    timeout: 300
    description: "WebSocket-based MCP server"
    env_keys: []
    envs: {}
    headers: {}
```

### Starting a Session with WebSocket Extension

```bash
# Start a session with a WebSocket extension
goose session --with-websocket-extension "ws://localhost:8080/mcp"

# With secure WebSocket (WSS)
goose session --with-websocket-extension "wss://example.com/mcp"

# Multiple WebSocket extensions
goose session \
  --with-websocket-extension "ws://localhost:8080/mcp" \
  --with-websocket-extension "wss://api.example.com/mcp"
```

## Configuration Options

### Basic Configuration

```yaml
extensions:
  example:
    type: websocket
    uri: "ws://localhost:8080/mcp"  # Required: WebSocket endpoint
    timeout: 300                     # Optional: Timeout in seconds
    enabled: true                    # Optional: Enable/disable
    name: "Example Extension"        # Optional: Display name
    description: "Example MCP server" # Optional: Description
```

### With Authentication

```yaml
extensions:
  secure_example:
    type: websocket
    uri: "wss://api.example.com/mcp"
    timeout: 300
    headers:
      Authorization: "Bearer YOUR_TOKEN_HERE"
      X-API-Key: "your-api-key"
```

### With Environment Variables

```yaml
extensions:
  env_example:
    type: websocket
    uri: "wss://api.example.com/mcp"
    timeout: 300
    env_keys: ["API_TOKEN", "API_KEY"]
    envs: {}
```

Then set the environment variables:

```bash
export API_TOKEN="your-token"
export API_KEY="your-key"
```

## Protocol Requirements

### WebSocket Subprotocol

The WebSocket server **must** support the `mcp` subprotocol. During the handshake:

1. Client sends: `Sec-WebSocket-Protocol: mcp`
2. Server must echo: `Sec-WebSocket-Protocol: mcp`

### Message Format

Messages are exchanged as JSON-RPC 2.0 formatted text frames:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "example_tool",
    "arguments": {}
  }
}
```

### Connection Flow

1. **Handshake**: Client initiates WebSocket connection with `mcp` subprotocol
2. **Initialization**: Client sends `initialize` request
3. **Ready**: Server responds with capabilities
4. **Communication**: Bidirectional JSON-RPC messages
5. **Cleanup**: Either side can close the connection

## Example Server Implementation

### Kotlin (Misk Framework)

```kotlin
import misk.web.actions.WebSocket
import misk.web.actions.WebSocketListener

class McpWebSocketAction : WebSocket {
    override fun webSocket(listener: WebSocketListener) {
        // Validate subprotocol
        val protocol = listener.request.headers["Sec-WebSocket-Protocol"]
        if (protocol != "mcp") {
            listener.close(1002, "Invalid subprotocol")
            return
        }
        
        // Accept the subprotocol
        listener.response.headers["Sec-WebSocket-Protocol"] = "mcp"
        
        // Handle incoming messages
        listener.onMessage { text ->
            val request = parseJsonRpc(text)
            val response = handleRequest(request)
            listener.send(serializeJsonRpc(response))
        }
        
        listener.onClose { code, reason ->
            cleanup()
        }
    }
}
```

### Node.js (ws library)

```javascript
const WebSocket = require('ws');

const wss = new WebSocket.Server({ 
    port: 8080,
    handleProtocols: (protocols) => {
        // Accept only 'mcp' subprotocol
        if (protocols.includes('mcp')) {
            return 'mcp';
        }
        return false;
    }
});

wss.on('connection', (ws) => {
    ws.on('message', (data) => {
        const request = JSON.parse(data);
        const response = handleMcpRequest(request);
        ws.send(JSON.stringify(response));
    });
    
    ws.on('close', () => {
        cleanup();
    });
});
```

### Python (websockets library)

```python
import asyncio
import json
import websockets

async def handle_connection(websocket):
    # Check subprotocol
    if websocket.subprotocol != 'mcp':
        await websocket.close(1002, "Invalid subprotocol")
        return
    
    async for message in websocket:
        request = json.loads(message)
        response = handle_mcp_request(request)
        await websocket.send(json.dumps(response))

async def main():
    async with websockets.serve(
        handle_connection,
        "localhost",
        8080,
        subprotocols=["mcp"]
    ):
        await asyncio.Future()  # run forever

asyncio.run(main())
```

## Advanced Features

### Custom Headers

Add custom headers for authentication or other purposes:

```yaml
extensions:
  custom_headers:
    type: websocket
    uri: "wss://api.example.com/mcp"
    headers:
      Authorization: "Bearer token123"
      X-Custom-Header: "value"
      User-Agent: "Goose/1.0"
```

### Timeout Configuration

Adjust timeouts based on your use case:

```yaml
extensions:
  long_running:
    type: websocket
    uri: "ws://localhost:8080/mcp"
    timeout: 600  # 10 minutes for long-running operations
```

### Secure Connections (WSS)

For production deployments, use WSS (WebSocket Secure):

```yaml
extensions:
  production:
    type: websocket
    uri: "wss://mcp.example.com/api"
    headers:
      Authorization: "Bearer production-token"
```

## Testing Your WebSocket Extension

### Using websocat

```bash
# Install websocat
cargo install websocat

# Test connection
websocat ws://localhost:8080/mcp

# Send an initialization request
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | websocat ws://localhost:8080/mcp
```

### Using wscat

```bash
# Install wscat
npm install -g wscat

# Connect with subprotocol
wscat -c ws://localhost:8080/mcp -s mcp

# Send messages interactively
> {"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
```

### Using curl (for HTTP upgrade)

```bash
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  -H "Sec-WebSocket-Protocol: mcp" \
  http://localhost:8080/mcp
```

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug goose session
```

For more verbose output:

```bash
RUST_LOG=debug,goose=trace goose session 2> goose.log
```

### Common Issues

See the [WebSocket Troubleshooting](/docs/troubleshooting#websocket-extension-connection-issues) section for solutions to common problems:

- Connection refused or timeout
- Subprotocol negotiation errors
- Premature connection closure
- Authentication issues
- SSL/TLS certificate errors
- Performance issues

## Best Practices

### Server Implementation

1. **Always validate the subprotocol**: Check for `mcp` and reject others
2. **Echo the subprotocol**: Include `Sec-WebSocket-Protocol: mcp` in response
3. **Handle errors gracefully**: Send proper JSON-RPC error responses
4. **Implement timeouts**: Don't let connections hang indefinitely
5. **Log connection events**: Track connections, messages, and errors

### Client Configuration

1. **Use WSS in production**: Never use unencrypted `ws://` in production
2. **Set appropriate timeouts**: Balance responsiveness with operation duration
3. **Secure credentials**: Use environment variables, never hardcode tokens
4. **Test thoroughly**: Verify connection, authentication, and tool calls
5. **Monitor performance**: Track latency and connection stability

### Security

1. **Authenticate connections**: Use headers or query parameters for auth
2. **Validate all inputs**: Never trust client data
3. **Rate limit requests**: Prevent abuse
4. **Use TLS certificates**: Ensure proper certificate validation
5. **Implement access controls**: Restrict tool access as needed

## Performance Considerations

### Connection Pooling

WebSocket connections are persistent, so connection pooling is less critical than with HTTP. However:

- Reuse connections when possible
- Implement reconnection logic for dropped connections
- Monitor connection health with ping/pong frames

### Message Batching

For high-frequency operations:

- Consider batching multiple tool calls
- Use streaming responses for large data
- Implement backpressure handling

### Resource Management

- Close connections when done
- Clean up resources on disconnect
- Monitor memory usage for long-lived connections

## Migration Guide

### From SSE to WebSocket

**SSE Configuration:**
```yaml
extensions:
  example:
    type: sse
    uri: "http://localhost:8080/sse"
```

**WebSocket Configuration:**
```yaml
extensions:
  example:
    type: websocket
    uri: "ws://localhost:8080/mcp"
```

**Key Differences:**
- WebSocket is bidirectional (server can initiate requests)
- WebSocket requires `mcp` subprotocol
- WebSocket uses persistent connections (better performance)

### From Streaming HTTP to WebSocket

**Streaming HTTP Configuration:**
```yaml
extensions:
  example:
    type: streamable_http
    uri: "http://localhost:8080/stream"
```

**WebSocket Configuration:**
```yaml
extensions:
  example:
    type: websocket
    uri: "ws://localhost:8080/mcp"
```

**Key Differences:**
- WebSocket has lower latency
- WebSocket supports server-initiated requests
- WebSocket requires different server implementation

## Examples

### Local Development Server

```yaml
extensions:
  local_dev:
    type: websocket
    uri: "ws://localhost:8080/mcp"
    timeout: 300
    enabled: true
    name: "Local Dev Server"
    description: "Development MCP server"
```

### Production API

```yaml
extensions:
  production_api:
    type: websocket
    uri: "wss://mcp.example.com/api/v1"
    timeout: 600
    enabled: true
    name: "Production API"
    description: "Production MCP server"
    headers:
      Authorization: "Bearer ${API_TOKEN}"
    env_keys: ["API_TOKEN"]
```

### Internal Service

```yaml
extensions:
  internal_service:
    type: websocket
    uri: "ws://internal-mcp.company.local:8080/mcp"
    timeout: 300
    enabled: true
    name: "Internal MCP Service"
    description: "Company internal MCP server"
    headers:
      X-Service-Token: "${SERVICE_TOKEN}"
    env_keys: ["SERVICE_TOKEN"]
```

## See Also

- [Using Extensions](/docs/getting-started/using-extensions) - General extension documentation
- [Configuration File](/docs/guides/config-file) - Configuration reference
- [Troubleshooting](/docs/troubleshooting#websocket-extension-connection-issues) - WebSocket-specific troubleshooting
- [MCP Specification](https://modelcontextprotocol.io/) - Model Context Protocol documentation
