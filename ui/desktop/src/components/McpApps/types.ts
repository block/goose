// Re-export generated types from Rust
export type {
  McpAppResource,
  CspMetadata,
  UiMetadata,
  ResourceMetadata,
  CallToolResponse as ToolResult,
} from '../../api/types.gen';

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  id?: string | number;
  method: string;
  params?: unknown;
}

export interface JsonRpcNotification {
  jsonrpc: '2.0';
  method: string;
  params?: unknown;
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

export type JsonRpcMessage = JsonRpcRequest | JsonRpcNotification | JsonRpcResponse;

export interface HostContext {
  toolInfo?: {
    id?: string | number;
    tool: {
      name: string;
      description?: string;
      inputSchema?: Record<string, unknown>;
    };
  };
  theme: 'light' | 'dark';
  displayMode: 'inline' | 'fullscreen' | 'standalone';
  availableDisplayModes: ('inline' | 'fullscreen' | 'standalone')[];
  viewport: {
    width: number;
    height: number;
    maxHeight: number;
    maxWidth: number;
  };
  locale: string;
  timeZone: string;
  userAgent: string;
  platform: 'web' | 'desktop' | 'mobile';
  deviceCapabilities: {
    touch: boolean;
    hover: boolean;
  };
  safeAreaInsets?: {
    top: number;
    right: number;
    bottom: number;
    left: number;
  };
}

export interface ToolInput {
  arguments: Record<string, unknown>;
}

export interface ToolInputPartial {
  arguments: Record<string, unknown>;
}

export interface ToolCancelled {
  reason?: string;
}
