import { JsonRpcNotification, JsonRpcResponse, ToolInput, ToolResult } from './types';
import packageJson from '../../../package.json';

/**
 * CSP metadata for MCP Apps.
 */
export interface CspMetadata {
  connectDomains?: string[];
  resourceDomains?: string[];
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
    params: toolResult as Record<string, unknown>,
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
  theme?: 'light' | 'dark';
  /** How the UI is currently displayed */
  displayMode?: 'inline' | 'fullscreen' | 'pip';
  /** Display modes the host supports */
  availableDisplayModes?: string[];
  /** Current and maximum dimensions available to the UI */
  viewport?: {
    width: number;
    height: number;
    maxHeight?: number;
    maxWidth?: number;
  };
  /** User's language/region preference (BCP 47, e.g., "en-US") */
  locale?: string;
  /** User's timezone (IANA, e.g., "America/New_York") */
  timeZone?: string;
  /** Host application identifier */
  userAgent?: string;
  /** Platform type for responsive design */
  platform?: 'web' | 'desktop' | 'mobile';
  /** Device capabilities such as touch */
  deviceCapabilities?: {
    touch?: boolean;
    hover?: boolean;
  };
  /** Safe area boundaries in pixels */
  safeAreaInsets?: {
    top: number;
    right: number;
    bottom: number;
    left: number;
  };
}

/**
 * Get the current theme from localStorage.
 */
export function getCurrentTheme(): 'light' | 'dark' {
  const useSystemTheme = localStorage.getItem('use_system_theme') === 'true';
  if (useSystemTheme) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return localStorage.getItem('theme') === 'dark' ? 'dark' : 'light';
}

/**
 * Create a host-context-changed notification.
 */
export function createHostContextChangedNotification(
  hostContext: HostContext
): JsonRpcNotification {
  return {
    jsonrpc: '2.0',
    method: 'ui/notifications/host-context-changed',
    params: hostContext as Record<string, unknown>,
  };
}

/**
 * Create an initialize response with host capabilities and context.
 */
export function createInitializeResponse(
  requestId: string | number,
  hostContext?: HostContext
): JsonRpcResponse {
  return {
    jsonrpc: '2.0',
    id: requestId,
    result: {
      protocolVersion: '2025-06-18', // THIS IS THE MCP APP PROTOCOL VERSION GOOSE SUPPORTS
      hostCapabilities: {
        links: true,
        messages: true,
      },
      hostInfo: {
        name: packageJson.productName,
        version: packageJson.version,
      },
      hostContext: hostContext || {},
    },
  };
}
