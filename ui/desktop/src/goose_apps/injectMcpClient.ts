import { GooseApp } from '../api';

export function injectMCPClient(app: GooseApp): string {
  const mcpClientScript = `
<script type="module">
  class MCPClient {
    constructor() {
      this.nextId = 1;
      this.pending = new Map();
      this.notificationHandlers = new Map();
      
      window.addEventListener('message', (event) => {
        const msg = event.data;
        if (!msg || msg.jsonrpc !== '2.0') return;
        
        console.log("msg", msg);
        
        if (msg.id && this.pending.has(msg.id)) {
          const { resolve, reject } = this.pending.get(msg.id);
          this.pending.delete(msg.id);
          
          if (msg.error) {
            reject(new Error(msg.error.message));
          } else {
            resolve(msg.result);
          }
        } else if (msg.method) {
          const handlers = this.notificationHandlers.get(msg.method);
          if (handlers) {
            handlers.forEach(h => h(msg.params));
          }
        }
      });
      
      window.addEventListener('error', (event) => {
        const errorMsg = {
          type: 'mcp-app-error',
          error: event.error ? event.error.message : event.message,
          stack: event.error?.stack,
        };
        window.parent.postMessage(errorMsg, '*');
        console.error('MCP App Error:', event.error || event.message);
      }, true);
      
      window.addEventListener('unhandledrejection', (event) => {
        window.parent.postMessage({
          type: 'mcp-app-error',
          error: event.reason?.message || String(event.reason),
        }, '*');
        console.error('MCP App Unhandled Promise Rejection:', event.reason);
      });
    }
    
    async request(method, params) {
      const id = this.nextId++;
      return new Promise((resolve, reject) => {
        this.pending.set(id, { resolve, reject });
        window.parent.postMessage({
          jsonrpc: '2.0',
          id,
          method,
          params
        }, '*');
      });
    }
    
    notify(method, params) {
      window.parent.postMessage({
        jsonrpc: '2.0',
        method,
        params
      }, '*');
    }
    
    onNotification(method, handler) {
      if (!this.notificationHandlers.has(method)) {
        this.notificationHandlers.set(method, []);
      }
      this.notificationHandlers.get(method).push(handler);
    }
    
    async callTool(name, args) {
      return this.request('tools/call', { name, arguments: args });
    }
    
    async readResource(uri) {
      return this.request('resources/read', { uri });
    }
    
    async sendMessage(text) {
      return this.request('ui/message', {
        role: 'user',
        content: { type: 'text', text }
      });
    }
    
    async openLink(url) {
      return this.request('ui/open-link', { url });
    }
  }
  
  window.mcp = new MCPClient();
  
  window.mcp.request('ui/initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: '${app.name}', version: '1.0.0' }
  }).then(result => {
    console.log('MCP initialized', result);
    window.dispatchEvent(new CustomEvent('mcp-ready', { detail: result }));
  });
</script>
`;

  const html = app.html || "";

  if (html.includes('</head>')) {
    return html.replace('</head>', `${mcpClientScript}</head>`);
  } else if (html.includes('<body')) {
    return html.replace(/<body[^>]*>/, (match) => `${match}${mcpClientScript}`);
  } else {
    return mcpClientScript + html;
  }
}
