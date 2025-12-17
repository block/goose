// =============================================================================
// JSON-RPC 2.0 Base Types
// =============================================================================

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

export interface JsonRpcNotification {
  jsonrpc: '2.0';
  method: string;
  params?: Record<string, unknown>;
}

export type JsonRpcMessage = JsonRpcNotification | JsonRpcResponse;

// =============================================================================
// Incoming Guest Messages (discriminated union for type-safe switching)
// =============================================================================

export interface SandboxReadyNotification {
  jsonrpc: '2.0';
  method: 'ui/notifications/sandbox-ready';
}

export interface InitializeRequest {
  jsonrpc: '2.0';
  id: string | number;
  method: 'ui/initialize';
  params?: Record<string, unknown>;
}

export interface InitializedNotification {
  jsonrpc: '2.0';
  method: 'ui/notifications/initialized';
}

export interface SizeChangedNotification {
  jsonrpc: '2.0';
  method: 'ui/notifications/size-changed';
  params: {
    height: number;
    width?: number;
  };
}

export interface OpenLinkRequest {
  jsonrpc: '2.0';
  id?: string | number;
  method: 'ui/open-link';
  params: {
    url: string;
  };
}

export interface MessageRequest {
  jsonrpc: '2.0';
  id?: string | number;
  method: 'ui/message';
  params: {
    content: {
      type: string;
      text: string;
    };
  };
}

type LoggingLevel =
  | 'debug'
  | 'info'
  | 'notice'
  | 'warning'
  | 'error'
  | 'critical'
  | 'alert'
  | 'emergency';
export interface LoggingMessageRequest {
  jsonrpc: '2.0';
  method: 'notifications/message';
  params: {
    _meta?: { [key: string]: unknown };
    data: string;
    level: LoggingLevel;
    logger?: string;
  };
}

type ProgressToken = string | number;
interface TaskMetadata {
  ttl?: number;
}

export interface CallToolRequest {
  jsonrpc: '2.0';
  id?: string | number;
  method: 'tools/call';
  params: {
    _meta?: { progressToken?: ProgressToken; [key: string]: unknown };
    arguments?: { [key: string]: unknown };
    name: string;
    task?: TaskMetadata;
  };
}

interface PaginatedRequestParams {
  cursor?: string;
}

export interface ListResourcesRequest {
  id?: string | number;
  jsonrpc: '2.0';
  method: 'resources/list';
  params?: PaginatedRequestParams;
}

export interface ListResourceTemplatesRequest {
  id?: string | number;
  jsonrpc: '2.0';
  method: 'resources/templates/list';
  params?: PaginatedRequestParams;
}

export interface ReadResourceRequest {
  id?: string | number;
  jsonrpc: '2.0';
  method: 'resources/read';
  params: {
    _meta?: { progressToken?: ProgressToken; [key: string]: unknown };
    uri: string;
  };
}

export interface ListPromptsRequest {
  id?: string | number;
  jsonrpc: '2.0';
  method: 'prompts/list';
  params?: PaginatedRequestParams;
}

export interface PingRequest {
  id?: string | number;
  jsonrpc: '2.0';
  method: 'ping';
  params?: Record<string, unknown>;
}

export type IncomingGuestMessage =
  | SandboxReadyNotification
  | InitializeRequest
  | InitializedNotification
  | SizeChangedNotification
  | OpenLinkRequest
  | MessageRequest
  | LoggingMessageRequest
  | CallToolRequest
  | ListResourcesRequest
  | ListResourceTemplatesRequest
  | ReadResourceRequest
  | ListPromptsRequest
  | PingRequest;

// =============================================================================
// MCP App Resource Type
// =============================================================================

export interface McpAppResource {
  uri: `ui://${string}`;
  name: string;
  description?: string;
  mimeType: 'text/html;profile=mcp-app';
  text?: string;
  blob?: string;
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

// =============================================================================
// Tool Types
// =============================================================================

/** Tool input passed to the MCP App */
export interface ToolInput {
  arguments: Record<string, unknown>;
}

/** Partial/streaming tool input passed to the MCP App */
export interface ToolInputPartial {
  arguments: Record<string, unknown>;
}

/** Tool result passed to the MCP App (matches MCP CallToolResult) */
export interface ToolResult {
  _meta?: Record<string, unknown>;
  content: unknown[];
  isError?: boolean;
  structuredContent?: Record<string, unknown>;
}

/** Tool cancellation notification */
export interface ToolCancelled {
  reason?: string;
}

// =============================================================================
// Host Context Types
// =============================================================================

/** CSP metadata for MCP Apps */
export interface CspMetadata {
  connectDomains?: string[];
  resourceDomains?: string[];
}

/** Host context sent to MCP Apps during initialization and updates */
export interface HostContext {
  /** Metadata of the tool call that instantiated the App */
  toolInfo?: {
    /** JSON-RPC id of the tools/call request */
    id?: string | number;
    /** Contains name, inputSchema, etc… */
    tool: {
      name: string;
      description?: string;
      inputSchema?: Record<string, unknown>;
    };
  };
  /** Current color theme preference */
  theme: 'light' | 'dark';
  /** How the UI is currently displayed
   * inline is the only supported mode for now
   * can support fullscreen and pip in the future
   */
  displayMode: 'inline';
  /** Display modes the host supports */
  availableDisplayModes: ['inline'];
  /** Current and maximum dimensions available to the UI */
  viewport: {
    width: number;
    height: number;
    maxHeight: number;
    maxWidth: number;
  };
  /** User's language/region preference (BCP 47, e.g., "en-US") */
  locale: string;
  /** User's timezone (IANA, e.g., "America/New_York") */
  timeZone: string;
  /** Host application identifier */
  userAgent: string;
  /** Platform type for responsive design */
  platform: 'web' | 'desktop' | 'mobile';
  /** Device capabilities such as touch */
  deviceCapabilities: {
    touch: boolean;
    hover: boolean;
  };
  /** Safe area boundaries in pixels */
  safeAreaInsets: {
    top: number;
    right: number;
    bottom: number;
    left: number;
  };
}
