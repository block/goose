import { spawn, ChildProcess } from 'child_process';
import { createServer } from 'net';
import os from 'node:os';
import path from 'node:path';
import fs from 'node:fs';
import { getBinaryPath } from './utils/pathUtils';
import log from './utils/logger';
import { App } from 'electron';
import { Buffer } from 'node:buffer';

// Find an available port to start goosed on
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

// Goose process manager. Take in the app, port, and directory to start goosed in.
// Check if goosed server is ready by polling the status endpoint
const checkServerStatus = async (
  port: number,
  maxAttempts?: number,
  interval: number = 100
): Promise<boolean> => {
  if (maxAttempts === undefined) {
    const isTemporalEnabled = process.env.GOOSE_SCHEDULER_TYPE === 'temporal';
    maxAttempts = isTemporalEnabled ? 200 : 80;
    log.info(
      `Using ${maxAttempts} max attempts (temporal scheduling: ${isTemporalEnabled ? 'enabled' : 'disabled'})`
    );
  }

  const statusUrl = `http://127.0.0.1:${port}/status`;
  log.info(`Checking server status at ${statusUrl}`);

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const response = await fetch(statusUrl);
      if (response.ok) {
        log.info(`Server is ready after ${attempt} attempts`);
        return true;
      }
    } catch (error) {
      // Expected error when server isn't ready yet
      if (attempt === maxAttempts) {
        log.error(`Server failed to respond after ${maxAttempts} attempts:`, error);
      }
    }
    await new Promise((resolve) => setTimeout(resolve, interval));
  }
  return false;
};

const connectToExternalBackend = async (
  workingDir: string,
  port: number = 3000
): Promise<[number, string, ChildProcess]> => {
  log.info(`Using external goosed backend on port ${port}`);

  const isReady = await checkServerStatus(port);
  if (!isReady) {
    throw new Error(`External goosed server not accessible on port ${port}`);
  }

  const mockProcess = {
    pid: undefined,
    kill: () => {
      log.info(`Not killing external process that is managed externally`);
    },
  } as ChildProcess;

  return [port, workingDir, mockProcess];
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

export const startGoosed = async (
  app: App,
  serverSecret: string,
  dir: string | null = null,
  env: Partial<GooseProcessEnv> = {}
): Promise<[number, string, ChildProcess]> => {
  const homeDir = os.homedir();
  const isWindows = process.platform === 'win32';

  if (!dir) {
    dir = homeDir;
  }

  dir = path.resolve(path.normalize(dir));

  if (process.env.GOOSE_EXTERNAL_BACKEND) {
    return connectToExternalBackend(dir, 3000);
  }

  // Validate that the directory actually exists and is a directory
  try {
    const stats = fs.lstatSync(dir);

    // Reject symlinks for security - they could point outside intended directories
    if (stats.isSymbolicLink()) {
      log.warn(`Provided path is a symlink, falling back to home directory for security`);
      dir = homeDir;
    } else if (!stats.isDirectory()) {
      log.warn(`Provided path is not a directory, falling back to home directory`);
      dir = homeDir;
    }
  } catch (error) {
    log.warn(`Directory does not exist, falling back to home directory`);
    dir = homeDir;
  }

  // Security check: Ensure the directory path doesn't contain suspicious characters
  if (dir.includes('..') || dir.includes(';') || dir.includes('|') || dir.includes('&')) {
    throw new Error(`Invalid directory path: ${dir}`);
  }

  // Get the goosed binary path using the shared utility
  let goosedPath = getBinaryPath(app, 'goosed');

  // Security validation: Ensure the binary path is safe
  const resolvedGoosedPath = path.resolve(goosedPath);

  // Validate that the binary path doesn't contain suspicious characters or sequences
  if (
    resolvedGoosedPath.includes('..') ||
    resolvedGoosedPath.includes(';') ||
    resolvedGoosedPath.includes('|') ||
    resolvedGoosedPath.includes('&') ||
    resolvedGoosedPath.includes('`') ||
    resolvedGoosedPath.includes('$')
  ) {
    throw new Error(`Invalid binary path detected: ${resolvedGoosedPath}`);
  }

  // Ensure the binary path is within expected application directories
  const appPath = app.getAppPath();
  const resourcesPath = process.resourcesPath;
  const currentWorkingDir = process.cwd();

  const isValidPath =
    resolvedGoosedPath.startsWith(path.resolve(appPath)) ||
    resolvedGoosedPath.startsWith(path.resolve(resourcesPath)) ||
    resolvedGoosedPath.startsWith(path.resolve(currentWorkingDir));

  if (!isValidPath) {
    throw new Error(`Binary path is outside of allowed directories: ${resolvedGoosedPath}`);
  }

  const port = await findAvailablePort();

  log.info(`Starting goosed from: ${resolvedGoosedPath} on port ${port} in dir ${dir}`);

  // Define additional environment variables
  const additionalEnv: GooseProcessEnv = {
    // Set HOME for UNIX-like systems
    HOME: homeDir,
    // Set USERPROFILE for Windows
    USERPROFILE: homeDir,
    // Set APPDATA for Windows
    APPDATA: process.env.APPDATA || path.join(homeDir, 'AppData', 'Roaming'),
    // Set LOCAL_APPDATA for Windows
    LOCALAPPDATA: process.env.LOCALAPPDATA || path.join(homeDir, 'AppData', 'Local'),
    // Set PATH to include the binary directory
    PATH: `${path.dirname(resolvedGoosedPath)}${path.delimiter}${process.env.PATH || ''}`,
    // start with the port specified
    GOOSE_PORT: String(port),
    GOOSE_SERVER__SECRET_KEY: serverSecret,
    // Add any additional environment variables passed in
    ...env,
  } as GooseProcessEnv;

  // Merge parent environment with additional environment variables
  const processEnv: GooseProcessEnv = { ...process.env, ...additionalEnv } as GooseProcessEnv;

  // Add detailed logging for troubleshooting
  log.info(`Process platform: ${process.platform}`);
  log.info(`Process cwd: ${process.cwd()}`);
  log.info(`Target working directory: ${dir}`);
  log.info(`Environment HOME: ${processEnv.HOME}`);
  log.info(`Environment USERPROFILE: ${processEnv.USERPROFILE}`);
  log.info(`Environment APPDATA: ${processEnv.APPDATA}`);
  log.info(`Environment LOCALAPPDATA: ${processEnv.LOCALAPPDATA}`);
  log.info(`Environment PATH: ${processEnv.PATH}`);

  // Ensure proper executable path on Windows
  if (isWindows && !resolvedGoosedPath.toLowerCase().endsWith('.exe')) {
    goosedPath = resolvedGoosedPath + '.exe';
  } else {
    goosedPath = resolvedGoosedPath;
  }
  log.info(`Binary path resolved to: ${goosedPath}`);

  const spawnOptions = {
    cwd: dir,
    env: processEnv,
    stdio: ['ignore', 'pipe', 'pipe'] as ['ignore', 'pipe', 'pipe'],
    // Hide terminal window on Windows
    windowsHide: true,
    // Run detached on Windows only to avoid terminal windows
    detached: isWindows,
    // Never use shell to avoid command injection - this is critical for security
    shell: false,
  };

  // Log spawn options for debugging (excluding sensitive env vars)
  const safeSpawnOptions = {
    ...spawnOptions,
    env: Object.keys(spawnOptions.env || {}).reduce(
      (acc, key) => {
        if (key.includes('SECRET') || key.includes('PASSWORD') || key.includes('TOKEN')) {
          acc[key] = '[REDACTED]';
        } else {
          acc[key] = spawnOptions.env![key] || '';
        }
        return acc;
      },
      {} as Record<string, string>
    ),
  };
  log.info('Spawn options:', JSON.stringify(safeSpawnOptions, null, 2));

  // Security: Use only hardcoded, safe arguments
  const safeArgs = ['agent']; // Only allow the 'agent' argument

  // Spawn the goosed process with validated inputs
  const goosedProcess: ChildProcess = spawn(goosedPath, safeArgs, spawnOptions);

  // Only unref on Windows to allow it to run independently of the parent
  if (isWindows && goosedProcess.unref) {
    goosedProcess.unref();
  }

  goosedProcess.stdout?.on('data', (data: Buffer) => {
    log.info(`goosed stdout for port ${port} and dir ${dir}: ${data.toString()}`);
  });

  goosedProcess.stderr?.on('data', (data: Buffer) => {
    log.error(`goosed stderr for port ${port} and dir ${dir}: ${data.toString()}`);
  });

  goosedProcess.on('close', (code: number | null) => {
    log.info(`goosed process exited with code ${code} for port ${port} and dir ${dir}`);
  });

  goosedProcess.on('error', (err: Error) => {
    log.error(`Failed to start goosed on port ${port} and dir ${dir}`, err);
    throw err; // Propagate the error
  });

  // Wait for the server to be ready
  const isReady = await checkServerStatus(port);
  log.info(`Goosed isReady ${isReady}`);

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

  if (!isReady) {
    log.error(`Goosed server failed to start on port ${port}`);
    try_kill_goose();
    throw new Error(`Goosed server failed to start on port ${port}`);
  }

  // Ensure goosed is terminated when the app quits
  // TODO will need to do it at tab level next
  app.on('will-quit', () => {
    log.info('App quitting, terminating goosed server');
    try_kill_goose();
  });

  log.info(`Goosed server successfully started on port ${port}`);
  return [port, dir, goosedProcess];
};
