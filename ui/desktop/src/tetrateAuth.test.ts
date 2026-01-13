import { describe, it, expect, vi, afterEach } from 'vitest';
import { createHash } from 'crypto';

vi.mock('electron', () => ({
  app: {
    getAppPath: () => '/tmp',
    getPath: () => '/tmp',
    isPackaged: false,
  },
  shell: {
    openExternal: vi.fn(),
  },
}));

vi.mock('./api', () => ({
  verifyTetrateSetup: vi.fn(),
}));

import { shell } from 'electron';
import type { Client } from './api/client';
import { verifyTetrateSetup } from './api';
import { __test, handleTetrateCallbackUrl, runTetrateAuthFlow } from './tetrateAuth';

describe('tetrateAuth', () => {
  afterEach(() => {
    __test.resetForTests();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('creates a PKCE verifier and matching challenge', () => {
    const { codeVerifier, codeChallenge } = __test.createPkcePair();
    const expectedChallenge = createHash('sha256').update(codeVerifier).digest('base64url');

    expect(codeVerifier.length).toBe(128);
    expect(codeVerifier).toMatch(/^[A-Za-z0-9_-]+$/);
    expect(codeChallenge).toBe(expectedChallenge);
  });

  it('builds an auth URL with required parameters', () => {
    const callbackUrl = 'goose://auth/tetrate?flow_id=flow&state=state';
    const authUrl = __test.buildTetrateAuthUrl(callbackUrl, 'challenge');
    const parsed = new URL(authUrl);

    expect(`${parsed.origin}${parsed.pathname}`).toBe('https://router.tetrate.ai/auth');
    expect(parsed.searchParams.get('callback')).toBe(callbackUrl);
    expect(parsed.searchParams.get('code_challenge')).toBe('challenge');
    expect(parsed.searchParams.get('code_challenge_method')).toBe('S256');
    expect(parsed.searchParams.get('client')).toBe('goose');
  });

  it('parses valid callback URLs and rejects invalid ones', () => {
    const url = 'goose://auth/tetrate?flow_id=flow&state=state&code=code';
    expect(__test.parseTetrateCallbackUrl(url)).toEqual({
      code: 'code',
      flowId: 'flow',
      state: 'state',
    });

    expect(__test.parseTetrateCallbackUrl('goose://auth/other?flow_id=flow')).toBeNull();
    expect(__test.parseTetrateCallbackUrl('https://example.com')).toBeNull();
  });

  it('resolves the waiting callback when a valid deep link arrives', async () => {
    const { flowId, authUrl } = __test.createTetrateAuthFlow();
    const callbackUrl = new URL(authUrl).searchParams.get('callback');
    expect(callbackUrl).toBeTruthy();

    const callbackWithCode = new URL(callbackUrl as string);
    callbackWithCode.searchParams.set('code', 'test-code');

    const waitPromise = __test.waitForTetrateCallback(flowId);
    expect(handleTetrateCallbackUrl(callbackWithCode.toString())).toBe(true);

    await expect(waitPromise).resolves.toBe(callbackWithCode.toString());
  });

  it('rejects the waiting callback on state mismatch', async () => {
    const { flowId, authUrl } = __test.createTetrateAuthFlow();
    const callbackUrl = new URL(authUrl).searchParams.get('callback');
    expect(callbackUrl).toBeTruthy();

    const callbackWithCode = new URL(callbackUrl as string);
    callbackWithCode.searchParams.set('code', 'test-code');

    const invalidUrl = new URL(callbackWithCode.toString());
    invalidUrl.searchParams.set('state', 'wrong');

    const waitPromise = __test.waitForTetrateCallback(flowId);
    expect(handleTetrateCallbackUrl(invalidUrl.toString())).toBe(true);

    await expect(waitPromise).rejects.toThrow('Authentication state mismatch');
  });

  it('rejects the waiting callback when the flow has timed out', async () => {
    vi.useFakeTimers();

    const { flowId, authUrl } = __test.createTetrateAuthFlow();
    const callbackUrl = new URL(authUrl).searchParams.get('callback');
    expect(callbackUrl).toBeTruthy();

    const callbackWithCode = new URL(callbackUrl as string);
    callbackWithCode.searchParams.set('code', 'test-code');

    const waitPromise = __test.waitForTetrateCallback(flowId);
    const ttlMs = __test.getTetrateAuthTtlMs();
    vi.setSystemTime(Date.now() + ttlMs + 1);

    expect(handleTetrateCallbackUrl(callbackWithCode.toString())).toBe(true);
    await expect(waitPromise).rejects.toThrow('Authentication timed out');
  });

  it('runs the full auth flow and verifies the code', async () => {
    const verifyMock = vi.mocked(verifyTetrateSetup);
    const request = new globalThis.Request('http://localhost/test');
    const response = new globalThis.Response();
    verifyMock.mockResolvedValue({
      data: { success: true, message: 'ok' },
      request,
      response,
    });

    const openExternalMock = vi.mocked(shell.openExternal);
    openExternalMock.mockImplementation(async (authUrl: string) => {
      const callbackUrl = new URL(authUrl).searchParams.get('callback');
      if (!callbackUrl) {
        throw new Error('Missing callback URL');
      }

      const callbackWithCode = new URL(callbackUrl);
      callbackWithCode.searchParams.set('code', 'test-code');
      handleTetrateCallbackUrl(callbackWithCode.toString());
    });

    const flowResult = await runTetrateAuthFlow({} as Client);

    expect(flowResult).toEqual({ success: true, message: 'ok' });
    expect(verifyMock).toHaveBeenCalledTimes(1);
    const call = verifyMock.mock.calls[0]?.[0];
    expect(call?.body?.code).toBe('test-code');
    const codeVerifier = call?.body?.code_verifier;
    expect(codeVerifier).toMatch(/^[A-Za-z0-9_-]+$/);
    expect(codeVerifier?.length).toBeGreaterThanOrEqual(43);
    expect(codeVerifier?.length).toBeLessThanOrEqual(128);
  });

  it('returns a failure response when verification fails', async () => {
    const verifyMock = vi.mocked(verifyTetrateSetup);
    verifyMock.mockRejectedValue(new Error('Verification failed'));

    const openExternalMock = vi.mocked(shell.openExternal);
    openExternalMock.mockImplementation(async (authUrl: string) => {
      const callbackUrl = new URL(authUrl).searchParams.get('callback');
      if (!callbackUrl) {
        throw new Error('Missing callback URL');
      }

      const callbackWithCode = new URL(callbackUrl);
      callbackWithCode.searchParams.set('code', 'test-code');
      handleTetrateCallbackUrl(callbackWithCode.toString());
    });

    const response = await runTetrateAuthFlow({} as Client);

    expect(response).toEqual({ success: false, message: 'Verification failed' });
  });
});
