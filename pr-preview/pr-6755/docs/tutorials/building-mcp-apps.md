# Building MCP Apps for goose

MCP Apps let MCP servers return interactive UIs that render directly inside the goose chat interface, rather than responding with text alone. This allows users to express intent through interaction, which is useful for workflows that require input, iteration, or visual feedback.

:::warning Experimental
MCP Apps support in goose is experimental and based on a draft specification. The implementation is minimal and may change, and does not yet support advanced capabilities or persistent app windows.
:::

In this tutorial, you will build an MCP App using JavaScript and Node.js. The app includes an interactive counter, stays in sync with the host theme, and sends messages back to the chat, showing how user intent flows from UI to agent.

:::info Prerequisites
- Node.js 18+ installed
- goose Desktop 1.19.1+ installed
:::

---

## Step 1: Initialize Your Project

Create a new directory and initialize a Node.js project:

```bash
mkdir mcp-app-demo
cd mcp-app-demo
npm init -y
```

Install the MCP SDK:

```bash
npm install @modelcontextprotocol/sdk
```

Update your `package.json` to use ES modules by adding `"type": "module"`:

```json5
{
  "name": "mcp-app-demo",
  "version": "1.0.0",
  // highlight-next-line
  "type": "module",
  "main": "server.js",
  "scripts": {
    "start": "node server.js"
  },
  "dependencies": {
    "@modelcontextprotocol/sdk": "^1.0.0"
  }
}
```

---

## Step 2: Create the MCP Server

Create `server.js` - this is the MCP server that loads and serves your HTML:

<details>
<summary>server.js</summary>

```javascript
#!/usr/bin/env node

  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

// Load HTML from file
const __dirname = dirname(fileURLToPath(import.meta.url));
const APP_HTML = readFileSync(join(__dirname, "index.html"), "utf-8");

// Create the MCP server
const server = new Server(
  {
    name: "mcp-app-demo",
    version: "1.0.0",
  },
  {
    capabilities: {
      tools: {},
      resources: {},
    },
  }
);

// List available tools
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: [
      {
        name: "show_demo_app",
        description: "Shows an interactive demo MCP App UI in the chat",
        inputSchema: {
          type: "object",
          properties: {},
          required: [],
        },
      },
    ],
  };
});

// Handle tool calls
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name } = request.params;

  if (name === "show_demo_app") {
    return {
      content: [
        {
          type: "text",
          text: "The demo app is now displayed!",
        },
      ],
      // This metadata tells goose to render the MCP App
      _meta: {
        ui: {
          resourceUri: "ui://mcp-app-demo/main",
        },
      },
    };
  }

  throw new Error(`Unknown tool: ${name}`);
});

// List available resources
server.setRequestHandler(ListResourcesRequestSchema, async () => {
  return {
    resources: [
      {
        uri: "ui://mcp-app-demo/main",
        name: "MCP App Demo",
        description: "An interactive demo",
        mimeType: "text/html;profile=mcp-app",
      },
    ],
  };
});

// Read resource content - returns the HTML
server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const { uri } = request.params;

  if (uri === "ui://mcp-app-demo/main") {
    return {
      contents: [
        {
          uri: "ui://mcp-app-demo/main",
          mimeType: "text/html;profile=mcp-app",
          text: APP_HTML,
          _meta: {
            ui: {
              csp: {
                connectDomains: [],
                resourceDomains: [],
                frameDomains: [],
                baseUriDomains: [],
              },
              prefersBorder: true,
            },
          },
        },
      ],
    };
  }

  throw new Error(`Resource not found: ${uri}`);
});

// Start the server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("MCP App Demo server running on stdio");
}

main().catch(console.error);
```

</details>

---

## Step 3: Create the App HTML

Create `index.html` - this is your interactive UI:

<details>
<summary>index.html</summary>

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>MCP App Demo</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      padding: 20px;
      min-height: 100vh;
      transition: background-color 0.3s, color 0.3s;
    }
    
    body.light { background: #f5f5f7; color: #1d1d1f; }
    body.dark { background: #1d1d1f; color: #f5f5f7; }
    
    .container {
      max-width: 500px;
      margin: 0 auto;
      padding: 24px;
      border-radius: 16px;
    }
    
    body.light .container { background: #ffffff; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }
    body.dark .container { background: #2d2d2f; box-shadow: 0 1px 3px rgba(0,0,0,0.3); }
    
    h1 { font-size: 24px; margin-bottom: 8px; }
    .subtitle { opacity: 0.7; margin-bottom: 20px; font-size: 14px; }
    
    .counter-section {
      text-align: center;
      padding: 24px;
      border-radius: 12px;
      margin-bottom: 20px;
    }
    
    body.light .counter-section { background: #f5f5f7; }
    body.dark .counter-section { background: #1d1d1f; }
    
    .counter-value { font-size: 64px; font-weight: bold; color: #0071e3; }
    .counter-label { font-size: 14px; opacity: 0.6; margin-top: 4px; }
    
    .button-row { display: flex; gap: 12px; justify-content: center; margin-top: 16px; }
    
    button {
      padding: 12px 24px;
      font-size: 18px;
      border: none;
      border-radius: 8px;
      cursor: pointer;
      font-weight: 600;
      color: white;
      transition: opacity 0.2s;
    }
    
    button:hover { opacity: 0.85; }
    button:active { opacity: 0.7; }
    
    .btn-increment { background: #0071e3; }
    .btn-decrement { background: #ff3b30; }
    .btn-reset { background: #86868b; }
    .btn-send { background: #34c759; }
    
    .message-section { margin-top: 20px; }
    .message-section h3 { font-size: 16px; margin-bottom: 12px; }
    .message-input { display: flex; gap: 8px; }
    
    input[type="text"] {
      flex: 1;
      padding: 12px 16px;
      border-radius: 8px;
      border: 2px solid transparent;
      font-size: 14px;
      transition: border-color 0.2s;
    }
    
    body.light input { background: #f5f5f7; color: #1d1d1f; }
    body.dark input { background: #1d1d1f; color: #f5f5f7; }
    input:focus { outline: none; border-color: #0071e3; }
    
    .status {
      margin-top: 16px;
      padding: 12px;
      border-radius: 8px;
      font-size: 13px;
      display: none;
    }
    
    .status.show { display: block; }
    .status.success { background: rgba(52, 199, 89, 0.15); color: #34c759; }
    .status.error { background: rgba(255, 59, 48, 0.15); color: #ff3b30; }
    
    .info-section {
      margin-top: 20px;
      padding: 16px;
      border-radius: 8px;
      font-size: 12px;
      opacity: 0.8;
    }
    
    body.light .info-section { background: #f5f5f7; }
    body.dark .info-section { background: #1d1d1f; }
    
    .info-section code {
      background: rgba(0, 113, 227, 0.1);
      padding: 2px 6px;
      border-radius: 4px;
      font-family: 'SF Mono', Monaco, monospace;
    }
  </style>
</head>
<body class="light">
  <div class="container">
    <h1>🎮 MCP App Demo</h1>
    <p class="subtitle">An interactive UI running inside goose</p>
    
    <div class="counter-section">
      <div class="counter-value" id="counter">0</div>
      <div class="counter-label">Counter Value</div>
      <div class="button-row">
        <button class="btn-decrement" onclick="updateCounter(-1)">−</button>
        <button class="btn-reset" onclick="resetCounter()">Reset</button>
        <button class="btn-increment" onclick="updateCounter(1)">+</button>
      </div>
    </div>
    
    <div class="message-section">
      <h3>💬 Send a message to goose</h3>
      <div class="message-input">
        <input type="text" id="messageInput" placeholder="Type a message..." />
        <button class="btn-send" onclick="sendMessage()">Send</button>
      </div>
      <div class="status" id="status"></div>
    </div>
    
    <div class="info-section">
      <strong>How this works:</strong><br><br>
      This UI is served as an MCP resource with the <code>ui://</code> scheme. 
      It communicates with goose via JSON-RPC messages through the sandbox bridge.
      <br><br>
      • Counter uses local state<br>
      • "Send" calls <code>ui/message</code> to append text to chat<br>
      • Theme syncs with goose's theme setting
    </div>
  </div>

  <script>
    class McpAppClient {
      constructor() {
        this.pendingRequests = new Map();
        this.requestId = 0;
        this.initialized = false;
        this.hostContext = null;
        
        window.addEventListener('message', (e) => this.handleMessage(e));
        this.initialize();
      }
      
      async initialize() {
        try {
          const result = await this.request('ui/initialize', {});
          this.hostContext = result.hostContext;
          this.initialized = true;
          
          if (this.hostContext?.theme) {
            this.applyTheme(this.hostContext.theme);
          }
          
          this.notify('ui/notifications/initialized', {});
          this.reportSize();
        } catch (error) {
          console.error('Failed to initialize MCP App:', error);
        }
      }
      
      handleMessage(event) {
        const data = event.data;
        if (!data || typeof data !== 'object') return;
        
        if ('id' in data && this.pendingRequests.has(data.id)) {
          const { resolve, reject } = this.pendingRequests.get(data.id);
          this.pendingRequests.delete(data.id);
          data.error ? reject(new Error(data.error.message)) : resolve(data.result);
          return;
        }
        
        if (data.method === 'ui/notifications/host-context-changed') {
          if (data.params?.theme) {
            this.applyTheme(data.params.theme);
          }
        }
      }
      
      request(method, params) {
        return new Promise((resolve, reject) => {
          const id = ++this.requestId;
          this.pendingRequests.set(id, { resolve, reject });
          window.parent.postMessage({ jsonrpc: '2.0', id, method, params }, '*');
          
          setTimeout(() => {
            if (this.pendingRequests.has(id)) {
              this.pendingRequests.delete(id);
              reject(new Error('Request timed out'));
            }
          }, 30000);
        });
      }
      
      notify(method, params) {
        window.parent.postMessage({ jsonrpc: '2.0', method, params }, '*');
      }
      
      applyTheme(theme) {
        document.body.className = theme;
      }
      
      reportSize() {
        this.notify('ui/notifications/size-changed', { height: document.body.scrollHeight });
      }
      
      async sendMessageToChat(text) {
        return this.request('ui/message', { content: { type: 'text', text } });
      }
    }
    
    const mcpApp = new McpAppClient();
    
    let counter = 0;
    
    function updateCounter(delta) {
      counter += delta;
      document.getElementById('counter').textContent = counter;
      mcpApp.reportSize();
    }
    
    function resetCounter() {
      counter = 0;
      document.getElementById('counter').textContent = counter;
      mcpApp.reportSize();
    }
    
    async function sendMessage() {
      const input = document.getElementById('messageInput');
      const message = input.value.trim();
      
      if (!message) {
        showStatus('Please enter a message', 'error');
        return;
      }
      
      try {
        await mcpApp.sendMessageToChat(message);
        showStatus('Message sent to chat!', 'success');
        input.value = '';
      } catch (error) {
        showStatus('Failed to send: ' + error.message, 'error');
      }
    }
    
    function showStatus(message, type) {
      const status = document.getElementById('status');
      status.textContent = message;
      status.className = 'status show ' + type;
      setTimeout(() => { status.className = 'status'; }, 3000);
    }
    
    document.getElementById('messageInput').addEventListener('keypress', (e) => {
      if (e.key === 'Enter') sendMessage();
    });
  </script>
</body>
</html>
```

</details>

---

## Step 4: Add to goose Desktop

1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
2. Click `Extensions`
3. Click `Add custom extension`
4. Fill in the details:
   - **Type**: `Standard IO`
   - **ID**: `mcp-app-demo`
   - **Name**: `MCP App Demo`
   - **Command**: `node /full/path/to/mcp-app-demo/server.js`
5. Click `Add`

For more options, see [Adding Extensions](/docs/getting-started/using-extensions#adding-extensions).

---

## Step 5: Test Your App

1. Restart goose to load the new extension
2. Prompt goose: "Show me the demo app"
3. goose will call the `show_demo_app` tool
4. Your interactive app will render in the chat!

Try:
- Clicking the counter buttons
- Typing a message and clicking "Send"
- Switching goose between light/dark mode

---

## How It Works

```
┌──────────────────────────────────────┐
│           Your MCP App               │  HTML/JS in sandboxed iframe
└──────────────────┬───────────────────┘
                   │ postMessage
┌──────────────────▼───────────────────┐
│          goose Desktop               │  Renders UI, routes messages
└──────────────────┬───────────────────┘
                   │ MCP Protocol
┌──────────────────▼───────────────────┐
│          Your MCP Server             │  Serves HTML via resources
└──────────────────────────────────────┘
```

Your server returns a `ui://` resource URI, goose fetches the HTML and renders it in an iframe. The app communicates back via `postMessage`—requesting theme info, sending messages to chat, or resizing itself.

MCP Apps run in a sandboxed iframe with strict Content Security Policy restrictions.

### Content Security Policy Configuration

By default, apps can only load resources from their own origin. If your app needs to interact with external domains—such as loading resources from a CDN, making API calls, or embedding maps—you can configure which domains are allowed through the `csp` object in the resource's `_meta.ui` section.

```javascript
_meta: {
  ui: {
    csp: {
      connectDomains: [],      // Domains for fetch/XHR requests
      resourceDomains: [],     // Domains for scripts, styles, images, fonts, media
      frameDomains: [],        // Origins allowed for nested iframes
      baseUriDomains: [],      // Additional allowed base URIs
    },
  },
}
```

| Option | CSP Directive | Purpose | Default |
|--------|---------------|---------|---------|
| `connectDomains` | `connect-src` | Domains your app can make network requests to | Same-origin only |
| `resourceDomains` | `script-src`, `style-src`, `img-src`, `font-src`, `media-src` | Domains for loading external resources | Same-origin only |
| `frameDomains` | `frame-src` | Origins allowed for nested `<iframe>` elements | `'none'` (no iframes) |
| `baseUriDomains` | `base-uri` | Additional domains allowed for `<base>` element | `'self'` only |

<details>
<summary>Examples</summary>

**Embedding a map:**

```javascript
csp: {
  frameDomains: ['https://www.openstreetmap.org'],
  resourceDomains: ['https://tile.openstreetmap.org'],
}
```

**Loading resources from a CDN:**

```javascript
csp: {
  resourceDomains: ['https://cdn.jsdelivr.net', 'https://unpkg.com'],
  connectDomains: ['https://api.example.com'],
}
```

</details>

See the [MCP Apps Specification](https://github.com/modelcontextprotocol/ext-apps) for details on security and the full protocol.

:::warning Security Consideration
Only add domains you trust. Each domain you add expands what external content can be loaded or embedded in your app. Keep the list minimal and specific to reduce security risks.
:::