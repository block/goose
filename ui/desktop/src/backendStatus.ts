import { acpHttpUrlFromHttpBase, statusHttpUrlFromHttpBase } from './acp/url';

const HEALTHCHECK_TIMEOUT_MS = 30000;
const HEALTHCHECK_INTERVAL_MS = 100;
const PROBE_TIMEOUT_MS = 1000;

type FetchInput = Parameters<typeof globalThis.fetch>[0];
type FetchInit = Parameters<typeof globalThis.fetch>[1];

export interface CheckServerStatusOptions {
  onEvent?: (name: string, details?: Record<string, unknown>) => void;
}

export interface CheckBackendStatusParams {
  baseUrl: string;
  serverSecret: string;
  /** When `bearer`, probes use Authorization: Bearer instead of X-Secret-Key / ?token=. */
  authMode?: 'secret' | 'bearer';
  fetch: typeof globalThis.fetch;
  errorLog?: string[];
  options?: CheckServerStatusOptions;
}

export const isFatalError = (line: string): boolean => {
  const fatalPatterns = [/panicked at/, /RUST_BACKTRACE/, /fatal error/i];
  return fatalPatterns.some((pattern) => pattern.test(line));
};

const delay = (timeoutMs: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, timeoutMs));

const fetchWithTimeout = async (
  fetch: typeof globalThis.fetch,
  input: FetchInput,
  init?: FetchInit
): Promise<Response> => {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), PROBE_TIMEOUT_MS);

  try {
    return await fetch(input, { ...init, signal: controller.signal });
  } finally {
    clearTimeout(timeout);
  }
};

export const checkBackendStatus = async ({
  baseUrl,
  serverSecret,
  authMode = 'secret',
  fetch,
  errorLog = [],
  options = {},
}: CheckBackendStatusParams): Promise<boolean> => {
  const deadline = Date.now() + HEALTHCHECK_TIMEOUT_MS;
  const statusUrl = statusHttpUrlFromHttpBase(baseUrl);
  // Bearer mode: gateway /status is public; ACP auth uses Authorization header (no ?token=).
  const acpUrl =
    authMode === 'bearer'
      ? acpHttpUrlFromHttpBase(baseUrl)
      : acpHttpUrlFromHttpBase(baseUrl, serverSecret);
  options.onEvent?.('healthcheck_start', {
    timeoutMs: HEALTHCHECK_TIMEOUT_MS,
    intervalMs: HEALTHCHECK_INTERVAL_MS,
  });

  let attempt = 1;
  while (Date.now() < deadline) {
    if (errorLog.some(isFatalError)) {
      options.onEvent?.('healthcheck_fatal_error', { attempt });
      return false;
    }

    try {
      const statusHeaders: Record<string, string> =
        authMode === 'bearer'
          ? { authorization: `Bearer ${serverSecret}` }
          : { 'X-Secret-Key': serverSecret };
      const response = await fetchWithTimeout(fetch, statusUrl, {
        headers: statusHeaders,
      });
      // Gateway exposes /healthz;/readyz publicly; goose /status may ignore auth.
      // Accept either /status or /healthz for reachability when using bearer.
      const reachable =
        response.ok ||
        (authMode === 'bearer' &&
          (
            await fetchWithTimeout(fetch, new URL('/healthz', baseUrl).toString())
          ).ok);

      if (reachable || response.ok) {
        const authHeaders: Record<string, string> | undefined =
          authMode === 'bearer'
            ? { authorization: `Bearer ${serverSecret}` }
            : undefined;
        const authResponse = await fetchWithTimeout(fetch, acpUrl, {
          headers: authHeaders,
        });
        // GET /acp without an SSE Accept header returns 406 after auth succeeds
        // on local goose; gateway may return 401/403/404 depending on method.
        if (authResponse.status === 406 || authResponse.status === 405) {
          options.onEvent?.('healthcheck_success', { attempt });
          return true;
        }
        // Gateway POST-only /acp: a GET that reaches auth and returns 401 with
        // invalid method still proves connectivity when status/healthz already ok.
        if (authMode === 'bearer' && (response.ok || reachable) && authResponse.status === 404) {
          options.onEvent?.('healthcheck_success', { attempt });
          return true;
        }
        if (authResponse.status === 401 || authResponse.status === 403) {
          // For bearer probes against the gateway, 401/403 on GET /acp after a
          // healthy /healthz still means the gateway is up; role/token failures
          // surface later on real ACP traffic. Prefer success if healthz is ok.
          if (authMode === 'bearer' && reachable) {
            options.onEvent?.('healthcheck_success', { attempt });
            return true;
          }
          options.onEvent?.('healthcheck_auth_failed', { attempt });
          return false;
        }
        if (authMode === 'bearer' && authResponse.ok) {
          options.onEvent?.('healthcheck_success', { attempt });
          return true;
        }
      }
    } catch {
      // Retry until the backend is ready or the timeout expires.
    }

    await delay(HEALTHCHECK_INTERVAL_MS);
    attempt += 1;
  }

  options.onEvent?.('healthcheck_timeout', { timeoutMs: HEALTHCHECK_TIMEOUT_MS });
  return false;
};
