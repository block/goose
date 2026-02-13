import { shell } from 'electron';
import * as crypto from 'crypto';
import type { Client } from './api/client';
import { verifyTetrateSetup } from './api';
import log from './utils/logger';

type TetrateAuthFlow = {
  codeVerifier: string;
  state: string;
  expiresAt: number;
  callbackUrl?: string;
  resolve?: (url: string) => void;
  reject?: (error: Error) => void;
  timeoutId?: ReturnType<typeof setTimeout>;
};

export type TetrateSetupResponse = {
  success: boolean;
  message: string;
};

type TetrateCallbackData = {
  code: string;
  flowId: string;
  state: string;
};

type TetrateCallbackMatch = {
  flowId: string;
  state: string;
  code?: string;
  error?: string;
};

const TETRATE_AUTH_URL = 'https://router.tetrate.ai/auth';
const TETRATE_AUTH_TTL_MS = 10 * 60 * 1000;
const TETRATE_AUTH_CALLBACK_SCHEME = 'goose';

const tetrateAuthFlows = new Map<string, TetrateAuthFlow>();

function createPkcePair(): { codeVerifier: string; codeChallenge: string } {
  const codeVerifier = crypto.randomBytes(96).toString('base64url');
  const codeChallenge = crypto.createHash('sha256').update(codeVerifier).digest('base64url');
  return { codeVerifier, codeChallenge };
}

function buildTetrateCallbackUrl(flowId: string, state: string): string {
  const url = new URL(`${TETRATE_AUTH_CALLBACK_SCHEME}://auth/tetrate`);
  url.searchParams.set('flow_id', flowId);
  url.searchParams.set('state', state);
  return url.toString();
}

function buildTetrateAuthUrl(callbackUrl: string, codeChallenge: string): string {
  const url = new URL(TETRATE_AUTH_URL);
  url.searchParams.set('callback', callbackUrl);
  url.searchParams.set('code_challenge', codeChallenge);
  url.searchParams.set('code_challenge_method', 'S256');
  url.searchParams.set('client', 'goose');
  return url.toString();
}

function parseTetrateCallbackUrl(callbackUrl: string): TetrateCallbackData | null {
  let parsedUrl: URL;
  try {
    parsedUrl = new URL(callbackUrl);
  } catch {
    return null;
  }

  const normalizedPath = parsedUrl.pathname.replace(/\/$/, '');
  if (
    parsedUrl.protocol !== `${TETRATE_AUTH_CALLBACK_SCHEME}:` ||
    parsedUrl.hostname !== 'auth' ||
    normalizedPath !== '/tetrate'
  ) {
    return null;
  }

  const code = parsedUrl.searchParams.get('code');
  const flowId = parsedUrl.searchParams.get('flow_id');
  const state = parsedUrl.searchParams.get('state');

  if (!code || !flowId || !state) {
    return null;
  }

  return { code, flowId, state };
}

function matchTetrateCallbackUrl(callbackUrl: string): TetrateCallbackMatch | null {
  let parsedUrl: URL;
  try {
    parsedUrl = new URL(callbackUrl);
  } catch {
    return null;
  }

  const normalizedPath = parsedUrl.pathname.replace(/\/$/, '');
  if (
    parsedUrl.protocol !== `${TETRATE_AUTH_CALLBACK_SCHEME}:` ||
    parsedUrl.hostname !== 'auth' ||
    normalizedPath !== '/tetrate'
  ) {
    return null;
  }

  const flowId = parsedUrl.searchParams.get('flow_id');
  const state = parsedUrl.searchParams.get('state');

  if (!flowId || !state) {
    return null;
  }

  const result: TetrateCallbackMatch = { flowId, state };
  const code = parsedUrl.searchParams.get('code');
  const error = parsedUrl.searchParams.get('error');
  if (code) result.code = code;
  if (error) result.error = error;

  return result;
}

function cleanupTetrateAuthFlow(flowId: string): void {
  const flow = tetrateAuthFlows.get(flowId);
  if (!flow) {
    return;
  }

  if (flow.timeoutId) {
    clearTimeout(flow.timeoutId);
  }

  tetrateAuthFlows.delete(flowId);
}

function expireTetrateAuthFlow(flowId: string, message: string): void {
  const flow = tetrateAuthFlows.get(flowId);
  if (!flow) {
    return;
  }

  log.warn('Tetrate auth flow expired:', { flowId, reason: message });
  flow.reject?.(new Error(message));
  cleanupTetrateAuthFlow(flowId);
}

function createTetrateAuthFlow(): { flowId: string; authUrl: string } {
  const flowId = crypto.randomUUID();
  const state = crypto.randomBytes(16).toString('base64url');
  const { codeVerifier, codeChallenge } = createPkcePair();
  const callbackUrl = buildTetrateCallbackUrl(flowId, state);
  const authUrl = buildTetrateAuthUrl(callbackUrl, codeChallenge);
  const expiresAt = Date.now() + TETRATE_AUTH_TTL_MS;
  const timeoutId = setTimeout(() => {
    expireTetrateAuthFlow(flowId, 'Authentication timed out');
  }, TETRATE_AUTH_TTL_MS);

  tetrateAuthFlows.set(flowId, {
    codeVerifier,
    state,
    expiresAt,
    timeoutId,
  });

  return { flowId, authUrl };
}

export function handleTetrateCallbackUrl(
  url: string,
  clearPendingDeepLink?: () => void
): boolean {
  const match = matchTetrateCallbackUrl(url);
  if (!match) {
    return false;
  }

  clearPendingDeepLink?.();

  const flow = tetrateAuthFlows.get(match.flowId);
  if (!flow) {
    log.info('Tetrate auth callback without active flow:', { flowId: match.flowId });
    return true;
  }

  if (flow.state !== match.state) {
    expireTetrateAuthFlow(match.flowId, 'Authentication state mismatch');
    return true;
  }

  if (Date.now() > flow.expiresAt) {
    expireTetrateAuthFlow(match.flowId, 'Authentication timed out');
    return true;
  }

  if (match.error) {
    expireTetrateAuthFlow(match.flowId, `Authentication denied: ${match.error}`);
    return true;
  }

  if (!match.code) {
    expireTetrateAuthFlow(match.flowId, 'Authentication failed');
    return true;
  }

  flow.callbackUrl = url;
  flow.resolve?.(url);
  return true;
}

function waitForTetrateCallback(flowId: string): Promise<string> {
  const flow = tetrateAuthFlows.get(flowId);
  if (!flow) {
    return Promise.reject(new Error('Authentication expired'));
  }

  if (flow.callbackUrl) {
    return Promise.resolve(flow.callbackUrl);
  }

  if (Date.now() > flow.expiresAt) {
    expireTetrateAuthFlow(flowId, 'Authentication timed out');
    return Promise.reject(new Error('Authentication timed out'));
  }

  return new Promise((resolve, reject) => {
    flow.resolve = resolve;
    flow.reject = reject;
  });
}

async function startTetrateAuthSession(flowId: string, authUrl: string): Promise<string> {
  await shell.openExternal(authUrl);
  return waitForTetrateCallback(flowId);
}

function getTetrateAuthErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }
  if (error && typeof error === 'object') {
    const message = (error as { message?: unknown }).message;
    if (typeof message === 'string' && message) {
      return message;
    }
  }
  if (typeof error === 'string' && error) {
    return error;
  }
  return 'Authentication failed';
}

export async function runTetrateAuthFlow(client: Client): Promise<TetrateSetupResponse> {
  const { flowId, authUrl } = createTetrateAuthFlow();

  try {
    const callbackUrl = await startTetrateAuthSession(flowId, authUrl);
    const callback = parseTetrateCallbackUrl(callbackUrl);

    if (!callback) {
      throw new Error('Invalid authentication response');
    }

    const flow = tetrateAuthFlows.get(callback.flowId);
    if (!flow) {
      throw new Error('Authentication expired');
    }

    if (flow.state !== callback.state) {
      throw new Error('Authentication state mismatch');
    }

    if (Date.now() > flow.expiresAt) {
      throw new Error('Authentication timed out');
    }

    const codeVerifier = flow.codeVerifier;
    cleanupTetrateAuthFlow(callback.flowId);

    const response = await verifyTetrateSetup({
      body: { code: callback.code, code_verifier: codeVerifier },
      throwOnError: true,
      client,
    });

    return response.data ?? {
      success: false,
      message: 'Setup failed',
    };
  } catch (error) {
    log.warn('Tetrate auth failed:', getTetrateAuthErrorMessage(error));
    cleanupTetrateAuthFlow(flowId);
    return {
      success: false,
      message: getTetrateAuthErrorMessage(error),
    };
  }
}

export const __test = {
  buildTetrateAuthUrl,
  createPkcePair,
  createTetrateAuthFlow,
  getTetrateAuthTtlMs: () => TETRATE_AUTH_TTL_MS,
  matchTetrateCallbackUrl,
  parseTetrateCallbackUrl,
  resetForTests: () => {
    for (const flow of tetrateAuthFlows.values()) {
      if (flow.timeoutId) {
        clearTimeout(flow.timeoutId);
      }
    }
    tetrateAuthFlows.clear();
  },
  waitForTetrateCallback,
};
