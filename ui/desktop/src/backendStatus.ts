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
  fetch,
  errorLog = [],
  options = {},
}: CheckBackendStatusParams): Promise<boolean> => {
  const deadline = Date.now() + HEALTHCHECK_TIMEOUT_MS;
  const statusUrl = statusHttpUrlFromHttpBase(baseUrl);
  const acpUrl = acpHttpUrlFromHttpBase(baseUrl, serverSecret);
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
      const response = await fetchWithTimeout(fetch, statusUrl, {
        headers: {
          'X-Secret-Key': serverSecret,
        },
      });
      if (response.ok) {
        const authResponse = await fetchWithTimeout(fetch, acpUrl);
        // GET /acp without an SSE Accept header returns 406 after auth succeeds.
        if (authResponse.status === 406) {
          options.onEvent?.('healthcheck_success', { attempt });
          return true;
        }
        if (authResponse.status === 401 || authResponse.status === 403) {
          options.onEvent?.('healthcheck_auth_failed', { attempt });
          return false;
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
