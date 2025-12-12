// JSON-RPC 2.0 message types
export interface JsonRpcNotification {
  jsonrpc: '2.0';
  method: string;
  params?: Record<string, unknown>;
}

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: string | number;
  method: string;
  params?: Record<string, unknown>;
}

export interface JsonRpcResponse {
  jsonrpc: '2.0';
  id: string | number;
  result?: unknown;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export type JsonRpcMessage = JsonRpcNotification | JsonRpcRequest | JsonRpcResponse;

// MCP App resource type
export interface McpAppResource {
  uri: `ui://${string}`;
  description?: string;
  mimeType: `text/html;profile=mcp-app`;
  text?: string;
  _meta?: {
    ui?: {
      csp?: {
        connectDomains?: string[];
        resourceDomains?: string[];
      };
      domain?: `https://${string}`;
      prefersBorder?: boolean;
    };
  };
}

// Tool input passed to the MCP App
export interface ToolInput {
  arguments: Record<string, unknown>;
}

// Tool result passed to the MCP App (matches MCP CallToolResult)
export interface ToolResult {
  content?: unknown[];
  structuredContent?: Record<string, unknown>;
  isError?: boolean;
  _meta?: Record<string, unknown>;
}
