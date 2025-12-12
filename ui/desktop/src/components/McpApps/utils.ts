import { JsonRpcNotification, JsonRpcResponse } from './types';

/**
 * Fetch the MCP App proxy URL from the Electron backend.
 */
export async function fetchMcpAppProxyUrl(): Promise<string | null> {
  try {
    const baseUrl = await window.electron.getGoosedHostPort();
    const secretKey = await window.electron.getSecretKey();
    if (baseUrl && secretKey) {
      return `${baseUrl}/mcp-app-proxy?secret=${encodeURIComponent(secretKey)}`;
    }
    console.error('Failed to get goosed host/port or secret key');
    return null;
  } catch (error) {
    console.error('Error fetching MCP App Proxy URL:', error);
    return null;
  }
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
 * Create an initialize response with host capabilities.
 */
export function createInitializeResponse(requestId: string | number): JsonRpcResponse {
  return {
    jsonrpc: '2.0',
    id: requestId,
    result: {
      protocolVersion: '2025-01-01',
      capabilities: {
        prompts: true,
        links: true,
        notifications: true,
      },
      hostInfo: {
        name: 'Goose Desktop',
        version: '1.0.0',
      },
    },
  };
}
