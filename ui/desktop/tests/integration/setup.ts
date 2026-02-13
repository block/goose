/**
 * Integration test setup for testing the goosed binary via the TypeScript API client.
 *
 * This test suite spawns a real goosed process and issues requests via the
 * auto-generated API client.
 */

import type { ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createClient, createConfig } from '../../src/api/client';
import type { Client } from '../../src/api/client';
import { startGoosed as startGoosedBase, waitForServer, type Logger } from '../../src/goosed';
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

export async function startGoosed({
  pathOverride,
  configYaml,
}: {
  pathOverride?: string;
  configYaml?: string;
}): Promise<GoosedTestContext> {
  const tempDir = await fs.promises.mkdtemp(path.join(os.tmpdir(), 'goose-app-root-'));

  if (configYaml) {
    await fs.promises.mkdir(path.join(tempDir, 'config'), { recursive: true });
    await fs.promises.writeFile(path.join(tempDir, 'config', 'config.yaml'), configYaml);
  }

  const testLogger: Logger = {
    info: (...args) => {
      if (process.env.DEBUG) {
        console.log('[goosed]', ...args);
      }
    },
    error: (...args) => console.error('[goosed]', ...args),
  };

  const additionalEnv: Record<string, string> = {
    GOOSE_PATH_ROOT: tempDir,
  };

  if (pathOverride) {
    additionalEnv.PATH = pathOverride;
  }

  const result = await startGoosedBase({
    serverSecret: TEST_SECRET_KEY,
    env: additionalEnv,
    logger: testLogger,
  });

  if (!result.process) {
    throw new Error('Expected goosed process to be started, but got external backend');
  }

  const port = parseInt(new URL(result.baseUrl).port, 10);

  try {
    const serverReady = await waitForServer(result.baseUrl, {
      logger: testLogger,
    });
    if (!serverReady) {
      result.cleanup();
      console.error('Server stderr:', result.errorLog.join('\n'));
  const serverReady = await waitForServer(baseUrl);
  if (!serverReady) {
    goosedProcess.kill();
    console.error('Server stderr:', stderrLines.join('\n'));
    throw new Error(`Failed to start goosed on port ${port}: server did not become ready`);
  }

  const client = createClient(
    createConfig({
      baseUrl: result.baseUrl,
      headers: {
        'X-Secret-Key': TEST_SECRET_KEY,
      },
    })
  );

  const cleanup = async (): Promise<void> => {
    try {
      const logsPath = path.join(tempDir, 'state', 'logs', 'server');
      if (fs.existsSync(logsPath)) {
        const logDirs = await fs.promises.readdir(logsPath);
        for (const logDir of logDirs) {
          const logFiles = await fs.promises.readdir(path.join(logsPath, logDir));
          for (const logFile of logFiles) {
            const logPath = path.join(logsPath, logDir, logFile);
            const logContent = await fs.promises.readFile(logPath, 'utf8');
            console.log(logContent);
          }
        }
      }
    } catch {
      // Logs may not exist, that's okay
    }

    return new Promise<void>((resolve) => {
      if (!result.process || result.process.killed) {
        resolve();
        return;
      }

      result.process.on('close', () => {
        resolve();
      });

      result.process.kill('SIGTERM');

      setTimeout(() => {
        if (result.process && !result.process.killed) {
          result.process.kill('SIGKILL');
        }
        resolve();
      }, 5000);
    }).then(async () => {
      await fs.promises.rm(tempDir, { recursive: true, force: true });
    });
  };

  return {
    client,
    baseUrl: result.baseUrl,
    port,
    secretKey: TEST_SECRET_KEY,
    process: result.process,
    cleanup,
  };
}
