import { describe, expect, it, vi } from 'vitest';
import { checkBackendStatus } from './backendStatus';

type FetchInput = Parameters<typeof globalThis.fetch>[0];
type FetchInit = NonNullable<Parameters<typeof globalThis.fetch>[1]>;

const fetchInputUrl = (input: FetchInput): string => {
  if (typeof input === 'string') {
    return input;
  }
  if (input instanceof URL) {
    return input.toString();
  }
  return input.url;
};

type FetchSignal = NonNullable<FetchInit['signal']>;

const expectAbortSignal = (init?: FetchInit): FetchSignal => {
  expect(init?.signal).toBeInstanceOf(globalThis.AbortSignal);
  return init!.signal!;
};

describe('checkBackendStatus', () => {
  it('checks /status and validates the secret against /acp', async () => {
    const fetch = vi.fn(async (input: FetchInput, init?: FetchInit) => {
      const url = fetchInputUrl(input);
      if (url === 'https://example.com/goose/status') {
        expect(init?.headers).toEqual({ 'X-Secret-Key': 'test-secret' });
        expectAbortSignal(init);
        return new Response(null, { status: 200 });
      }
      if (url === 'https://example.com/goose/acp?token=test-secret') {
        expectAbortSignal(init);
        return new Response(null, { status: 406 });
      }

      throw new Error(`Unexpected URL: ${url}`);
    });

    await expect(
      checkBackendStatus({
        baseUrl: 'https://example.com/goose',
        serverSecret: 'test-secret',
        fetch,
      })
    ).resolves.toBe(true);

    expect(fetch).toHaveBeenCalledTimes(2);
    expect(fetch.mock.calls.map(([input]) => fetchInputUrl(input))).toEqual([
      'https://example.com/goose/status',
      'https://example.com/goose/acp?token=test-secret',
    ]);
  });

  it('fails immediately when the ACP auth probe rejects the secret', async () => {
    const onEvent = vi.fn();
    const fetch = vi.fn(async (input: FetchInput) => {
      const url = fetchInputUrl(input);
      if (url === 'https://example.com/status') {
        return new Response(null, { status: 200 });
      }
      if (url === 'https://example.com/acp?token=wrong-secret') {
        return new Response(null, { status: 401 });
      }

      throw new Error(`Unexpected URL: ${url}`);
    });

    await expect(
      checkBackendStatus({
        baseUrl: 'https://example.com',
        serverSecret: 'wrong-secret',
        fetch,
        options: { onEvent },
      })
    ).resolves.toBe(false);

    expect(fetch).toHaveBeenCalledTimes(2);
    expect(onEvent).toHaveBeenCalledWith('healthcheck_auth_failed', { attempt: 1 });
  });

  it('aborts hanging ACP auth probes and reports the healthcheck timeout', async () => {
    vi.useFakeTimers();

    try {
      const onEvent = vi.fn();
      const acpSignals: FetchSignal[] = [];
      const fetch = vi.fn((input: FetchInput, init?: FetchInit): Promise<Response> => {
        const url = fetchInputUrl(input);
        if (url === 'https://example.com/status') {
          expectAbortSignal(init);
          return Promise.resolve(new Response(null, { status: 200 }));
        }
        if (url === 'https://example.com/acp?token=test-secret') {
          const signal = expectAbortSignal(init);
          acpSignals.push(signal);
          return new Promise<Response>((_, reject) => {
            signal.addEventListener('abort', () => reject(new Error('aborted')), { once: true });
          });
        }

        throw new Error(`Unexpected URL: ${url}`);
      });

      const result = checkBackendStatus({
        baseUrl: 'https://example.com',
        serverSecret: 'test-secret',
        fetch,
        options: { onEvent },
      });

      await vi.advanceTimersByTimeAsync(31000);

      await expect(result).resolves.toBe(false);
      expect(acpSignals.length).toBeGreaterThan(0);
      expect(acpSignals.every((signal) => signal.aborted)).toBe(true);
      expect(onEvent).toHaveBeenCalledWith('healthcheck_timeout', { timeoutMs: 30000 });
    } finally {
      vi.useRealTimers();
    }
  });
});
