export const DEFAULT_IFRAME_HEIGHT = 200;

/**
 * Fetch the MCP App proxy URL from the Electron backend.
 * The proxy enforces CSP as a security boundary for sandboxed apps.
 * TODO(Douwe): make this work better with the generated API rather than poking around
 */
export async function fetchMcpAppProxyUrl(
  csp?: {
    connectDomains?: string[] | null;
    resourceDomains?: string[] | null;
    frameDomains?: string[] | null;
    baseUriDomains?: string[] | null;
  } | null
): Promise<string | null> {
  try {
    const baseUrl = await window.electron.getGoosedHostPort();
    const secretKey = await window.electron.getSecretKey();
    if (!baseUrl || !secretKey) {
      console.error('Failed to get goosed host/port or secret key');
      return null;
    }

    const params = new URLSearchParams();
    params.set('secret', secretKey);

    if (csp?.connectDomains?.length) {
      params.set('connect_domains', csp.connectDomains.join(','));
    }
    if (csp?.resourceDomains?.length) {
      params.set('resource_domains', csp.resourceDomains.join(','));
    }
    if (csp?.frameDomains?.length) {
      params.set('frame_domains', csp.frameDomains.join(','));
    }
    if (csp?.baseUriDomains?.length) {
      params.set('base_uri_domains', csp.baseUriDomains.join(','));
    }

    return `${baseUrl}/mcp-app-proxy?${params.toString()}`;
  } catch (error) {
    console.error('Error fetching MCP App Proxy URL:', error);
    return null;
  }
}
