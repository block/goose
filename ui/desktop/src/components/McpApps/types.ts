export type { CspMetadata, CallToolResponse as ToolResult } from '../../api/types.gen';

export type McpMethodParams = {
  'ui/open-link': { url: string };
  'ui/message': { content: { type: string; text: string } };
  'tools/call': { name: string; arguments?: Record<string, unknown> };
  'resources/read': { uri: string };
  'notifications/message': { level?: string; logger?: string; data: unknown };
  ping: Record<string, never>;
};

export type McpMethodResponse = {
  'ui/open-link': { status: string; message: string };
  'ui/message': { status: string; message: string };
  'tools/call': {
    content: unknown[];
    isError: boolean;
    structuredContent?: Record<string, unknown>;
  };
  'resources/read': { contents: unknown[] };
  'notifications/message': Record<string, never>;
  ping: Record<string, never>;
};

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  id?: string | number;
  method: string;
  params?: Record<string, unknown>;
}

export interface JsonRpcNotification {
  jsonrpc: '2.0';
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
  safeAreaInsets: {
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
