/**
 * Integration test setup for testing the goosed binary via the TypeScript API client.
 *
 * This test suite spawns a real goosed process and issues requests via the
 * auto-generated API client from ui/desktop/src/api.
 */

import { spawn, type ChildProcess } from 'node:child_process';
import * as path from 'node:path';
import * as fs from 'node:fs';
import { createClient, createConfig } from '../../src/api/client';
import type { Client } from '../../src/api/client';

// Secret key for authenticating with goosed (must match GOOSE_SERVER__SECRET_KEY env var)
const TEST_SECRET_KEY = 'test';

export interface GoosedTestContext {
  client: Client;
  baseUrl: string;
  port: number;
  secretKey: string;
  process: ChildProcess;
  cleanup: () => Promise<void>;
}

let portCounter = 13100;

function getNextPort(): number {
  return portCounter++;
}

function findGoosedBinary(): string {
  const goosedBinaryEnv = process.env.GOOSED_BINARY;
  if (goosedBinaryEnv) {
    return goosedBinaryEnv;
  }

  const possiblePaths = [
    path.join(process.cwd(), 'src', 'bin', 'goosed'),
    path.join(process.cwd(), 'bin', 'goosed'),
    path.join(process.cwd(), '..', '..', 'target', 'debug', 'goosed'),
    path.join(process.cwd(), '..', '..', 'target', 'release', 'goosed'),
  ];

  for (const binPath of possiblePaths) {
    const resolvedPath = path.resolve(binPath);
    if (fs.existsSync(resolvedPath)) {
      return resolvedPath;
    }
  }

  throw new Error(
    `Could not find goosed binary in any of the expected locations: ${possiblePaths.join(', ')}`
  );
}

async function waitForServer(baseUrl: string, timeoutMs = 10000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const response = await fetch(`${baseUrl}/status`);
      if (response.ok) {
        return;
      }
    } catch {
      // Server not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Server at ${baseUrl} did not become ready within ${timeoutMs}ms`);
}

export async function startGoosed(): Promise<GoosedTestContext> {
  const port = getNextPort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const goosedPath = findGoosedBinary();

  const goosedProcess = spawn(goosedPath, ['agent'], {
    env: {
      ...process.env,
      GOOSE_PORT: port.toString(),
      GOOSE_SERVER__SECRET_KEY: TEST_SECRET_KEY,
    },
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
    await waitForServer(baseUrl);
  } catch (error) {
    goosedProcess.kill();
    console.error('Server stderr:', stderrLines.join('\n'));
    throw error;
  }

  // Create client with authentication header
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

      // Force kill after timeout
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

// Global test context for shared server instance
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
