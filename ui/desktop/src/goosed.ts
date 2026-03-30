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
  info: (...args) => console.log('[goose-server]', ...args),
  error: (...args) => console.error('[goose-server]', ...args),
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

export const findGooseBinaryPath = (options: FindBinaryOptions = {}): string => {
  const pathFromEnv = process.env.GOOSE_BINARY ?? process.env.GOOSED_BINARY;
  if (pathFromEnv) {
    if (fs.existsSync(pathFromEnv) && fs.statSync(pathFromEnv).isFile()) {
      return path.resolve(pathFromEnv);
    } else {
      throw new Error(`Invalid GOOSE_BINARY path: ${pathFromEnv} (pwd is ${process.cwd()})`);
    }
  }
  const { isPackaged = false, resourcesPath } = options;
  const binaryName = process.platform === 'win32' ? 'goose.exe' : 'goose';

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
    `Goose binary not found in any of the possible paths: ${possiblePaths.join(', ')}`
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

export const buildServerEnv = (
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

// Configuration for external goose server
export interface ExternalGooseServerConfig {
  enabled: boolean;
  url?: string;
  secret?: string;
}

export interface StartGooseServerOptions {
  dir?: string;
  serverSecret: string;
  env?: Record<string, string | undefined>;
  externalGoosed?: ExternalGooseServerConfig;
  isPackaged?: boolean;
  resourcesPath?: string;
  logger?: Logger;
}

export interface GooseServerResult {
  baseUrl: string;
  workingDir: string;
  process: ChildProcess | null;
  errorLog: string[];
  stopErrorLogCollection: () => void;
  cleanup: () => Promise<void>;
  client: Client;
  certFingerprint: string | null;
}

const gooseClientForUrlAndSecret = (url: string, secret: string): Client => {
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

export const startGooseServer = async (
  options: StartGooseServerOptions
): Promise<GooseServerResult> => {
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
    logger.info(`Using external goose backend at ${url}`);

    return {
      baseUrl: url,
      workingDir,
      process: null,
      errorLog,
      stopErrorLogCollection: () => {},
      cleanup: async () => {
        logger.info('Not killing external process that is managed externally');
      },
      client: gooseClientForUrlAndSecret(url, serverSecret),
      certFingerprint: null,
    };
  }

  if (process.env.GOOSE_EXTERNAL_BACKEND) {
    const port = process.env.GOOSE_PORT || '3000';
    const url = `https://127.0.0.1:${port}`;
    logger.info(`Using external goose backend from env at ${url}`);

    return {
      baseUrl: url,
      workingDir,
      process: null,
      errorLog,
      stopErrorLogCollection: () => {},
      cleanup: async () => {
        logger.info('Not killing external process that is managed externally');
      },
      client: gooseClientForUrlAndSecret(url, serverSecret),
      certFingerprint: null,
    };
  }

  const goosePath = findGooseBinaryPath({ isPackaged, resourcesPath });

  const port = await findAvailablePort();
  logger.info(`Starting goose server from: ${goosePath} on port ${port} in dir ${workingDir}`);

  const baseUrl = `https://127.0.0.1:${port}`;

  const spawnEnv: Record<string, string | undefined> = {
    ...process.env,
    ...buildServerEnv(port, serverSecret, goosePath),
  };

  for (const [key, value] of Object.entries(additionalEnv)) {
    if (value !== undefined) {
      spawnEnv[key] = value;
    }
  }

  const spawnCommand = goosePath;
  const spawnArgs = ['server'];

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

  const gooseProcess = spawn(spawnCommand, spawnArgs, spawnOptions);

  let certFingerprint: string | null = null;
  const fingerprintReady = new Promise<string | null>((resolve) => {
    const FINGERPRINT_PREFIX = 'GOOSED_CERT_FINGERPRINT=';
    let resolved = false;

    gooseProcess.stdout?.on('data', (data: Buffer) => {
      const text = data.toString();
      logger.info(`goose server stdout for port ${port} and dir ${workingDir}: ${text}`);

      if (!resolved && text.includes(FINGERPRINT_PREFIX)) {
        for (const line of text.split('\n')) {
          if (line.startsWith(FINGERPRINT_PREFIX)) {
            certFingerprint = line.slice(FINGERPRINT_PREFIX.length).trim();
            logger.info(`Pinned cert fingerprint: ${certFingerprint}`);
            resolved = true;
            resolve(certFingerprint);
            break;
          }
        }
      }
    });

    gooseProcess.on('exit', () => {
      if (!resolved) {
        resolved = true;
        resolve(null);
      }
    });
  });

  // Once we have the fingerprint (or the process exits before emitting one),
  // remove the stdout listener. Leaving it attached for the lifetime of the
  // long-running goose server process means every chunk of stdout data triggers
  // Node's internal EmitToJSStreamListener::OnStreamRead which converts raw
  // bytes into a JS string via v8::String::NewFromTwoByte. Over multi-hour
  // sessions this has been observed to hit a V8 assertion and crash the
  // Electron main process. Removing the listener and calling resume()
  // lets the pipe drain harmlessly without buffering into Node/V8.
  void fingerprintReady.then(() => {
    gooseProcess.stdout?.removeAllListeners('data');
    gooseProcess.stdout?.resume();
  });

  const onStderrData = (data: Buffer) => {
    const lines = data.toString().split('\n');
    for (const line of lines) {
      if (line.trim()) {
        errorLog.push(line);
        if (isFatalError(line)) {
          logger.error(`goose server stderr for port ${port} and dir ${workingDir}: ${line}`);
        }
      }
    }
  };
  gooseProcess.stderr?.on('data', onStderrData);

  const stopErrorLogCollection = () => {
    gooseProcess.stderr?.off('data', onStderrData);
  };

  gooseProcess.on('exit', (code) => {
    logger.info(
      `goose server process exited with code ${code} for port ${port} and dir ${workingDir}`
    );
  });

  gooseProcess.on('error', (err) => {
    logger.error(`Failed to start goose server on port ${port} and dir ${workingDir}`, err);
    errorLog.push(err.message);
  });

  const cleanup = async (): Promise<void> => {
    return new Promise<void>((resolve) => {
      if (!gooseProcess || gooseProcess.killed) {
        resolve();
        return;
      }

      gooseProcess.on('close', () => {
        resolve();
      });

      logger.info('Terminating goose server');
      try {
        if (process.platform === 'win32') {
          spawn('taskkill', ['/pid', gooseProcess.pid!.toString(), '/f', '/t']);
        } else {
          gooseProcess.kill('SIGTERM');
        }
      } catch (error) {
        logger.error('Error while terminating goose server process:', error);
      }

      setTimeout(() => {
        if (gooseProcess && !gooseProcess.killed && process.platform !== 'win32') {
          gooseProcess.kill('SIGKILL');
        }
        resolve();
      }, 5000);
    });
  };

  logger.info(`Goose server successfully started on port ${port}`);

  await fingerprintReady;

  return {
    baseUrl,
    workingDir,
    process: gooseProcess,
    errorLog,
    stopErrorLogCollection,
    cleanup,
    client: gooseClientForUrlAndSecret(baseUrl, serverSecret),
    certFingerprint,
  };
};
