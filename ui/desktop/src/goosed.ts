/**
 * Goosed process management utilities.
 * These utilities are designed to work in both the main Electron process
 * and in Node.js test environments.
 */

import { spawn, ChildProcess } from 'child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createServer } from 'net';
import { Buffer } from 'node:buffer';

export interface Logger {
  info: (...args: unknown[]) => void;
  error: (...args: unknown[]) => void;
}

export const defaultLogger: Logger = {
  info: (...args) => console.log('[goosed]', ...args),
  error: (...args) => console.error('[goosed]', ...args),
};

export const findAvailablePort = (): Promise<number> => {
  return new Promise((resolve, reject) => {
    const server = createServer();

    server.on('error', reject);

    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      if (address && typeof address === 'object') {
        const { port } = address;
        server.close(() => resolve(port));
      } else {
        server.close();
        reject(new Error('Failed to get port from server address'));
      }
    });
  });
};

export interface FindBinaryOptions {
  isPackaged?: boolean;
  resourcesPath?: string;
}

export const findGoosedBinaryPath = (options: FindBinaryOptions = {}): string => {
  const { isPackaged = false, resourcesPath } = options;
  const binaryName = process.platform === 'win32' ? 'goosed.exe' : 'goosed';

  const possiblePaths: string[] = [];

  // Packaged app paths
  if (isPackaged && resourcesPath) {
    possiblePaths.push(path.join(resourcesPath, 'bin', binaryName));
    possiblePaths.push(path.join(resourcesPath, binaryName));
  }

  // Development paths
  possiblePaths.push(
    path.join(process.cwd(), 'src', 'bin', binaryName),
    path.join(process.cwd(), '..', '..', 'target', 'release', binaryName),
    path.join(process.cwd(), '..', '..', 'target', 'debug', binaryName)
  );

  for (const p of possiblePaths) {
    try {
      if (fs.existsSync(p) && fs.statSync(p).isFile()) {
        return p;
      }
    } catch {
      // continue
    }
  }

  throw new Error(
    `Goosed binary not found in any of the possible paths: ${possiblePaths.join(', ')}`
  );
};

export interface WaitForServerOptions {
  timeout?: number;
  interval?: number;
  logger?: Logger;
}

export const waitForServer = async (
  baseUrl: string,
  options: WaitForServerOptions = {}
): Promise<boolean> => {
  const { timeout = 10000, interval = 100, logger = defaultLogger } = options;
  const maxAttempts = Math.ceil(timeout / interval);

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const response = await fetch(`${baseUrl}/status`);
      if (response.ok) {
        return true;
      }
    } catch {
      // Server not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, interval));
  }

  logger.error(`Server failed to respond after ${timeout / 1000} seconds`);
  return false;
};

export const isFatalError = (line: string): boolean => {
  const fatalPatterns = [/panicked at/, /RUST_BACKTRACE/, /fatal error/i, /^error\[E\d+\]/];
  return fatalPatterns.some((pattern) => pattern.test(line));
};

export const buildGoosedEnv = (port: number, secretKey: string): Record<string, string> => {
  // Note: Only returns the goosed-specific env vars. Caller should spread with process.env.
  // Environment variable naming follows the config crate convention:
  // - GOOSE_ prefix with _ separator for top-level fields (GOOSE_PORT, GOOSE_HOST)
  // - __ separator for nested fields (GOOSE_SERVER__SECRET_KEY)
  const env: Record<string, string> = {
    GOOSE_PORT: port.toString(),
    GOOSE_SERVER__SECRET_KEY: secretKey,
    HOME: process.env.HOME || os.homedir(),
  };

  // Handle PATH for different platforms
  const pathKey = process.platform === 'win32' ? 'Path' : 'PATH';
  if (process.env[pathKey]) {
    env[pathKey] = process.env[pathKey];
  }

  return env;
};

// Configuration for external goosed server
export interface ExternalGoosedConfig {
  enabled: boolean;
  url?: string;
  secret?: string;
}

export interface StartGoosedOptions {
  dir?: string;
  isPackaged?: boolean;
  resourcesPath?: string;
  serverSecret: string;
  env?: Record<string, string | undefined>;
  externalGoosed?: ExternalGoosedConfig;
  logger?: Logger;
}

export interface GoosedResult {
  baseUrl: string;
  workingDir: string;
  process: ChildProcess | null;
  errorLog: string[];
  cleanup: () => void;
}

export const startGoosed = async (options: StartGoosedOptions): Promise<GoosedResult> => {
  const {
    dir,
    isPackaged = false,
    resourcesPath,
    serverSecret,
    env: additionalEnv = {},
    externalGoosed,
    logger = defaultLogger,
  } = options;

  const errorLog: string[] = [];
  const workingDir = dir || os.homedir();

  if (externalGoosed?.enabled && externalGoosed.url) {
    const url = externalGoosed.url.replace(/\/$/, '');
    logger.info(`Using external goosed backend at ${url}`);

    return {
      baseUrl: url,
      workingDir,
      process: null,
      errorLog,
      cleanup: () => {
        logger.info('Not killing external process that is managed externally');
      },
    };
  }

  const externalBackendUrl = process.env.GOOSE_EXTERNAL_BACKEND;
  if (externalBackendUrl) {
    const url = externalBackendUrl.replace(/\/$/, '');
    logger.info(`Using external goosed backend from env at ${url}`);

    return {
      baseUrl: url,
      workingDir,
      process: null,
      errorLog,
      cleanup: () => {
        logger.info('Not killing external process that is managed externally');
      },
    };
  }

  const goosedPath = findGoosedBinaryPath({ isPackaged, resourcesPath });
  if (!goosedPath) {
    throw new Error('Could not find goosed binary');
  }

  const port = await findAvailablePort();
  logger.info(`Starting goosed from: ${goosedPath} on port ${port} in dir ${workingDir}`);

  const baseUrl = `http://127.0.0.1:${port}`;

  const spawnEnv = buildGoosedEnv(port, serverSecret);

  for (const [key, value] of Object.entries(additionalEnv)) {
    if (value !== undefined) {
      spawnEnv[key] = value;
    }
  }

  const spawnOptions = {
    env: spawnEnv,
    cwd: workingDir,
    windowsHide: true,
  };

  const safeSpawnOptions = {
    ...spawnOptions,
    env: Object.fromEntries(
      Object.entries(spawnOptions.env).map(([k, v]) =>
        k.toLowerCase().includes('secret') || k.toLowerCase().includes('key')
          ? [k, '[REDACTED]']
          : [k, v]
      )
    ),
  };
  logger.info('Spawn options:', JSON.stringify(safeSpawnOptions, null, 2));

  const goosedProcess = spawn(goosedPath, ['agent'], spawnOptions);

  goosedProcess.stdout?.on('data', (data: Buffer) => {
    logger.info(`goosed stdout for port ${port} and dir ${workingDir}: ${data.toString()}`);
  });

  goosedProcess.stderr?.on('data', (data: Buffer) => {
    const lines = data.toString().split('\n');
    for (const line of lines) {
      if (line.trim()) {
        errorLog.push(line);
        if (isFatalError(line)) {
          logger.error(`goosed stderr for port ${port} and dir ${workingDir}: ${line}`);
        }
      }
    }
  });

  goosedProcess.on('exit', (code) => {
    logger.info(`goosed process exited with code ${code} for port ${port} and dir ${workingDir}`);
  });

  goosedProcess.on('error', (err) => {
    logger.error(`Failed to start goosed on port ${port} and dir ${workingDir}`, err);
    errorLog.push(err.message);
  });

  const cleanup = () => {
    if (goosedProcess && !goosedProcess.killed) {
      logger.info('Terminating goosed server');
      try {
        if (process.platform === 'win32') {
          spawn('taskkill', ['/pid', goosedProcess.pid!.toString(), '/f', '/t']);
        } else {
          goosedProcess.kill('SIGTERM');
        }
      } catch (error) {
        logger.error('Error while terminating goosed process:', error);
      }
    }
  };

  logger.info(`Goosed server successfully started on port ${port}`);

  return {
    baseUrl,
    workingDir,
    process: goosedProcess,
    errorLog,
    cleanup,
  };
};
