import type { CspMetadata, PermissionsMetadata } from './types';

export const DEFAULT_IFRAME_HEIGHT = 200;

// Extended CSP metadata type that includes all fields (some may not be in generated types yet)
interface ExtendedCspMetadata {
  connectDomains?: string[] | null;
  resourceDomains?: string[] | null;
  frameDomains?: string[] | null;
  baseUriDomains?: string[] | null;
}

/**
 * Create a secure MCP App View URL.
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
 * 1. POST HTML + metadata to /mcp-app-view
 * 2. Backend stores it temporarily and returns a token
 * 3. Use the returned URL as iframe src
 * 4. Backend serves HTML with proper CSP headers
 * 5. Token is single-use and expires after 60 seconds
 */
export async function createMcpAppViewUrl(
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

    // Cast to extended type to access all CSP fields
    const extendedCsp = csp as ExtendedCspMetadata | null | undefined;

    console.log('[MCP App View] Creating view URL', { baseUrl, secretKeyLength: secretKey?.length });
    const response = await fetch(`${baseUrl}/mcp-app-view`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        secret: secretKey,
        html,
        csp: extendedCsp ? {
          connectDomains: extendedCsp.connectDomains || [],
          resourceDomains: extendedCsp.resourceDomains || [],
          frameDomains: extendedCsp.frameDomains || [],
          baseUriDomains: extendedCsp.baseUriDomains || [],
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
      console.error('Failed to create MCP App View:', response.statusText);
      return null;
    }

    const data = await response.json();
    return `${baseUrl}${data.url}`;
  } catch (error) {
    console.error('Error creating MCP App View URL:', error);
    return null;
  }
}
