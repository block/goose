/* eslint-disable no-undef */
/**
 * Integration tests for goose-acp-server custom request methods.
 *
 * Spawns a real goose-acp-server process and sends JSON-RPC requests
 * via HTTP+SSE to verify the custom _<method> handlers work end-to-end.
 *
 * Tests are split into two groups:
 * 1. Session-independent: work with just an ACP session (initialize)
 * 2. Session-dependent: require a goose session (session/new) which needs
 *    a configured provider - these are skipped in environments without one.
 */

import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { describe, it, expect, beforeAll, afterAll } from 'vitest';

const ACP_SERVER_BINARY = path.resolve(__dirname, '../../../../target/debug/goose-acp-server');

interface JsonRpcResponse {
  jsonrpc: string;
  id?: number;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
}

interface AcpTestContext {
  baseUrl: string;
  serverProcess: ChildProcess;
  tempDir: string;
  acpSessionId: string;
  gooseSessionId: string | null;
}

let ctx: AcpTestContext;

/**
 * Read an SSE stream from a fetch Response, collecting JSON-RPC messages.
 * Resolves once a message with the expected `id` is found (or times out).
 */
async function readSseResponse(
  response: Response,
  expectedId: number,
  timeoutMs = 10000
): Promise<{ messages: JsonRpcResponse[]; headers: Headers }> {
  const messages: JsonRpcResponse[] = [];
  const reader = response.body!.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  const timeout = new Promise<never>((_, reject) =>
    setTimeout(() => reject(new Error(`SSE timeout waiting for id=${expectedId}`)), timeoutMs)
  );

  const read = async (): Promise<{ messages: JsonRpcResponse[]; headers: Headers }> => {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });

      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        const trimmed = line.trim();
        if (trimmed.startsWith('data:')) {
          const data = trimmed.slice('data:'.length).trim();
          if (data) {
            try {
              const parsed = JSON.parse(data) as JsonRpcResponse;
              messages.push(parsed);
              if (parsed.id === expectedId) {
                reader.cancel().catch(() => {});
                return { messages, headers: response.headers };
              }
            } catch {
              // skip non-JSON data lines
            }
          }
        }
      }
    }
    return { messages, headers: response.headers };
  };

  return Promise.race([read(), timeout]);
}

/**
 * Send a JSON-RPC request and wait for the matching response via SSE.
 */
async function sendJsonRpc(
  baseUrl: string,
  method: string,
  params: Record<string, unknown>,
  id: number,
  acpSessionId: string
): Promise<JsonRpcResponse> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    Accept: 'application/json, text/event-stream',
    'Acp-Session-Id': acpSessionId,
  };

  const response = await fetch(`${baseUrl}/acp`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ jsonrpc: '2.0', method, params, id }),
  });

  const { messages } = await readSseResponse(response, id);
  const match = messages.find((m) => m.id === id);
  if (!match) {
    throw new Error(`No response for id=${id}, method=${method}. Got: ${JSON.stringify(messages)}`);
  }
  return match;
}

async function waitForServer(baseUrl: string, timeoutMs = 15000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const resp = await fetch(`${baseUrl}/health`);
      if (resp.ok) return;
    } catch {
      // not ready yet
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`ACP server did not start within ${timeoutMs}ms`);
}

async function initializeSession(baseUrl: string): Promise<string> {
  const response = await fetch(`${baseUrl}/acp`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
    },
    body: JSON.stringify({
      jsonrpc: '2.0',
      method: 'initialize',
      params: {
        protocolVersion: '2025-03-26',
        clientInfo: { name: 'integration-test', version: '0.1.0' },
        capabilities: {},
      },
      id: 1,
    }),
  });

  const acpSessionId = response.headers.get('acp-session-id');
  if (!acpSessionId) {
    throw new Error(`No Acp-Session-Id header in initialize response`);
  }

  // Consume the SSE stream
  await readSseResponse(response, 1);
  return acpSessionId;
}

beforeAll(async () => {
  if (!fs.existsSync(ACP_SERVER_BINARY)) {
    throw new Error(
      `Binary not found at ${ACP_SERVER_BINARY}. Run 'cargo build -p goose-acp' first.`
    );
  }

  const tempDir = await fs.promises.mkdtemp(path.join(os.tmpdir(), 'goose-acp-test-'));
  const port = 30000 + Math.floor(Math.random() * 10000);
  const baseUrl = `http://127.0.0.1:${port}`;

  const serverProcess = spawn(ACP_SERVER_BINARY, ['--host', '127.0.0.1', '--port', String(port)], {
    env: { ...process.env, GOOSE_PATH_ROOT: tempDir },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  serverProcess.stderr?.on('data', (data: Buffer) => {
    if (process.env.DEBUG) console.error('[acp]', data.toString().trim());
  });

  ctx = {
    baseUrl,
    serverProcess,
    tempDir,
    acpSessionId: '',
    gooseSessionId: null,
  };

  console.log(`[ACP TEST] Starting server on port ${port}...`);
  await waitForServer(baseUrl);
  console.log(`[ACP TEST] Server ready. Initializing ACP session...`);
  ctx.acpSessionId = await initializeSession(baseUrl);
  console.log(`[ACP TEST] ACP session: ${ctx.acpSessionId}`);
}, 30000);

afterAll(async () => {
  if (ctx?.serverProcess) {
    ctx.serverProcess.kill('SIGTERM');
    await new Promise<void>((resolve) => {
      ctx.serverProcess.on('close', () => resolve());
      setTimeout(() => {
        ctx.serverProcess.kill('SIGKILL');
        resolve();
      }, 5000);
    });
  }
  if (ctx?.tempDir) {
    await fs.promises.rm(ctx.tempDir, { recursive: true, force: true }).catch(() => {});
  }
});

describe('ACP custom requests - session independent', () => {
  it('session/list returns a sessions array', async () => {
    const response = await sendJsonRpc(
      ctx.baseUrl,
      '_goose/session/list',
      {},
      10,
      ctx.acpSessionId
    );

    expect(response.error).toBeUndefined();
    expect(response.result).toBeDefined();

    const result = response.result as { sessions: unknown[] };
    expect(Array.isArray(result.sessions)).toBe(true);
  });

  it('config/extensions returns extensions and warnings', async () => {
    const response = await sendJsonRpc(
      ctx.baseUrl,
      '_goose/config/extensions',
      {},
      11,
      ctx.acpSessionId
    );

    expect(response.error).toBeUndefined();
    const result = response.result as { extensions: unknown[]; warnings: unknown[] };
    expect(Array.isArray(result.extensions)).toBe(true);
    expect(Array.isArray(result.warnings)).toBe(true);
  });

  it('unknown _ method returns method_not_found error', async () => {
    const response = await sendJsonRpc(ctx.baseUrl, '_unknown/method', {}, 12, ctx.acpSessionId);

    expect(response.error).toBeDefined();
    expect(response.error!.code).toBe(-32601);
  });
});

describe('ACP custom requests - session dependent', () => {
  it('_session/get retrieves a session', async () => {
    if (!ctx.gooseSessionId) {
      console.log('Skipping: no goose session (provider not configured)');
      return;
    }

    const response = await sendJsonRpc(
      ctx.baseUrl,
      '_session/get',
      { session_id: ctx.gooseSessionId },
      20,
      ctx.acpSessionId
    );

    expect(response.error).toBeUndefined();
    const result = response.result as { session: { id: string } };
    expect(result.session.id).toBe(ctx.gooseSessionId);
  });

  it('_agent/tools returns tools for a session', async () => {
    if (!ctx.gooseSessionId) {
      console.log('Skipping: no goose session (provider not configured)');
      return;
    }

    const response = await sendJsonRpc(
      ctx.baseUrl,
      '_agent/tools',
      { session_id: ctx.gooseSessionId },
      21,
      ctx.acpSessionId
    );

    expect(response.error).toBeUndefined();
    const result = response.result as { tools: unknown[] };
    expect(Array.isArray(result.tools)).toBe(true);
  });

  it('_session/delete removes a session', async () => {
    if (!ctx.gooseSessionId) {
      console.log('Skipping: no goose session (provider not configured)');
      return;
    }

    const deleteResp = await sendJsonRpc(
      ctx.baseUrl,
      '_session/delete',
      { session_id: ctx.gooseSessionId },
      22,
      ctx.acpSessionId
    );
    expect(deleteResp.error).toBeUndefined();

    // Verify it's gone
    const getResp = await sendJsonRpc(
      ctx.baseUrl,
      '_session/get',
      { session_id: ctx.gooseSessionId },
      23,
      ctx.acpSessionId
    );
    expect(getResp.error).toBeDefined();
  });
});
