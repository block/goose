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

type TetrateCallbackMatch = {
  flowId: string;
  state: string;
  code?: string;
  error?: string;
  errorDescription?: string;
};

const TETRATE_AUTH_URL = 'https://router.tetrate.ai/auth';
const TETRATE_AUTH_TTL_MS = 2 * 60 * 1000;
const TETRATE_AUTH_CALLBACK_SCHEME = 'goose';

const tetrateAuthFlows = new Map<string, TetrateAuthFlow>();
const completedTetrateAuthFlowErrors = new Map<string, string>();
let activeTetrateFlowId: string | null = null;

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
  const errorDescription = parsedUrl.searchParams.get('error_description');
  if (code) result.code = code;
  if (error) result.error = error;
  if (errorDescription) result.errorDescription = errorDescription;

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
  if (!flow.reject) {
    completedTetrateAuthFlowErrors.set(flowId, message);
  }
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
    const errorMessage = match.errorDescription || match.error;
    expireTetrateAuthFlow(match.flowId, `Authentication denied: ${errorMessage}`);
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
    const completedFlowError = completedTetrateAuthFlowErrors.get(flowId);
    if (completedFlowError) {
      completedTetrateAuthFlowErrors.delete(flowId);
      return Promise.reject(new Error(completedFlowError));
    }
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

export function cancelTetrateAuthFlow(message = 'Authentication canceled by user'): boolean {
  if (!activeTetrateFlowId) {
    return false;
  }
  const flowId = activeTetrateFlowId;
  activeTetrateFlowId = null;
  expireTetrateAuthFlow(flowId, message);
  return true;
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
  if (activeTetrateFlowId) {
    return {
      success: false,
      message: 'Authentication already in progress',
    };
  }

  const { flowId, authUrl } = createTetrateAuthFlow();
  activeTetrateFlowId = flowId;

  try {
    const callbackUrl = await startTetrateAuthSession(flowId, authUrl);
    const match = matchTetrateCallbackUrl(callbackUrl);

    if (!match?.code) {
      throw new Error('Invalid authentication response');
    }

    const flow = tetrateAuthFlows.get(flowId);
    if (!flow) {
      throw new Error('Authentication expired');
    }

    const codeVerifier = flow.codeVerifier;
    cleanupTetrateAuthFlow(flowId);

    const response = await verifyTetrateSetup({
      body: { code: match.code, code_verifier: codeVerifier },
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
  } finally {
    if (activeTetrateFlowId === flowId) {
      activeTetrateFlowId = null;
    }
  }
}

export const __test = {
  buildTetrateAuthUrl,
  createPkcePair,
  createTetrateAuthFlow,
  getTetrateAuthTtlMs: () => TETRATE_AUTH_TTL_MS,
  matchTetrateCallbackUrl,
  resetForTests: () => {
    for (const flow of tetrateAuthFlows.values()) {
      if (flow.timeoutId) {
        clearTimeout(flow.timeoutId);
      }
    }
    tetrateAuthFlows.clear();
    completedTetrateAuthFlowErrors.clear();
    activeTetrateFlowId = null;
  },
  waitForTetrateCallback,
};
