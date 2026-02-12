/**
 * Integration test setup for testing the goosed binary via the TypeScript API client.
 *
 * This test suite spawns a real goosed process and issues requests via the
 * auto-generated API client from ui/desktop/src/api.
 */

import { spawn, type ChildProcess } from 'node:child_process';
import { createClient, createConfig } from '../../src/api/client';
import type { Client } from '../../src/api/client';
import {
  findAvailablePort,
  findGoosedBinaryPath,
  waitForServer,
  buildGoosedEnv,
} from '../../src/goosed';
import { expect } from 'vitest';

function stringifyResponse(response: Response) {
  const details = {
    ok: response.ok,
    status: response.status,
    statusText: response.statusText,
    url: response.url,
    headers: response.headers ? Object.fromEntries(response.headers) : undefined,
  };
  return JSON.stringify(details, null, 2);
}

expect.extend({
  toBeOkResponse(response) {
    const pass = response.ok === true;
    return {
      pass,
      message: () =>
        pass
          ? 'expected response not to be ok'
          : `expected response to be ok, got: ${stringifyResponse(response)}`,
    };
  },
});

const TEST_SECRET_KEY = 'test';

export interface GoosedTestContext {
  client: Client;
  baseUrl: string;
  port: number;
  secretKey: string;
  process: ChildProcess;
  cleanup: () => Promise<void>;
}

export async function startGoosed(pathOverride?: string): Promise<GoosedTestContext> {
  const port = await findAvailablePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const goosedPath = findGoosedBinaryPath({
    envOverride: process.env.GOOSED_BINARY,
  });

  const env = {
    ...buildGoosedEnv(port, TEST_SECRET_KEY),
    ...(pathOverride && { PATH: pathOverride }),
  };

  const goosedProcess = spawn(goosedPath, ['agent'], {
    env: { ...process.env, ...env },
    stdio: ['pipe', 'pipe', 'pipe'],
  });

  const stderrLines: string[] = [];

  goosedProcess.stdout?.on('data', (data: Buffer) => {
    if (process.env.DEBUG) {
      console.log(`[goosed:${port}:stdout]`, data.toString());
    }
  });

  goosedProcess.stderr?.on('data', (data: Buffer) => {
    const lines = data
      .toString()
      .split('\n')
      .filter((l) => l.trim());
    lines.forEach((line) => {
      stderrLines.push(line);
      if (process.env.DEBUG) {
        console.error(`[goosed:${port}:stderr]`, line);
      }
    });
  });

  goosedProcess.on('error', (err: Error) => {
    console.error(`Failed to start goosed on port ${port}:`, err);
  });

  try {
    await waitForServer(baseUrl, { errorLog: stderrLines });
  } catch (error) {
    goosedProcess.kill();
    console.error('Server stderr:', stderrLines.join('\n'));
    throw error;
  }

  const client = createClient(
    createConfig({
      baseUrl,
      headers: {
        'X-Secret-Key': TEST_SECRET_KEY,
      },
    })
  );

  const cleanup = async (): Promise<void> => {
    return new Promise((resolve) => {
      if (goosedProcess.killed) {
        resolve();
        return;
      }

      goosedProcess.on('close', () => {
        resolve();
      });

      goosedProcess.kill('SIGTERM');

      setTimeout(() => {
        if (!goosedProcess.killed) {
          goosedProcess.kill('SIGKILL');
        }
        resolve();
      }, 5000);
    });
  };

  return {
    client,
    baseUrl,
    port,
    secretKey: TEST_SECRET_KEY,
    process: goosedProcess,
    cleanup,
  };
}

let sharedContext: GoosedTestContext | null = null;

export async function getSharedGoosed(): Promise<GoosedTestContext> {
  if (!sharedContext) {
    sharedContext = await startGoosed();
  }
  return sharedContext;
}

export async function cleanupSharedGoosed(): Promise<void> {
  if (sharedContext) {
    await sharedContext.cleanup();
    sharedContext = null;
  }
}
