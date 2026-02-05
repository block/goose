import type { CspMetadata, PermissionsMetadata } from './types';

export const DEFAULT_IFRAME_HEIGHT = 200;

/**
 * Create a secure MCP App Proxy URL.
 *
 * This approach serves the MCP App HTML from a real URL endpoint instead of
 * using srcdoc iframes. This gives the MCP App a proper origin and secure
 * context, which is required for:
 * - Web Payments SDK (Square, Stripe, etc.)
 * - WebAuthn / Passkeys
 * - Certain OAuth flows
 * - Any API that checks window.isSecureContext
 *
 * Flow:
 * 1. POST HTML + metadata to /mcp-app-proxy
 * 2. Backend stores it temporarily and returns a token
 * 3. Use the returned URL as iframe src
 * 4. Backend serves HTML with proper CSP headers
 * 5. Token expires after 5 minutes
 */
export async function createMcpAppProxyUrl(
  html: string,
  csp?: CspMetadata | null,
  permissions?: PermissionsMetadata | null
): Promise<string | null> {
  try {
    const baseUrl = await window.electron.getGoosedHostPort();
    const secretKey = await window.electron.getSecretKey();
    if (!baseUrl || !secretKey) {
      console.error('Failed to get goosed host/port or secret key');
      return null;
    }

    console.log('[MCP App Proxy] Creating proxy URL', { baseUrl, secretKeyLength: secretKey?.length });
    const response = await fetch(`${baseUrl}/mcp-app-proxy`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        secret: secretKey,
        html,
        csp: csp ? {
          connectDomains: csp.connectDomains || [],
          resourceDomains: csp.resourceDomains || [],
          frameDomains: csp.frameDomains || [],
          baseUriDomains: csp.baseUriDomains || [],
        } : {},
        permissions: permissions ? {
          camera: permissions.camera || false,
          microphone: permissions.microphone || false,
          geolocation: permissions.geolocation || false,
          clipboardWrite: permissions.clipboardWrite || false,
        } : {},
      }),
    });

    if (!response.ok) {
      console.error('Failed to create MCP App Proxy:', response.statusText);
      return null;
    }

    const data = await response.json();
    return `${baseUrl}/mcp-app-proxy/${data.token}`;
  } catch (error) {
    console.error('Error creating MCP App Proxy URL:', error);
    return null;
  }
}
