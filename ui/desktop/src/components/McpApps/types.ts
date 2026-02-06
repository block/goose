export type { CspMetadata, CallToolResponse as ToolResult } from '../../api/types.gen';

/**
 * Valid iframe sandbox attribute tokens.
 * @see https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe#sandbox
 */
export type SandboxToken =
  | 'allow-downloads'
  | 'allow-forms'
  | 'allow-modals'
  | 'allow-orientation-lock'
  | 'allow-pointer-lock'
  | 'allow-popups'
  | 'allow-popups-to-escape-sandbox'
  | 'allow-presentation'
  | 'allow-same-origin'
  | 'allow-scripts'
  | 'allow-storage-access-by-user-activation'
  | 'allow-top-navigation'
  | 'allow-top-navigation-by-user-activation'
  | 'allow-top-navigation-to-custom-protocols';

/**
 * Space-separated string of sandbox tokens for iframe sandbox attribute.
 * While typed as string for flexibility, valid values are space-separated SandboxToken values.
 * @example "allow-scripts allow-same-origin allow-forms"
 */
export type SandboxPermissions = string;

export type ContentBlock =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mimeType: string }
  | {
      type: 'resource';
      resource: { uri: string; mimeType?: string; text?: string; blob?: string };
    };

export type McpMethodParams = {
  'ui/open-link': { url: string };
  'ui/message': { role: 'user'; content: ContentBlock[] };
  'tools/call': { name: string; arguments?: Record<string, unknown> };
  'resources/read': { uri: string };
  'notifications/message': { level?: string; logger?: string; data: unknown };
  ping: Record<string, never>;
};

export type McpMethodResponse = {
  'ui/open-link': { status: string; message: string };
  'ui/message': Record<string, never>;
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
