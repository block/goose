# MCP Apps Protocol Glossary

> Reference for the JSON-RPC message protocol between Goose (Host) and MCP App Guest UIs.
> 
> Based on [SEP-1865 Draft Specification](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Goose Desktop (Host)                     │
│                                                                 │
│  ┌──────────────────┐     postMessage      ┌─────────────────┐  │
│  │ useSandboxBridge │ ◄──────────────────► │  Sandbox Iframe │  │
│  │   (React Hook)   │                      │ (mcp_app_proxy) │  │
│  └──────────────────┘                      └────────┬────────┘  │
│                                                     │           │
│                                            postMessage           │
│                                                     │           │
│                                            ┌────────▼────────┐  │
│                                            │  Guest UI       │  │
│                                            │  (MCP App HTML) │  │
│                                            └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**Three layers:**
1. **Host** - Goose Desktop (`useSandboxBridge.ts`)
2. **Sandbox** - Security boundary iframe (`mcp_app_proxy.html`)
3. **Guest UI** - The actual MCP App content (HTML from server)

---

## Message Flow Lifecycle

```
Host                          Sandbox                       Guest UI
  │                              │                              │
  │  ──── load iframe ────────►  │                              │
  │                              │                              │
  │  ◄── sandbox-ready ────────  │                              │
  │                              │                              │
  │  ── sandbox-resource-ready ► │                              │
  │      (html, csp)             │  ──── create iframe ────►    │
  │                              │                              │
  │  ◄─────────────────────────────── ui/initialize ──────────  │
  │                              │                              │
  │  ─────────────────────────────── initialize response ────►  │
  │                              │                              │
  │  ◄─────────────────────────────── initialized ────────────  │
  │                              │                              │
  │         (Guest UI is now ready for interaction)             │
  │                              │                              │
```

---

## Messages: Guest UI → Host

### Lifecycle Messages

| Method | Type | Description | Status |
|--------|------|-------------|--------|
| `ui/initialize` | Request | Guest UI requests initialization handshake | ✅ Implemented |
| `ui/notifications/initialized` | Notification | Guest UI confirms it's ready | ✅ Implemented |
| `ui/notifications/size-changed` | Notification | Guest UI reports its dimensions changed | ✅ Implemented |

### Action Requests

| Method | Type | Description | Status |
|--------|------|-------------|--------|
| `ui/open-link` | Request | Open an external URL | ✅ Implemented |
| `ui/message` | Request | Send message content to host's chat | ✅ Implemented |

### MCP Passthrough (forwarded to MCP server)

| Method | Type | Description | Status |
|--------|------|-------------|--------|
| `tools/call` | Request | Execute a tool on the MCP server | 🚧 TODO |
| `resources/read` | Request | Read a resource from the MCP server | 🚧 TODO |
| `notifications/message` | Notification | Log messages to MCP server | 🚧 TODO |
| `ping` | Request | Connection health check | 🚧 TODO |

---

## Messages: Host → Guest UI

### Lifecycle Messages

| Method | Type | Description | Status |
|--------|------|-------------|--------|
| Initialize Response | Response | Response to `ui/initialize` request | ✅ Implemented |
| `ui/notifications/host-context-changed` | Notification | Host context has changed (theme, viewport) | ✅ Implemented |
| `ui/resource-teardown` | Request | Host notifies UI before teardown | 🚧 TODO |

### Tool Interaction

| Method | Type | Description | Status |
|--------|------|-------------|--------|
| `ui/notifications/tool-input` | Notification | Deliver tool input to the UI | 🚧 TODO |
| `ui/notifications/tool-input-partial` | Notification | Streaming/partial tool input | 🚧 TODO |
| `ui/notifications/tool-result` | Notification | Tool execution result | 🚧 TODO |
| `ui/notifications/tool-cancelled` | Notification | Tool call was cancelled | 🚧 TODO |

---

## Internal Messages (Sandbox Proxy)

These messages are internal to the Host ↔ Sandbox communication and are NOT forwarded to the Guest UI.

| Method | Type | Direction | Description | Status |
|--------|------|-----------|-------------|--------|
| `ui/notifications/sandbox-ready` | Notification | Sandbox → Host | Sandbox iframe is loaded | ✅ Implemented |
| `ui/notifications/sandbox-resource-ready` | Notification | Host → Sandbox | HTML content delivered to sandbox | ✅ Implemented |

---

## Message Schemas

### Guest UI → Host

#### `ui/initialize` (Request)
```typescript
{
  jsonrpc: "2.0",
  id: string | number,
  method: "ui/initialize",
  params: {
    protocolVersion: string,
    capabilities: {
      // Guest capabilities
    },
    clientInfo: {
      name: string,
      version: string
    }
  }
}
```

#### `ui/notifications/initialized` (Notification)
```typescript
{
  jsonrpc: "2.0",
  method: "ui/notifications/initialized"
}
```

#### `ui/notifications/size-changed` (Notification)
```typescript
{
  jsonrpc: "2.0",
  method: "ui/notifications/size-changed",
  params: {
    width: number,
    height: number
  }
}
```

#### `ui/open-link` (Request)
```typescript
{
  jsonrpc: "2.0",
  id: string | number,
  method: "ui/open-link",
  params: {
    url: string
  }
}

// Success Response
{
  jsonrpc: "2.0",
  id: string | number,
  result: {}
}
```

#### `ui/message` (Request)
```typescript
{
  jsonrpc: "2.0",
  id: string | number,
  method: "ui/message",
  params: {
    role: "user",
    content: {
      type: "text",
      text: string
    }
  }
}

// Success Response
{
  jsonrpc: "2.0",
  id: string | number,
  result: {}
}
```

---

### Host → Guest UI

#### Initialize Response
```typescript
{
  jsonrpc: "2.0",
  id: string | number,
  result: {
    protocolVersion: string,
    hostCapabilities: { /* ... */ },
    hostInfo: {
      name: string,      // "Goose Desktop"
      version: string
    },
    hostContext: {
      theme?: "light" | "dark",
      displayMode?: "inline" | "fullscreen" | "pip",
      viewport?: { width: number, height: number },
      locale?: string,
      timeZone?: string,
      platform?: "web" | "desktop" | "mobile"
    }
  }
}
```

#### `ui/notifications/tool-input` (Notification)
```typescript
{
  jsonrpc: "2.0",
  method: "ui/notifications/tool-input",
  params: {
    toolName: string,
    arguments: Record<string, unknown>
  }
}
```

#### `ui/notifications/tool-input-partial` (Notification)
```typescript
{
  jsonrpc: "2.0",
  method: "ui/notifications/tool-input-partial",
  params: {
    toolName: string,
    arguments: Record<string, unknown>  // Partial/streaming arguments
  }
}
```

#### `ui/notifications/tool-result` (Notification)
```typescript
{
  jsonrpc: "2.0",
  method: "ui/notifications/tool-result",
  params: {
    toolName: string,
    result: unknown
  }
}
```

#### `ui/notifications/tool-cancelled` (Notification)
```typescript
{
  jsonrpc: "2.0",
  method: "ui/notifications/tool-cancelled",
  params: {
    reason?: string
  }
}
```

#### `ui/resource-teardown` (Request)
```typescript
{
  jsonrpc: "2.0",
  id: string | number,
  method: "ui/resource-teardown",
  params: {
    reason: string
  }
}

// Success Response
{
  jsonrpc: "2.0",
  id: string | number,
  result: {}
}
```

#### `ui/notifications/host-context-changed` (Notification)
```typescript
{
  jsonrpc: "2.0",
  method: "ui/notifications/host-context-changed",
  params: {
    // Same structure as hostContext in initialize response
    theme?: "light" | "dark",
    displayMode?: "inline" | "fullscreen" | "pip",
    viewport?: { width: number, height: number },
    // ... etc
  }
}
```

---

## Resource Format

MCP Apps are delivered as resources with special metadata:

```typescript
interface McpAppResource {
  uri: `ui://${string}`;              // Must start with ui://
  description?: string;
  mimeType: "text/html;profile=mcp-app";  // Required MIME type
  text?: string;                      // The HTML content
  _meta?: {
    ui?: {
      csp?: {
        connectDomains?: string[];    // Allowed fetch/XHR domains
        resourceDomains?: string[];   // Allowed asset domains (fonts, images)
      };
      domain?: `https://${string}`;   // Optional origin domain
      prefersBorder?: boolean;        // UI hint for rendering
    };
  };
}
```

### Tool Metadata Linking

Tools can link to UI resources via metadata:

```typescript
{
  name: "get_weather",
  description: "Get current weather",
  inputSchema: { /* ... */ },
  _meta: {
    "ui/resourceUri": "ui://weather-server/dashboard"
  }
}
```

---

## Implementation Status in Goose

### ✅ Implemented
- Sandbox iframe loading (`mcp_app_proxy.html`)
- `ui/notifications/sandbox-ready` handling
- `ui/notifications/sandbox-resource-ready` sending
- `ui/initialize` / initialize response handshake (with full `hostContext`)
- `ui/notifications/initialized` handling
- `ui/notifications/size-changed` handling
- `ui/notifications/host-context-changed` - theme and viewport changes
- `ui/open-link` - opening external URLs
- `ui/message` - sending messages to chat
- CSP enforcement based on resource metadata

### 🚧 TODO
- `ui/resource-teardown` - cleanup before UI removal
- `ui/notifications/tool-input` - sending tool inputs
- `ui/notifications/tool-input-partial` - streaming tool inputs
- `ui/notifications/tool-result` - sending tool results
- `ui/notifications/tool-cancelled` - cancellation
- MCP passthrough (`tools/call`, `resources/read`, etc.)

---

## File Reference

| File | Purpose |
|------|---------|
| `useSandboxBridge.ts` | React hook managing Host ↔ Sandbox communication |
| `utils.ts` | Helper functions for creating JSON-RPC messages |
| `types.ts` | TypeScript type definitions |
| `McpAppRenderer.tsx` | React component rendering the iframe |
| `mcp_app_proxy.html` | Sandbox HTML (served by goosed) |
| `mcp_app_proxy.rs` | Rust route serving the sandbox HTML |

---

## Quick Reference: What to Implement

### To handle a new Guest → Host message:
1. Add case to `handleMessage()` in `useSandboxBridge.ts`
2. Implement the handler logic

### To send a new Host → Guest message:
1. Add helper function in `utils.ts`
2. Call `sendToSandbox()` from `useSandboxBridge.ts`

### To forward to MCP server:
1. Detect non-`ui/` prefixed methods in `handleMessage()`
2. Forward via appropriate MCP client API
3. Send response back via `sendToSandbox()`
