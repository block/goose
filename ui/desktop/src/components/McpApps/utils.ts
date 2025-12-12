import { JsonRpcNotification, JsonRpcResponse } from './types';
import packageJson from '../../../package.json';

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
