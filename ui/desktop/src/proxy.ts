import type { Session } from 'electron';

type ProxySession = Pick<Session, 'setProxy'>;
export type ProxyEnvironment = Record<string, string | undefined>;

export async function configureProxy(
  defaultSession: ProxySession,
  rendererSession: ProxySession,
  environment: ProxyEnvironment = process.env
): Promise<void> {
  const httpsProxy = environment.HTTPS_PROXY || environment.https_proxy;
  const httpProxy = environment.HTTP_PROXY || environment.http_proxy;
  const proxyUrl = httpsProxy || httpProxy;

  if (!proxyUrl) {
    return;
  }

  console.log('[Main] Configuring proxy');
  const proxyConfig = {
    proxyRules: proxyUrl,
    proxyBypassRules: environment.NO_PROXY || environment.no_proxy || '',
  };

  await Promise.all([defaultSession.setProxy(proxyConfig), rendererSession.setProxy(proxyConfig)]);
  console.log('[Main] Proxy configured successfully');
}

// Mirrors the rules handed to Chromium in `configureProxy`, so main-process
// backend requests traverse the same forward proxy the renderer does.
export function proxyFor(target: URL, environment: ProxyEnvironment = process.env): URL | null {
  const httpsProxy = environment.HTTPS_PROXY || environment.https_proxy;
  const httpProxy = environment.HTTP_PROXY || environment.http_proxy;
  const secure = target.protocol === 'https:' || target.protocol === 'wss:';
  // `configureProxy` gives Chromium a single rule set with HTTPS_PROXY taking
  // precedence, so fall back the same way here.
  const proxyUrl = secure ? httpsProxy || httpProxy : httpProxy || httpsProxy;

  if (!proxyUrl || isBypassed(target, environment.NO_PROXY || environment.no_proxy)) {
    return null;
  }

  try {
    return new URL(proxyUrl.includes('://') ? proxyUrl : `http://${proxyUrl}`);
  } catch {
    return null;
  }
}

function isBypassed(target: URL, bypassRules: string | undefined): boolean {
  const hostname = target.hostname.replace(/^\[|\]$/g, '').toLowerCase();

  return (bypassRules ?? '')
    .split(',')
    .map((rule) => rule.trim().toLowerCase())
    .filter(Boolean)
    .some((rule) => {
      if (rule === '*') {
        return true;
      }
      const bare = rule.startsWith('.') ? rule.slice(1) : rule;
      return hostname === bare || hostname.endsWith(`.${bare}`);
    });
}
