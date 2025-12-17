import { McpAppResource } from './types';

interface MockResourceListItem {
  uri: McpAppResource['uri'];
  name: McpAppResource['name'];
  description: McpAppResource['description'];
  mimeType: McpAppResource['mimeType'];
}

interface MockListedResources {
  resources: MockResourceListItem[];
}

interface MockReadResources {
  contents: McpAppResource[];
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
      --bg-terminal: #0a0a0a;
      --text-primary: #fafafa;
      --text-secondary: #a1a1aa;
      --border: #3f3f46;
    }
    
    .theme-light {
      --bg-primary: #fafafa;
      --bg-terminal: #f4f4f5;
      --text-primary: #18181b;
      --text-secondary: #52525b;
      --border: #e4e4e7;
    }
    
    .theme-dark {
      --bg-primary: #18181b;
      --bg-terminal: #0a0a0a;
      --text-primary: #fafafa;
      --text-secondary: #a1a1aa;
      --border: #3f3f46;
    }

    html {
      overflow: hidden;
    }
    body {
      margin: 0;
      padding: 24px 24px 0 24px;
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
      margin: 2rem auto 3rem;
      letter-spacing: -0.02em;
    }
    .host-info-subtitle {
      text-align: center;
      margin: 0 0 2rem 0;
      font-family: ui-monospace, monospace;
      font-size: 0.875rem;
      color: var(--text-secondary);
    }
    .actions {
      margin-top: 1rem;
      margin-bottom: 2rem;
      text-align: center;
    }
    .actions-heading {
      margin: 0 0 0.75rem 0;
      letter-spacing: 0.05em;
      color: var(--text-primary);
    }
    .actions-note {
      color: var(--text-secondary);
      font-size: 1.2rem;
      // margin: -0.5rem 0 0.75rem 0;
    }
    .actions-buttons {
      display: flex;
      justify-content: center;
      flex-wrap: wrap;
      gap: 0.5rem;
    }
    .action-btn {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.625rem 1.25rem;
      background: var(--text-primary);
      border: 1px solid var(--border);
      border-radius: 6px;
      color: var(--bg-primary);
      font-family: ui-monospace, monospace;
      font-size: 1rem;
      cursor: pointer;
      transition: all 0.15s ease;
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
    }
    .action-btn:hover {
      background: var(--bg-primary);
      color: var(--text-primary);
      transform: translateY(-1px);
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
    }
    .action-btn:active {
      transform: translateY(0);
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
    }
    .action-btn code {
      color: var(--bg-primary);
      opacity: 0.7;
      font-size: 0.75rem;
    }
    .action-btn:hover code {
      color: var(--text-primary);
      opacity: 0.7;
    }
    .action-btn:disabled {
      opacity: 0.25;
      cursor: not-allowed;
      transform: none;
    }
    .action-btn:disabled:hover {
      background: var(--bg-primary);
      transform: none;
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
    }
    .theme-light .action-btn {
      box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
    }
    .theme-light .action-btn:hover {
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.12);
    }
    .terminal {
      margin: 1.5rem -24px 0 -24px;
      background: var(--bg-terminal);
      border-top: 1px solid var(--border);
      box-shadow: inset 0 1px 4px rgba(0, 0, 0, 0.5);
      transition: background-color 0.15s ease, border-color 0.15s ease;
    }
    .theme-light .terminal {
      box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.08);
    }
    .terminal-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
      align-items: stretch;
    }
    .terminal-grid > .terminal-section {
      padding: 1.5rem 24px;
    }
    .terminal-grid > .terminal-section:not(:last-child) {
      border-right: 1px solid var(--border);
    }
    @media (max-width: 700px) {
      .terminal-grid > .terminal-section:not(:last-child) {
        border-right: none;
        border-bottom: 1px solid var(--border);
      }
    }
    .terminal-section h2 {
      margin: 0 0 0.75rem 0;
      font-family: ui-monospace, monospace;
      font-size: 0.75rem;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: #71717a;
    }
    .terminal-section pre[class*="language-"] {
      margin: 0 !important;
      padding: 0 !important;
      background: transparent !important;
      font-size: 0.875rem !important;
      line-height: 1.6 !important;
      overflow: visible !important;
    }
    .terminal-section code[class*="language-"] {
      font-family: ui-monospace, monospace !important;
      font-size: 0.875rem !important;
      white-space: pre-wrap !important;
      word-break: break-word !important;
    }
    /* Dark mode: custom terminal colors */
    .theme-dark .terminal-section code[class*="language-"] { color: #e4e4e7 !important; }
    .theme-dark .terminal-section .token.property { color: #a78bfa !important; }
    .theme-dark .terminal-section .token.string { color: #86efac !important; }
    .theme-dark .terminal-section .token.number { color: #fcd34d !important; }
    .theme-dark .terminal-section .token.boolean { color: #67e8f9 !important; }
    .theme-dark .terminal-section .token.null { color: #f87171 !important; }
    .theme-dark .terminal-section .token.punctuation { color: #a1a1aa !important; }
    /* Light mode: use Prism default light theme colors */
    .theme-light .terminal-section code[class*="language-"] { color: #383a42 !important; }
    .theme-light .terminal-section .token.property { color: #7c3aed !important; }
    .theme-light .terminal-section .token.string { color: #16a34a !important; }
    .theme-light .terminal-section .token.number { color: #ca8a04 !important; }
    .theme-light .terminal-section .token.boolean { color: #0891b2 !important; }
    .theme-light .terminal-section .token.null { color: #dc2626 !important; }
    .theme-light .terminal-section .token.punctuation { color: #71717a !important; }
  </style>
</head>
<body>
  <p class="host-info-subtitle" id="host-info-subtitle">Connecting...</p>
  <h1>MCP App Demo</h1>
  <div class="actions">
    <h2 class="actions-heading">Action Requests</h2>
    <div class="actions-buttons">
      <button class="action-btn" id="btn-open-link">
        Open Link <code>ui/open-link</code>
      </button>
      <button class="action-btn" id="btn-message">
        Send Message <code>ui/message</code>
      </button>
      <button class="action-btn" id="btn-size-change">
        Size Change <code>ui/notifications/size-changed</code>
      </button>
    </div>
  </div>
  <div class="actions">
    <h2 class="actions-heading">MCP Passthrough (forwarded to MCP server)</h2>
    <div class="actions-buttons">
      <button class="action-btn" id="btn-tools-call">
        Call Tool <code>tools/call</code>
      </button>
      <button class="action-btn" id="btn-resources-list">
        List Resources <code>resources/list</code>
      </button>
      <button class="action-btn" id="btn-resources-templates-list">
        List Templates <code>resources/templates/list</code>
      </button>
      <button class="action-btn" id="btn-resources-read">
        Read Resource <code>resources/read</code>
      </button>
      <button class="action-btn" id="btn-prompts-list">
        List Prompts <code>prompts/list</code>
      </button>
      <button class="action-btn" id="btn-notifications-message">
        Log Message <code>notifications/message</code>
      </button>
      <button class="action-btn" id="btn-ping">
        Ping <code>ping</code>
      </button>
    </div>
  </div>
  <div class="terminal">
    <div class="terminal-grid">
      <div class="terminal-section">
        <h2>Tool Input</h2>
        <pre class="language-json"><code class="language-json" id="tool-input-content">Waiting...</code></pre>
      </div>
      <div class="terminal-section">
        <h2>Tool Result</h2>
        <pre class="language-json"><code class="language-json" id="tool-result-content">Waiting...</code></pre>
      </div>
    </div>
    <div class="terminal-grid" style="border-top: 1px solid var(--border);">
      <div class="terminal-section">
        <h2>Host Context</h2>
        <pre class="language-json"><code class="language-json" id="context-content">Initializing...</code></pre>
      </div>
      <div class="terminal-section">
        <h2>Host Info</h2>
        <pre class="language-json"><code class="language-json" id="host-support-content">...</code></pre>
      </div>
    </div>
  </div>
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
        // Update subtitle with host info
        const subtitle = document.getElementById('host-info-subtitle');
        if (subtitle && result.hostInfo) {
          const name = result.hostInfo.name || 'Unknown Host';
          const version = result.hostInfo.version || '';
          subtitle.textContent = version ? name + ' v' + version : name;
        }

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

        // Handle tool-input notification
        if (data.method === 'ui/notifications/tool-input') {
          renderJson('tool-input-content', data.params);
          sendSizeChanged();
        }

        // Handle tool-result notification
        if (data.method === 'ui/notifications/tool-result') {
          renderJson('tool-result-content', data.params);
          sendSizeChanged();
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

      // Action button: Open Link
      document.getElementById('btn-open-link').addEventListener('click', function() {
        sendRequest('ui/open-link', {
          url: 'https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx'
        }).then(function(result) {
          console.log('[MockApp] ui/open-link response:', result);
        }).catch(function(error) {
          console.error('[MockApp] ui/open-link error:', error);
        });
      });

      // Action button: Send Message
      document.getElementById('btn-message').addEventListener('click', function() {
        sendRequest('ui/message', {
          role: 'user',
          content: {
            type: 'text',
            text: 'Hello from MCP App Demo! This message was sent via ui/message.'
          }
        }).then(function(result) {
          console.log('[MockApp] ui/message response:', result);
        }).catch(function(error) {
          console.error('[MockApp] ui/message error:', error);
        });
      });

      // Action button: Size Change (reports actual body size)
      document.getElementById('btn-size-change').addEventListener('click', function() {
        sendSizeChanged();
        console.log('[MockApp] ui/notifications/size-changed sent (no response expected)');
      });

      // Action button: Call Tool
      document.getElementById('btn-tools-call').addEventListener('click', function() {
        sendRequest('tools/call', {
          name: 'example_tool',
          arguments: { param1: 'value1', param2: 42 }
        }).then(function(result) {
          console.log('[MockApp] tools/call response:', result);
        }).catch(function(error) {
          console.error('[MockApp] tools/call error:', error);
        });
      });

      // Action button: List Resources
      document.getElementById('btn-resources-list').addEventListener('click', function() {
        sendRequest('resources/list', {}).then(function(result) {
          console.log('[MockApp] resources/list response:', result);
        }).catch(function(error) {
          console.error('[MockApp] resources/list error:', error);
        });
      });

      // Action button: List Resource Templates
      document.getElementById('btn-resources-templates-list').addEventListener('click', function() {
        sendRequest('resources/templates/list', {}).then(function(result) {
          console.log('[MockApp] resources/templates/list response:', result);
        }).catch(function(error) {
          console.error('[MockApp] resources/templates/list error:', error);
        });
      });

      // Action button: Read Resource
      document.getElementById('btn-resources-read').addEventListener('click', function() {
        sendRequest('resources/read', {
          uri: 'resource://example/demo-resource'
        }).then(function(result) {
          console.log('[MockApp] resources/read response:', result);
        }).catch(function(error) {
          console.error('[MockApp] resources/read error:', error);
        });
      });

      // Action button: List Prompts
      document.getElementById('btn-prompts-list').addEventListener('click', function() {
        sendRequest('prompts/list', {}).then(function(result) {
          console.log('[MockApp] prompts/list response:', result);
        }).catch(function(error) {
          console.error('[MockApp] prompts/list error:', error);
        });
      });

      // Action button: Log Message (notification)
      document.getElementById('btn-notifications-message').addEventListener('click', function() {
        sendNotification('notifications/message', {
          level: 'info',
          data: 'This is a log message from the MCP App Demo!',
          logger: 'MockMcpApp'
        });
        console.log('[MockApp] notifications/message sent (no response expected)');
      });

      // Action button: Ping
      document.getElementById('btn-ping').addEventListener('click', function() {
        sendRequest('ping', {}).then(function(result) {
          console.log('[MockApp] ping response:', result);
        }).catch(function(error) {
          console.error('[MockApp] ping error:', error);
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
      name: 'Demo MCP App',
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
