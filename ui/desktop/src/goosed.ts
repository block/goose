import type { Buffer } from 'node:buffer';
import { type ChildProcess, spawn } from 'node:child_process';
import fs from 'node:fs';
import { createServer } from 'node:net';
import os from 'node:os';
import path from 'node:path';
import type Electron from 'electron';
import type { App } from 'electron';
import { status } from './api';
import type { Client } from './api/client';
import log from './utils/logger';
import type { ExternalGoosedConfig } from './utils/settings';

export const findAvailablePort = (): Promise<number> => {
  return new Promise((resolve, _reject) => {
    const server = createServer();

    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address() as { port: number };
      server.close(() => {
        log.info(`Found available port: ${port}`);
        resolve(port);
      });
    });
  });
};

// Check if goosed server is ready by polling the status endpoint
export const checkServerStatus = async (client: Client, errorLog: string[]): Promise<boolean> => {
  const interval = 100; // ms
  const maxAttempts = 100; // 10s

  const fatal = (line: string) => {
    const trimmed = line.trim().toLowerCase();
    return trimmed.startsWith("thread 'main' panicked at") || trimmed.startsWith('error:');
  };

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    if (errorLog.some(fatal)) {
      log.error('Detected fatal error in server logs');
      return false;
    }
    try {
      await status({ client, throwOnError: true });
      return true;
    } catch {
      if (attempt === maxAttempts) {
        log.error(`Server failed to respond after ${(interval * maxAttempts) / 1000} seconds`);
      }
    }
    await new Promise((resolve) => setTimeout(resolve, interval));
  }
  return false;
};

export interface GoosedResult {
  baseUrl: string;
  workingDir: string;
  process: ChildProcess;
  errorLog: string[];
}

const connectToExternalBackend = (workingDir: string, url: string): GoosedResult => {
  log.info(`Using external goosed backend at ${url}`);

  const mockProcess = {
    pid: undefined,
    kill: () => {
      log.info(`Not killing external process that is managed externally`);
    },
  } as ChildProcess;

  return { baseUrl: url, workingDir, process: mockProcess, errorLog: [] };
};

interface GooseProcessEnv {
  [key: string]: string | undefined;

  HOME: string;
  USERPROFILE: string;
  APPDATA: string;
  LOCALAPPDATA: string;
  PATH: string;
  GOOSE_PORT: string;
  GOOSE_SERVER__SECRET_KEY?: string;
}

export interface StartGoosedOptions {
  app: App;
  serverSecret: string;
  dir: string;
  env?: Partial<GooseProcessEnv>;
  externalGoosed?: ExternalGoosedConfig;
}

export const startGoosed = async (options: StartGoosedOptions): Promise<GoosedResult> => {
  const { app, serverSecret, dir: inputDir, env = {}, externalGoosed } = options;
  const isWindows = process.platform === 'win32';
  const homeDir = os.homedir();
  const dir = path.resolve(path.normalize(inputDir));

  if (externalGoosed?.enabled && externalGoosed.url) {
    return connectToExternalBackend(dir, externalGoosed.url);
  }

  if (process.env.GOOSE_EXTERNAL_BACKEND) {
    const port = process.env.GOOSE_PORT || '3000';
    return connectToExternalBackend(dir, `http://127.0.0.1:${port}`);
  }

  let goosedPath = getGoosedBinaryPath(app);

  const resolvedGoosedPath = path.resolve(goosedPath);

  const port = await findAvailablePort();
  const stderrLines: string[] = [];

  log.info(`Starting goosed from: ${resolvedGoosedPath} on port ${port} in dir ${dir}`);

  const additionalEnv: GooseProcessEnv = {
    HOME: homeDir,
    USERPROFILE: homeDir,
    APPDATA: process.env.APPDATA || path.join(homeDir, 'AppData', 'Roaming'),
    LOCALAPPDATA: process.env.LOCALAPPDATA || path.join(homeDir, 'AppData', 'Local'),
    PATH: `${path.dirname(resolvedGoosedPath)}${path.delimiter}${process.env.PATH || ''}`,
    GOOSE_PORT: String(port),
    GOOSE_SERVER__SECRET_KEY: serverSecret,
    ...env,
  } as GooseProcessEnv;

  const processEnv: GooseProcessEnv = { ...process.env, ...additionalEnv } as GooseProcessEnv;

  if (isWindows && !resolvedGoosedPath.toLowerCase().endsWith('.exe')) {
    goosedPath = `${resolvedGoosedPath}.exe`;
  } else {
    goosedPath = resolvedGoosedPath;
  }
  log.info(`Binary path resolved to: ${goosedPath}`);

  const spawnOptions = {
    cwd: dir,
    env: processEnv,
    stdio: ['ignore', 'pipe', 'pipe'] as ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
    detached: isWindows,
    shell: false,
  };

  const safeSpawnOptions = {
    ...spawnOptions,
    env: Object.keys(spawnOptions.env || {}).reduce(
      (acc, key) => {
        if (key.includes('SECRET') || key.includes('PASSWORD') || key.includes('TOKEN')) {
          acc[key] = '[REDACTED]';
        } else {
          acc[key] = spawnOptions.env?.[key] || '';
        }
        return acc;
      },
      {} as Record<string, string>
    ),
  };
  log.info('Spawn options:', JSON.stringify(safeSpawnOptions, null, 2));

  const safeArgs = ['agent'];

  const goosedProcess: ChildProcess = spawn(goosedPath, safeArgs, spawnOptions);

  if (isWindows && goosedProcess.unref) {
    goosedProcess.unref();
  }

  goosedProcess.stdout?.on('data', (data: Buffer) => {
    log.info(`goosed stdout for port ${port} and dir ${dir}: ${data.toString()}`);
  });

  goosedProcess.stderr?.on('data', (data: Buffer) => {
    const lines = data
      .toString()
      .split('\n')
      .filter((l) => l.trim());
    lines.forEach((line) => {
      log.error(`goosed stderr for port ${port} and dir ${dir}: ${line}`);
      stderrLines.push(line);
    });
  });

  goosedProcess.on('close', (code: number | null) => {
    log.info(`goosed process exited with code ${code} for port ${port} and dir ${dir}`);
  });

  goosedProcess.on('error', (err: Error) => {
    log.error(`Failed to start goosed on port ${port} and dir ${dir}`, err);
    throw err;
  });

  const try_kill_goose = () => {
    try {
      if (isWindows) {
        const pid = goosedProcess.pid?.toString() || '0';
        spawn('taskkill', ['/pid', pid, '/T', '/F'], { shell: false });
      } else {
        goosedProcess.kill?.();
      }
    } catch (error) {
      log.error('Error while terminating goosed process:', error);
    }
  };

  app.on('will-quit', () => {
    log.info('App quitting, terminating goosed server');
    try_kill_goose();
  });

  log.info(`Goosed server successfully started on port ${port}`);
  return {
    baseUrl: `http://127.0.0.1:${port}`,
    workingDir: dir,
    process: goosedProcess,
    errorLog: stderrLines,
  };
};

const getGoosedBinaryPath = (app: Electron.App): string => {
  const executableName = process.platform === 'win32' ? 'goosed.exe' : 'goosed';

  let possiblePaths: string[];
  if (!app.isPackaged) {
    possiblePaths = [
      path.join(process.cwd(), 'src', 'bin', executableName),
      path.join(process.cwd(), 'bin', executableName),
      path.join(process.cwd(), '..', '..', 'target', 'debug', executableName),
      path.join(process.cwd(), '..', '..', 'target', 'release', executableName),
    ];
  } else {
    possiblePaths = [path.join(process.resourcesPath, 'bin', executableName)];
  }

  for (const binPath of possiblePaths) {
    try {
      const resolvedPath = path.resolve(binPath);

      if (fs.existsSync(resolvedPath)) {
        const stats = fs.statSync(resolvedPath);
        if (stats.isFile()) {
          return resolvedPath;
        } else {
          log.error(`Path exists but is not a regular file: ${resolvedPath}`);
        }
      }
    } catch (error) {
      log.error(`Error checking path ${binPath}:`, error);
    }
  }

  throw new Error(
    `Could not find ${executableName} binary in any of the expected locations: ${possiblePaths.join(
      ', '
    )}`
  );
};
