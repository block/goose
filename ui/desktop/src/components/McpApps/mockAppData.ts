type UIResourceUri = `ui://${string}`;
type UIResourceMimeType = `text/html;profile=mcp-app`;

interface UIResourceMeta {
  csp?: {
    connectDomains?: string[];
    resourceDomains?: string[];
  };
  domain?: `https://${string}`;
  prefersBorder?: boolean;
}

interface MockResourceListItem {
  uri: UIResourceUri;
  name: string;
  description: string;
  mimeType: UIResourceMimeType;
}

interface MockReadResourceItem {
  uri: UIResourceUri;
  description?: string;
  mimeType: UIResourceMimeType;
  text?: string;
  _meta?: {
    ui?: UIResourceMeta;
  };
}

interface MockListedResources {
  resources: MockResourceListItem[];
}

interface MockReadResources {
  contents: MockReadResourceItem[];
}

const UI_RESOURCE_URI = 'ui://weather-server/dashboard-template' as const;

export const mockToolListResult = {
  name: 'get_weather',
  description: 'Get current weather for a location',
  inputSchema: {
    type: 'object',
    properties: {
      location: { type: 'string' },
    },
  },
  _meta: {
    'ui/resourceUri': UI_RESOURCE_URI,
  },
};

export const mockResourceListResult: MockListedResources = {
  resources: [
    {
      uri: UI_RESOURCE_URI,
      name: 'weather_dashboard',
      description: 'Interactive weather dashboard widget',
      mimeType: 'text/html;profile=mcp-app',
    },
  ],
};

const mockAppHtml = `<!DOCTYPE html>
<html>
<head>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap" rel="stylesheet">
  <link href="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism-tomorrow.min.css" rel="stylesheet" id="prism-dark" />
  <link href="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism.min.css" rel="stylesheet" id="prism-light" disabled />
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/prism.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-json.min.js"></script>
  <style>
    :root {
      --bg-primary: #18181b;
      --bg-secondary: #000;
      --text-primary: #fafafa;
      --text-secondary: #a1a1aa;
      --border: #3f3f46;
    }
    
    .theme-light {
      --bg-primary: #fafafa;
      --bg-secondary: #fff;
      --text-primary: #18181b;
      --text-secondary: #52525b;
      --border: #e4e4e7;
    }
    
    .theme-dark {
      --bg-primary: #18181b;
      --bg-secondary: #000;
      --text-primary: #fafafa;
      --text-secondary: #a1a1aa;
      --border: #3f3f46;
    }

    html {
      overflow: hidden;
    }
    body {
      margin: 0;
      padding: 24px;
      color: var(--text-primary);
      background-color: var(--bg-primary);
      font-family: "Instrument Serif", system-ui, sans-serif;
      font-weight: 400;
      font-style: normal;
      transition: background-color 0.15s ease, color 0.15s ease;
    }
    h1 {
      font-size: min(max(4rem, 8vw), 8rem);
      text-align: center;
      line-height: 0.95;
      margin: 0;
      letter-spacing: -0.02em;
    }
    .cards {
      margin-top: 1.5rem;
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
      gap: 1rem;
      align-items: start;
    }
    .card {
      padding: 1rem;
      background: var(--bg-secondary);
      border: 1px solid var(--border);
      transition: background-color 0.15s ease, border-color 0.15s ease;
    }
    .card h2 {
      margin: 0 0 0.75rem 0;
      font-family: ui-monospace, monospace;
      font-size: 0.75rem;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-secondary);
    }
    .card pre[class*="language-"] {
      margin: 0 !important;
      padding: 0 !important;
      background: transparent !important;
      font-size: 1rem !important;
      line-height: 1.75 !important;
      overflow: visible !important;
    }
    .card code[class*="language-"] {
      font-family: ui-monospace, monospace !important;
      font-size: 1rem !important;
      white-space: pre-wrap !important;
      word-break: break-word !important;
    }
    footer {
      margin-top: 1.5rem;
      text-align: center;
    }
    footer a {
      color: var(--text-secondary);
      text-decoration: underline;
      font-size: 1.2rem;
      letter-spacing: revert;
      transition: color 0.15s ease;
      underline-offset: 1px;
      text-decoration-thickness: 1px;
    }
    footer a:hover {
      color: var(--text-primary);
      text-decoration: underline;
    }
  </style>
</head>
<body>
  <h1>MCP App Demo</h1>
  <div class="cards">
    <div class="card">
      <h2>Host Info</h2>
      <pre class="language-json"><code class="language-json" id="host-support-content">...</code></pre>
    </div>
    <div class="card">
      <h2>Host Context</h2>
      <pre class="language-json"><code class="language-json" id="context-content">Initializing...</code></pre>
    </div>
  </div>
  <footer>
    <a href="#" id="spec-link">MCP Apps Specification: SEP-1865</a>
  </footer>
  <script>
    (function() {
      let requestId = 1;
      const pendingRequests = new Map();
      let currentHostContext = null;

      function setTheme(theme) {
        document.body.classList.remove('theme-light', 'theme-dark');
        const prismDark = document.getElementById('prism-dark');
        const prismLight = document.getElementById('prism-light');
        
        if (theme === 'light') {
          document.body.classList.add('theme-light');
          prismDark.disabled = true;
          prismLight.disabled = false;
        } else {
          document.body.classList.add('theme-dark');
          prismDark.disabled = false;
          prismLight.disabled = true;
        }
      }

      function sendSizeChanged() {
        const width = document.body.scrollWidth;
        const height = document.body.scrollHeight;
        window.parent.postMessage({
          jsonrpc: '2.0',
          method: 'ui/notifications/size-changed',
          params: { width, height }
        }, '*');
      }

      function sendRequest(method, params) {
        return new Promise((resolve, reject) => {
          const id = requestId++;
          pendingRequests.set(id, { resolve, reject });
          window.parent.postMessage({
            jsonrpc: '2.0',
            id: id,
            method: method,
            params: params
          }, '*');
        });
      }

      function sendNotification(method, params) {
        window.parent.postMessage({
          jsonrpc: '2.0',
          method: method,
          params: params
        }, '*');
      }

      function renderJson(elementId, data) {
        const container = document.getElementById(elementId);
        if (!container) return;
        const json = JSON.stringify(data, null, 2);
        container.textContent = json;
        if (typeof Prism !== 'undefined') {
          Prism.highlightElement(container);
        }
      }

      function renderAllCards(result) {
        const hostSupport = {
          protocolVersion: result.protocolVersion,
          hostInfo: result.hostInfo,
          hostCapabilities: result.hostCapabilities,
        };
        renderJson('host-support-content', hostSupport);
        renderJson('context-content', result.hostContext);
        sendSizeChanged();
      }

      function renderHostContext(hostContext) {
        renderJson('context-content', hostContext);
        sendSizeChanged();
      }

      function handleMessage(event) {
        const data = event.data;
        if (!data || typeof data !== 'object' || data.jsonrpc !== '2.0') return;

        // Handle response to our request
        if ('id' in data && pendingRequests.has(data.id)) {
          const { resolve, reject } = pendingRequests.get(data.id);
          pendingRequests.delete(data.id);
          if (data.error) {
            reject(data.error);
          } else {
            resolve(data.result);
          }
          return;
        }

        // Handle host-context-changed notification
        if (data.method === 'ui/notifications/host-context-changed') {
          if (data.params && data.params.theme) {
            setTheme(data.params.theme);
          }
          if (currentHostContext) {
            Object.assign(currentHostContext, data.params);
            renderHostContext(currentHostContext);
          }
        }
      }

      async function initialize() {
        try {
          const result = await sendRequest('ui/initialize', {
            protocolVersion: '2025-06-18',
            capabilities: {},
            clientInfo: { name: 'MockMcpApp', version: '1.0.0' }
          });
          
          currentHostContext = result.hostContext || {};
          
          // Apply initial theme
          if (currentHostContext.theme) {
            setTheme(currentHostContext.theme);
          }
          
          renderAllCards(result);
          
          // Send initialized notification
          sendNotification('ui/notifications/initialized');
        } catch (error) {
          document.getElementById('context-content').textContent = 'Error: ' + error.message;
        }
      }

      window.addEventListener('message', handleMessage);

      // Handle spec link click
      document.getElementById('spec-link').addEventListener('click', function(e) {
        e.preventDefault();
        sendRequest('ui/open-link', {
          url: 'https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx'
        });
      });

      // Send initial size
      sendSizeChanged();

      // Observe size changes
      const resizeObserver = new ResizeObserver(sendSizeChanged);
      resizeObserver.observe(document.body);

      // Start initialization
      initialize();
    })();
  </script>
</body>
</html>`;

export const mockResourceReadResult: MockReadResources = {
  contents: [
    {
      uri: UI_RESOURCE_URI,
      mimeType: 'text/html;profile=mcp-app',
      text: mockAppHtml,
      _meta: {
        ui: {
          csp: {
            connectDomains: ['https://api.openweathermap.org'],
            resourceDomains: [
              'https://fonts.googleapis.com',
              'https://fonts.gstatic.com',
              'https://cdnjs.cloudflare.com',
            ],
          },
          prefersBorder: true,
        },
      },
    },
  ],
};
