---
title: 构建 MCP Apps
description: 创建可直接在 goose Desktop 中渲染的交互式 UI 应用
---

import { PanelLeft } from 'lucide-react';

# 为 goose 构建 MCP Apps

MCP Apps 允许 MCP server 返回可以直接渲染在 goose 聊天界面中的交互式 UI，而不是只返回纯文本。这样用户就能通过点选、输入和可视化交互表达意图，特别适合需要用户参与、反复迭代或即时反馈的工作流。

:::warning Experimental
goose 对 MCP Apps 的支持仍是实验性的，并且基于一份草案规范。当前实现较为精简，后续能力与接口都可能变化，暂时也还不支持更高级的能力或持久化 app 窗口。
:::

在这个教程里，你会使用 JavaScript 和 Node.js 构建一个 MCP App。这个 app 包含一个交互式计数器、会自动同步宿主主题，并且能把消息发回聊天窗口，从而展示“用户意图如何从 UI 流向 agent”。

:::info 前置条件
- 已安装 Node.js 18+
- 已安装 goose Desktop 1.19.1+
:::

---

## 第 1 步：初始化项目

创建一个新目录并初始化 Node.js 项目：

```bash
mkdir mcp-app-demo
cd mcp-app-demo
npm init -y
```

安装 MCP SDK：

```bash
npm install @modelcontextprotocol/sdk
```

然后更新 `package.json`，添加 `"type": "module"`，让项目使用 ES modules：

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

## 第 2 步：创建 MCP Server

创建 `server.js`。这个文件负责加载并提供你的 HTML：

<details>
<summary>server.js</summary>

```javascript
#!/usr/bin/env node

import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
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

这个 server 做了几件关键的事：
- 注册一个 `show_demo_app` 工具，让 goose 可以调用它
- 暴露一个 `ui://mcp-app-demo/main` 资源，用于返回 HTML
- 在工具调用结果中通过 `_meta.ui.resourceUri` 告诉 goose 要渲染哪个 UI 资源

---

## 第 3 步：创建 App HTML

创建 `index.html`。这就是你的交互式 UI：

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

这个 UI 展示了 MCP App 最常见的几类能力：
- 本地状态管理：计数器保存在前端状态中
- 和宿主通信：通过 `postMessage` 调 `ui/message`
- 跟随 goose 主题：根据宿主上下文切换浅色 / 深色
- 尺寸上报：UI 高度变化后通知宿主重新布局

---

## 第 4 步：添加到 goose Desktop

1. 点击左上角的 <PanelLeft className="inline" size={16} /> 打开侧边栏
2. 点击 `Extensions`
3. 点击 `Add custom extension`
4. 填写以下信息：
   - **Type**：`Standard IO`
   - **ID**：`mcp-app-demo`
   - **Name**：`MCP App Demo`
   - **Command**：`node /full/path/to/mcp-app-demo/server.js`
5. 点击 `Add`

如果你需要更多配置项，可以参考[添加扩展](/zh-CN/docs/getting-started/using-extensions)。

---

## 第 5 步：测试你的 App

1. 重启 goose，让新扩展被加载
2. 对 goose 输入：“Show me the demo app”
3. goose 会调用 `show_demo_app` 工具
4. 你的交互式 app 就会直接渲染在聊天窗口里

你可以试着：
- 点击计数器按钮
- 输入一条消息后点击 “Send”
- 切换 goose 的浅色 / 深色模式

---

## 它是如何工作的

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

你的 server 返回一个 `ui://` 资源 URI，goose 拉取对应 HTML 后，把它渲染在 iframe 里。这个 app 再通过 `postMessage` 与宿主通信，例如请求主题信息、向聊天发送消息，或者上报自身尺寸。

MCP Apps 运行在一个带严格 Content Security Policy 限制的沙箱 iframe 中。

### Content Security Policy 配置

默认情况下，app 只能加载同源资源。如果你的 app 需要访问外部域名，例如从 CDN 拉静态资源、调用外部 API，或嵌入地图，则可以在资源的 `_meta.ui` 里通过 `csp` 对象声明允许的域名范围。

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

| 选项 | 对应 CSP 指令 | 用途 | 默认值 |
|--------|---------------|------|--------|
| `connectDomains` | `connect-src` | 允许发起网络请求的域名 | 仅同源 |
| `resourceDomains` | `script-src`、`style-src`、`img-src`、`font-src`、`media-src` | 允许加载外部脚本、样式、图片、字体和媒体的域名 | 仅同源 |
| `frameDomains` | `frame-src` | 允许嵌套 `<iframe>` 的来源 | `'none'`（不允许 iframe） |
| `baseUriDomains` | `base-uri` | 允许 `<base>` 使用的额外域名 | 仅 `'self'` |

<details>
<summary>示例</summary>

**嵌入地图：**

```javascript
csp: {
  frameDomains: ['https://www.openstreetmap.org'],
  resourceDomains: ['https://tile.openstreetmap.org'],
}
```

**从 CDN 加载资源：**

```javascript
csp: {
  resourceDomains: ['https://cdn.jsdelivr.net', 'https://unpkg.com'],
  connectDomains: ['https://api.example.com'],
}
```

</details>

:::warning 安全注意事项
只添加你信任的域名。每增加一个域名，都会扩大你的 app 可以加载或嵌入的外部内容范围。列表应尽量小，且只保留真正需要的域名，以降低安全风险。
:::

### 申请浏览器权限

MCP Apps 还可以通过 Permission Policy 申请特定浏览器权限。这对需要摄像头、麦克风或地理位置等设备能力的 app 很有用。需要注意的是，这里只是“申请”，宿主不一定会授予，app 应当通过特性检测优雅处理权限不可用的情况。

如需声明权限，在资源的 `_meta.ui` 中加入 `permissions` 对象：

```javascript
_meta: {
  ui: {
    permissions: {
      camera: true,           // Request camera access
      microphone: true,       // Request microphone access
      geolocation: true,      // Request geolocation access
      clipboardWrite: true,   // Request clipboard write access
    },
  },
}
```

| 权限 | 对应 Permission Policy Feature | 常见用途 |
|------------|---------------------------|----------|
| `camera` | `camera` | 视频采集、扫码 |
| `microphone` | `microphone` | 录音、语音输入 |
| `geolocation` | `geolocation` | 定位类应用、地图 |
| `clipboardWrite` | `clipboard-write` | 复制内容到剪贴板 |

所有权限默认都是 `false`。只申请你的 app 真正需要的权限。

<details>
<summary>示例：录像应用</summary>

```javascript
server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const { uri } = request.params;

  if (uri === "ui://my-video-app/recorder") {
    return {
      contents: [
        {
          uri: "ui://my-video-app/recorder",
          mimeType: "text/html;profile=mcp-app",
          text: VIDEO_RECORDER_HTML,
          _meta: {
            ui: {
              permissions: {
                camera: true,
                microphone: true,
              },
            },
          },
        },
      ],
    };
  }
});
```

</details>

:::info 需要用户授权
即使 MCP App 申请了权限，浏览器仍然会先向用户弹出授权提示。用户可以在任何时候拒绝授权。
:::

如果你想深入了解安全模型和完整协议，请继续查看 [MCP Apps Specification](https://github.com/modelcontextprotocol/ext-apps)。
