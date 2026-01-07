# MCP App Demo

A simple MCP (Model Context Protocol) App that demonstrates how to create interactive UIs that render inside Goose.

## What is an MCP App?

MCP Apps are interactive HTML/JS applications that can be served by MCP servers and rendered inline in Goose's chat interface. They communicate with Goose through a JSON-RPC bridge, allowing them to:

- Send messages to the chat
- Call tools on the MCP server
- Read resources
- Open external links
- Sync with Goose's theme (light/dark mode)

## Features

This demo includes:

1. **Interactive Counter** - A simple counter with increment, decrement, and reset buttons
2. **Message Sender** - Send text messages directly to the Goose chat
3. **Theme Sync** - Automatically matches Goose's light/dark theme
4. **Tool Integration** - Demonstrates how tools can trigger UI display

## Installation

```bash
cd examples/mcp-app-demo
npm install
```

## Usage with Goose

### Option 1: Add to Goose Config

Add this extension to your `~/.config/goose/config.yaml`:

```yaml
extensions:
  mcp-app-demo:
    enabled: true
    type: stdio
    name: mcp-app-demo
    description: Interactive MCP App demo
    cmd: node
    args:
      - /path/to/goose/examples/mcp-app-demo/server.js
    timeout: 300
```

Then restart Goose and ask: "Show me the demo app"

### Option 2: Run with npx (after publishing)

```bash
# If published to npm
goose --with-extension "npx mcp-app-demo"
```

## How It Works

### Server Side (`server.js`)

The MCP server provides:

1. **Resources** - Lists and serves the HTML UI via the `ui://` scheme
2. **Tools** - `show_demo_app` triggers the UI to display

```javascript
// Resource with ui:// scheme tells Goose this is a renderable app
{
  uri: "ui://mcp-app-demo/main",
  mimeType: "text/html;profile=mcp-app",
  text: APP_HTML,
  _meta: {
    ui: {
      csp: { connectDomains: [], resourceDomains: [] },
      prefersBorder: true
    }
  }
}
```

### Client Side (embedded in HTML)

The app uses a minimal MCP App client that:

1. **Initializes** with the host via `ui/initialize`
2. **Reports size** changes via `ui/notifications/size-changed`
3. **Sends messages** to chat via `ui/message`
4. **Listens** for theme changes via `ui/notifications/host-context-changed`

```javascript
// Send a message to the Goose chat
await mcpApp.request('ui/message', {
  content: { type: 'text', text: 'Hello from the MCP App!' }
});
```

## MCP App Protocol

MCP Apps communicate via JSON-RPC 2.0 messages:

### Requests (App → Host)

| Method | Description |
|--------|-------------|
| `ui/initialize` | Initialize and get host context |
| `ui/message` | Send message to chat |
| `ui/open-link` | Open URL in browser |
| `tools/call` | Call an MCP tool |
| `resources/read` | Read an MCP resource |

### Notifications (App → Host)

| Method | Description |
|--------|-------------|
| `ui/notifications/initialized` | App is ready |
| `ui/notifications/size-changed` | Report new dimensions |

### Notifications (Host → App)

| Method | Description |
|--------|-------------|
| `ui/notifications/host-context-changed` | Theme/viewport changed |
| `ui/notifications/tool-input` | Tool input received |
| `ui/notifications/tool-result` | Tool execution result |

## Development

To modify the app:

1. Edit the `APP_HTML` constant in `server.js`
2. Restart the MCP server (Goose will reconnect)
3. Ask Goose to show the demo app again

## Security

MCP Apps run in a sandboxed iframe with:

- Content Security Policy (CSP) restrictions
- `sandbox="allow-scripts allow-same-origin"` attribute
- No direct access to parent window or Goose internals

The `_meta.ui.csp` field in resources declares what domains the app needs access to.

## Learn More

- [MCP Apps Specification](https://github.com/modelcontextprotocol/ext-apps)
- [Model Context Protocol](https://modelcontextprotocol.io)
- [Goose Documentation](https://block.github.io/goose)
