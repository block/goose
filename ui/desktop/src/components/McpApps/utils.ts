import {
  CspMetadata,
  HostContext,
  JsonRpcNotification,
  JsonRpcResponse,
  ToolInput,
  ToolResult,
} from './types';
import packageJson from '../../../package.json';

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
