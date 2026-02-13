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
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

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
  const port = await findAvailablePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const pathFromEnv = process.env.GOOSED_BINARY;
  const goosedPath = pathFromEnv ?? findGoosedBinaryPath();

  // mk temp dir for app root
  const tempDir = await fs.promises.mkdtemp(path.join(os.tmpdir(), 'goose-app-root-'));

  if (configYaml) {
    await fs.promises.mkdir(path.join(tempDir, 'config'), { recursive: true });
    await fs.promises.writeFile(path.join(tempDir, 'config', 'config.yaml'), configYaml);
  }

  const env = {
    ...buildGoosedEnv(port, TEST_SECRET_KEY),
    ...(pathOverride && { PATH: pathOverride }),
    GOOSE_PATH_ROOT: tempDir,
  };

  console.log('spawning goosed with env:', env);
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
    const logDirs = await fs.promises.readdir(path.join(tempDir, 'state', 'logs', 'server'));
    for (const logDir of logDirs) {
      const logFiles = await fs.promises.readdir(
        path.join(tempDir, 'state', 'logs', 'server', logDir)
      );
      for (const logFile of logFiles) {
        const logPath = path.join(tempDir, 'state', 'logs', 'server', logDir, logFile);
        const logContent = await fs.promises.readFile(logPath, 'utf8');
        console.log(logContent);
      }
    }

    return new Promise<void>((resolve) => {
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
    }).then(async () => {
      await fs.promises.rm(tempDir, { recursive: true, force: true });
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
