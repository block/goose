/**
 * Types for MCP Apps integration.
 *
 * Types are sourced from:
 * - `@mcp-ui/client` - AppRenderer component types
 * - `../../api/types.gen` - Auto-generated from Rust backend
 * - Manual definitions - For MCP protocol types not exported by SDK
 */

// Re-export types from generated API (Rust backend)
export type { CspMetadata, CallToolResponse as ToolResult } from '../../api/types.gen';

// Re-export types from @mcp-ui/client SDK
export type { McpUiHostContext as HostContext } from '@mcp-ui/client';

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

/**
 * Tool input arguments passed to MCP apps.
 * This wraps the arguments to match our message stream format.
 * McpAppRenderer extracts `.arguments` when passing to AppRenderer.
 */
export interface ToolInput {
  arguments: Record<string, unknown>;
}

/**
 * Partial tool input for streaming updates.
 * Same structure as ToolInput - represents incremental argument updates.
 */
export interface ToolInputPartial {
  arguments: Record<string, unknown>;
}

/**
 * Tool cancellation state from the message stream.
 * McpAppRenderer converts this to a boolean for AppRenderer.
 */
export interface ToolCancelled {
  reason?: string;
}

// ============================================================================
// MCP Protocol Types
// These types represent the JSON-RPC protocol used by MCP apps.
// They are manually defined as they're not exported by the SDK.
// ============================================================================

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
