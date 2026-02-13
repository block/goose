import { spawn, ChildProcess } from 'child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createServer } from 'net';
import { Buffer } from 'node:buffer';
import { status } from './api';
import { Client, createClient, createConfig } from './api/client';

export interface Logger {
  info: (...args: unknown[]) => void;
  error: (...args: unknown[]) => void;
}

export const defaultLogger: Logger = {
  info: (...args) => console.log('[goosed]', ...args),
  error: (...args) => console.error('[goosed]', ...args),
};

export const findAvailablePort = (): Promise<number> => {
  return new Promise((resolve, _reject) => {
    const server = createServer();

    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address() as { port: number };
      server.close(() => {
        resolve(port);
      });
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

export const checkServerStatus = async (client: Client, errorLog: string[]): Promise<boolean> => {
  const timeout = 10000;
  const interval = 100;
  const maxAttempts = Math.ceil(timeout / interval);

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    if (errorLog.some(isFatalError)) {
      return false;
    }

    try {
      await status({ client, throwOnError: true });
      return true;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, interval));
    }
  }

  return false;
};

export const isFatalError = (line: string): boolean => {
  const fatalPatterns = [/panicked at/, /RUST_BACKTRACE/, /fatal error/i];
  return fatalPatterns.some((pattern) => pattern.test(line));
};

export const buildGoosedEnv = (
  port: number,
  secretKey: string,
  binaryPath?: string
): Record<string, string> => {
  // Environment variable naming follows the config crate convention:
  // - GOOSE_ prefix with _ separator for top-level fields (GOOSE_PORT, GOOSE_HOST)
  // - __ separator for nested fields (GOOSE_SERVER__SECRET_KEY)
  const homeDir = process.env.HOME || os.homedir();
  const env: Record<string, string> = {
    GOOSE_PORT: port.toString(),
    GOOSE_SERVER__SECRET_KEY: secretKey,
    HOME: homeDir,
  };

  // Windows-specific environment variables
  if (process.platform === 'win32') {
    env.USERPROFILE = homeDir;
    env.APPDATA = process.env.APPDATA || path.join(homeDir, 'AppData', 'Roaming');
    env.LOCALAPPDATA = process.env.LOCALAPPDATA || path.join(homeDir, 'AppData', 'Local');
  }

  // Add binary directory to PATH for any dependencies
  const pathKey = process.platform === 'win32' ? 'Path' : 'PATH';
  const currentPath = process.env[pathKey] || '';
  if (binaryPath) {
    env[pathKey] = `${path.dirname(binaryPath)}${path.delimiter}${currentPath}`;
  } else if (currentPath) {
    env[pathKey] = currentPath;
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
  serverSecret: string;
  env?: Record<string, string | undefined>;
  externalGoosed?: ExternalGoosedConfig;
  isPackaged?: boolean;
  resourcesPath?: string;
  logger?: Logger;
}

export interface GoosedResult {
  baseUrl: string;
  workingDir: string;
  process: ChildProcess | null;
  errorLog: string[];
  cleanup: () => Promise<void>;
  client: Client;
}

const goosedClientForUrlAndSecret = (url: string, secret: string): Client => {
  return createClient(
    createConfig({
      baseUrl: url,
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': secret,
      },
    })
  );
};

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
      cleanup: async () => {
        logger.info('Not killing external process that is managed externally');
      },
      client: goosedClientForUrlAndSecret(url, serverSecret),
    };
  }

  if (process.env.GOOSE_EXTERNAL_BACKEND) {
    const port = process.env.GOOSE_PORT || '3000';
    const url = `http://127.0.0.1:${port}`;
    logger.info(`Using external goosed backend from env at ${url}`);

    return {
      baseUrl: url,
      workingDir,
      process: null,
      errorLog,
      cleanup: async () => {
        logger.info('Not killing external process that is managed externally');
      },
      client: goosedClientForUrlAndSecret(url, serverSecret),
    };
  }

  const goosedPath = findGoosedBinaryPath({ isPackaged, resourcesPath });

  const port = await findAvailablePort();
  logger.info(`Starting goosed from: ${goosedPath} on port ${port} in dir ${workingDir}`);

  const baseUrl = `http://127.0.0.1:${port}`;

  const spawnEnv = {
    ...process.env,
    ...buildGoosedEnv(port, serverSecret, goosedPath),
  };

  for (const [key, value] of Object.entries(additionalEnv)) {
    if (value !== undefined) {
      spawnEnv[key] = value;
    }
  }

  const isWindows = process.platform === 'win32';
  const spawnOptions = {
    env: spawnEnv,
    cwd: workingDir,
    windowsHide: true,
    detached: isWindows,
    shell: false as const,
    stdio: ['ignore', 'pipe', 'pipe'] as ['ignore', 'pipe', 'pipe'],
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

  const cleanup = async (): Promise<void> => {
    return new Promise<void>((resolve) => {
      if (!goosedProcess || goosedProcess.killed) {
        resolve();
        return;
      }

      goosedProcess.on('close', () => {
        resolve();
      });

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

      setTimeout(() => {
        if (goosedProcess && !goosedProcess.killed && process.platform !== 'win32') {
          goosedProcess.kill('SIGKILL');
        }
        resolve();
      }, 5000);
    });
  };

  logger.info(`Goosed server successfully started on port ${port}`);

  return {
    baseUrl,
    workingDir,
    process: goosedProcess,
    errorLog,
    cleanup,
    client: goosedClientForUrlAndSecret(baseUrl, serverSecret),
  };
};
