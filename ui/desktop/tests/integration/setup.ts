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
import type { Client } from '../../src/api/client';
import { startGoosed as startGoosedBase, checkServerStatus, type Logger } from '../../src/goosed';
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
  secretKey: string;
  process: ChildProcess | null;
  cleanup: () => Promise<void>;
}

export async function setupGoosed({
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

  // Accept self-signed TLS certs from the local goosed server.
  // In Electron this is handled by setCertificateVerifyProc, but integration
  // tests run in plain Node.js where fetch rejects self-signed certs.
  process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

  const additionalEnv: Record<string, string> = {
    GOOSE_PATH_ROOT: tempDir,
  };

  if (pathOverride) {
    additionalEnv.PATH = pathOverride;
  }

  const {
    baseUrl,
    process: goosedProcess,
    client,
    cleanup: baseCleanup,
    errorLog,
  } = await startGoosedBase({
    serverSecret: TEST_SECRET_KEY,
    env: additionalEnv,
    logger: testLogger,
  });

  if (!goosedProcess) {
    throw new Error('Expected goosed process to be started, but got external backend');
  }

  const cleanup = async (): Promise<void> => {
    // dump server logs to test logs, visible if there are test failures
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
      // Logs may not exist
    }

    await baseCleanup();
    await fs.promises.rm(tempDir, { recursive: true, force: true });
  };

  const serverReady = await checkServerStatus(client, errorLog);
  if (!serverReady) {
    await cleanup();
    console.error('Server stderr:', errorLog.join('\n'));
    throw new Error('Failed to start goosed');
  }

  return {
    client,
    baseUrl,
    secretKey: TEST_SECRET_KEY,
    process: goosedProcess,
    cleanup,
  };
}
