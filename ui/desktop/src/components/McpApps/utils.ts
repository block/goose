import {
  CspMetadata,
  HostContext,
  JsonRpcNotification,
  JsonRpcResponse,
  ToolInput,
  ToolResult,
  SizeChangedNotification,
  OpenLinkRequest,
  MessageRequest,
  LoggingMessageRequest,
  CallToolRequest,
  ListResourcesRequest,
  ListResourceTemplatesRequest,
  ReadResourceRequest,
  ListPromptsRequest,
  PingRequest,
} from './types';
import packageJson from '../../../package.json';

// =============================================================================
// JSON-RPC Response Helpers
// =============================================================================

/** Standard JSON-RPC error codes */
export const JsonRpcErrorCode = {
  ParseError: -32700,
  InvalidRequest: -32600,
  MethodNotFound: -32601,
  InvalidParams: -32602,
  InternalError: -32603,
} as const;

/**
 * Create a successful JSON-RPC response.
 */
export function createSuccessResponse(id: string | number, result: unknown = {}): JsonRpcResponse {
  return {
    jsonrpc: '2.0',
    id,
    result,
  };
}

/**
 * Create an error JSON-RPC response.
 */
export function createErrorResponse(
  id: string | number,
  code: number,
  message: string,
  data?: unknown
): JsonRpcResponse {
  return {
    jsonrpc: '2.0',
    id,
    error: {
      code,
      message,
      ...(data !== undefined && { data }),
    },
  };
}

/**
 * Create a "method not implemented" error response.
 */
export function createNotImplementedResponse(id: string | number, method: string): JsonRpcResponse {
  return createErrorResponse(
    id,
    JsonRpcErrorCode.MethodNotFound,
    `Method not implemented: ${method}`
  );
}

/**
 * Fetch the MCP App proxy URL from the Electron backend.
 *
 * @param csp - Optional CSP metadata to include in the URL. The outer sandbox
 *              CSP will be templated to allow these domains, acting as a ceiling
 *              for what the inner guest UI CSP can permit.
 */
export async function fetchMcpAppProxyUrl(csp?: CspMetadata | null): Promise<string | null> {
  try {
    const baseUrl = await window.electron.getGoosedHostPort();
    const secretKey = await window.electron.getSecretKey();
    if (baseUrl && secretKey) {
      const params = new URLSearchParams();
      params.set('secret', secretKey);

      // Include CSP domains if provided
      if (csp?.connectDomains?.length) {
        params.set('connect_domains', csp.connectDomains.join(','));
      }
      if (csp?.resourceDomains?.length) {
        params.set('resource_domains', csp.resourceDomains.join(','));
      }

      return `${baseUrl}/mcp-app-proxy?${params.toString()}`;
    }
    console.error('Failed to get goosed host/port or secret key');
    return null;
  } catch (error) {
    console.error('Error fetching MCP App Proxy URL:', error);
    return null;
  }
}

/**
 * Create a tool-input notification to send tool arguments to the guest UI.
 */
export function createToolInputNotification(toolInput: ToolInput): JsonRpcNotification {
  return {
    jsonrpc: '2.0',
    method: 'ui/notifications/tool-input',
    params: { arguments: toolInput.arguments },
  };
}

/**
 * Create a tool-result notification to send tool execution result to the guest UI.
 */
export function createToolResultNotification(toolResult: ToolResult): JsonRpcNotification {
  return {
    jsonrpc: '2.0',
    method: 'ui/notifications/tool-result',
    params: toolResult as unknown as Record<string, unknown>,
  };
}

/**
 * Create a sandbox-resource-ready notification to send HTML content to the sandbox.
 */
export function createSandboxResourceReadyMessage(
  html: string,
  csp: Record<string, string[]> | null
): JsonRpcNotification {
  return {
    jsonrpc: '2.0',
    method: 'ui/notifications/sandbox-resource-ready',
    params: { html, csp },
  };
}

/**
 * Create a host-context-changed notification for incremental updates.
 * Only the changed fields need to be provided.
 */
export function createHostContextChangedNotification(
  hostContext: Partial<HostContext>
): JsonRpcNotification {
  return {
    jsonrpc: '2.0',
    method: 'ui/notifications/host-context-changed',
    params: hostContext,
  };
}

const MCP_PROTOCOL_VERSION = '2025-06-18';

/**
 * Create an initialize response with host capabilities and context.
 */
export function createInitializeResponse(
  requestId: string | number,
  hostContext: HostContext
): JsonRpcResponse {
  return {
    jsonrpc: '2.0',
    id: requestId,
    result: {
      protocolVersion: MCP_PROTOCOL_VERSION,
      hostCapabilities: {
        links: true,
        messages: true,
      },
      hostInfo: {
        name: packageJson.productName,
        version: packageJson.version,
      },
      hostContext,
    },
  };
}

// =============================================================================
// Message Handlers
// Handlers return JsonRpcResponse | null:
// - Requests (with id) should return a response
// - Notifications (without id) return null
// =============================================================================

/**
 * Handle ui/message requests from the guest UI.
 * Per spec: Host SHOULD add the message to the conversation context, preserving the specified role.
 * Host MAY request user consent.
 */
export function handleMessage(msg: MessageRequest): JsonRpcResponse | null {
  console.warn(
    '[MCP Apps] TODO ui/message: Should add message to chat conversation with specified role.',
    'Host MAY request user consent before adding.',
    { role: msg.params.content?.type, text: msg.params.content?.text }
  );
  if (msg.id !== undefined) {
    return createNotImplementedResponse(msg.id, msg.method);
  }
  return null;
}

/**
 * Handle ui/open-link requests from the guest UI.
 * Per spec: Host SHOULD open the URL in the user's default browser or a new tab.
 */
export function handleOpenLink(msg: OpenLinkRequest): JsonRpcResponse | null {
  const { url } = msg.params;
  window.electron.openExternal(url).catch(console.error);
  if (msg.id !== undefined) {
    return createSuccessResponse(msg.id, {});
  }
  return null;
}

/**
 * Handle notifications/message from the guest UI.
 * Per spec: Log messages to host. This is a standard MCP logging notification.
 * Host should forward to the MCP server.
 */
export function handleNotificationMessage(msg: LoggingMessageRequest): null {
  // TODO: Forward to MCP server
  console.warn('[MCP Apps] TODO notifications/message: Should forward to MCP server.', {
    level: msg.params.level,
    data: msg.params.data,
    logger: msg.params.logger,
  });
  return null;
}

/**
 * Handle tools/call requests from the guest UI.
 * Per spec: Execute a tool on the MCP server. Host MUST forward to the MCP server
 * that owns this App. Host MUST reject requests for tools that don't include "app" in visibility.
 */
export function handleToolsCall(msg: CallToolRequest): JsonRpcResponse | null {
  console.warn(
    '[MCP Apps] tools/call: Should forward to MCP server to execute tool.',
    'Host MUST reject if tool visibility does not include "app".',
    { tool: msg.params.name, arguments: msg.params.arguments }
  );
  if (msg.id !== undefined) {
    return createNotImplementedResponse(msg.id, msg.method);
  }
  return null;
}

/**
 * Handle resources/list requests from the guest UI.
 * Per spec: List available resources from the MCP server.
 * Host MAY forward to MCP server or return cached resource list.
 */
export function handleResourcesList(msg: ListResourcesRequest): JsonRpcResponse | null {
  console.warn(
    '[MCP Apps] TODO resources/list: Should return list of available resources from MCP server.',
    { cursor: msg.params?.cursor }
  );
  if (msg.id !== undefined) {
    return createNotImplementedResponse(msg.id, msg.method);
  }
  return null;
}

/**
 * Handle resources/templates/list requests from the guest UI.
 * Per spec: List available resource templates from the MCP server.
 */
export function handleResourceTemplatesList(
  msg: ListResourceTemplatesRequest
): JsonRpcResponse | null {
  console.warn(
    '[MCP Apps] TODO resources/templates/list: Should return list of resource templates from MCP server.',
    { cursor: msg.params?.cursor }
  );
  if (msg.id !== undefined) {
    return createNotImplementedResponse(msg.id, msg.method);
  }
  return null;
}

/**
 * Handle resources/read requests from the guest UI.
 * Per spec: Read resource content from the MCP server.
 * This is how Apps fetch data or additional UI resources.
 */
export function handleResourcesRead(msg: ReadResourceRequest): JsonRpcResponse | null {
  console.warn('[MCP Apps] TODO resources/read: Should fetch resource content from MCP server.', {
    uri: msg.params.uri,
  });
  if (msg.id !== undefined) {
    return createNotImplementedResponse(msg.id, msg.method);
  }
  return null;
}

/**
 * Handle prompts/list requests from the guest UI.
 * Per spec: List available prompts from the MCP server.
 */
export function handlePromptsList(msg: ListPromptsRequest): JsonRpcResponse | null {
  console.warn(
    '[MCP Apps] TODO prompts/list: Should return list of available prompts from MCP server.',
    { cursor: msg.params?.cursor }
  );
  if (msg.id !== undefined) {
    return createNotImplementedResponse(msg.id, msg.method);
  }
  return null;
}

/**
 * Handle ping requests from the guest UI.
 * Per spec: Connection health check. Should forward to MCP server and return its response.
 */
export function handlePing(msg: PingRequest): JsonRpcResponse | null {
  // TODO: Forward ping to MCP server and return its response
  console.warn('[MCP Apps] TODO ping: Should forward to MCP server and return its response.');
  if (msg.id !== undefined) {
    return createNotImplementedResponse(msg.id, msg.method);
  }
  return null;
}

const DEFAULT_IFRAME_HEIGHT = 200;

/**
 * Handle ui/notifications/size-changed from the guest UI.
 * This is a notification, so no response is sent.
 * Returns a handler function that updates iframe height.
 */
export function handleSizeChanged(setIframeHeight: (height: number) => void) {
  return (msg: SizeChangedNotification): null => {
    const newHeight = Math.max(DEFAULT_IFRAME_HEIGHT, msg.params.height);
    setIframeHeight(newHeight);
    return null;
  };
}
