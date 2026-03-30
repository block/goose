import type { ExternalGooseServerConfig } from './settings';

const DEFAULT_CONNECT_SOURCES = [
  "'self'",
  'http://127.0.0.1:*',
  'https://127.0.0.1:*',
  'http://localhost:*',
  'https://localhost:*',
  'https://api.github.com',
  'https://github.com',
  'https://objects.githubusercontent.com',
];

export function buildConnectSrc(externalGooseServer?: ExternalGooseServerConfig): string {
  const sources = [...DEFAULT_CONNECT_SOURCES];

  if (externalGooseServer?.enabled && externalGooseServer.url) {
    try {
      const externalUrl = new URL(externalGooseServer.url);
      sources.push(externalUrl.origin);
    } catch {
      console.warn('Invalid external goose server URL in settings, skipping CSP entry');
    }
  }

  return sources.join(' ');
}

/**
 * Returns true when upgrade-insecure-requests should be included in the CSP.
 *
 * The directive is omitted when the user has configured an external backend
 * that uses plain HTTP, because Chromium would silently rewrite those
 * requests to HTTPS. The remote server typically does not speak TLS, so the
 * upgraded requests fail with "Failed to fetch".
 *
 * Loopback addresses (127.0.0.1 / localhost) are exempt from the upgrade
 * per the CSP spec, which is why the built-in local backend is unaffected.
 */
export function shouldUpgradeInsecureRequests(
  externalGooseServer?: ExternalGooseServerConfig
): boolean {
  if (!externalGooseServer?.enabled || !externalGooseServer.url) {
    return true;
  }

  try {
    const parsed = new URL(externalGooseServer.url);
    return parsed.protocol !== 'http:';
  } catch {
    return true;
  }
}

export function buildCSP(externalGooseServer?: ExternalGooseServerConfig): string {
  const connectSrc = buildConnectSrc(externalGooseServer);
  const upgradeDirective = shouldUpgradeInsecureRequests(externalGooseServer)
    ? 'upgrade-insecure-requests;'
    : '';

  return (
    "default-src 'self';" +
    "style-src 'self' 'unsafe-inline';" +
    "script-src 'self' 'unsafe-inline';" +
    "img-src 'self' data: https:;" +
    `connect-src ${connectSrc};` +
    "object-src 'none';" +
    "frame-src 'self' https: http:;" +
    "font-src 'self' data: https:;" +
    "media-src 'self' mediastream:;" +
    "form-action 'none';" +
    "base-uri 'self';" +
    "manifest-src 'self';" +
    "worker-src 'self';" +
    upgradeDirective
  );
}
