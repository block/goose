import { spawn, ChildProcess } from 'child_process';
import { createServer } from 'net';
import os from 'node:os';
import path from 'node:path';
import fs from 'node:fs';
import { getBinaryPath } from './utils/pathUtils';
import log from './utils/logger';
import { App } from 'electron';
import { Buffer } from 'node:buffer';
import * as yaml from 'yaml';

import { status } from './api';
import { Client } from './api/client';

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

// Check if goosed server is ready by polling the status endpoint
export const checkServerStatus = async (client: Client): Promise<boolean> => {
  const interval = 100;
  const maxAttempts = 200;
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      await status({ client, throwOnError: true });
      return true;
    } catch (error) {
      log.error('failure to connect, will retry', error);
      if (attempt === maxAttempts) {
        log.error(`Server failed to respond after ${(interval * maxAttempts) / 1000} seconds`);
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

// Function to read goose configuration from ~/.config/goose/config.yaml
const readGooseConfig = (): Record<string, any> => {
  try {
    const homeDir = os.homedir();
    const configPath = path.join(homeDir, '.config', 'goose', 'config.yaml');
    
    log.info(`Attempting to read goose config from: ${configPath}`);
    
    if (!fs.existsSync(configPath)) {
      log.info('Goose config file does not exist, using defaults');
      return {};
    }
    
    const configContent = fs.readFileSync(configPath, 'utf-8');
    const config = yaml.parse(configContent);
    
    log.info('Successfully loaded goose configuration');
    log.info(`GOOSE_PROVIDER from config: ${config.GOOSE_PROVIDER || 'not set'}`);
    log.info(`GOOSE_MODEL from config: ${config.GOOSE_MODEL || 'not set'}`);
    
    return config || {};
  } catch (error) {
    log.error('Error reading goose config file:', error);
    return {};
  }
};

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
  } catch {
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

  // Read goose configuration from config file
  const gooseConfig = readGooseConfig();

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
    // Add provider configuration from config file if available
    ...(gooseConfig.GOOSE_PROVIDER && { GOOSE_DEFAULT_PROVIDER: gooseConfig.GOOSE_PROVIDER }),
    ...(gooseConfig.GOOSE_MODEL && { GOOSE_DEFAULT_MODEL: gooseConfig.GOOSE_MODEL }),
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
  
  // Log provider configuration being used
  log.info(`GOOSE_DEFAULT_PROVIDER: ${processEnv.GOOSE_DEFAULT_PROVIDER || 'not set'}`);
  log.info(`GOOSE_DEFAULT_MODEL: ${processEnv.GOOSE_DEFAULT_MODEL || 'not set'}`);

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

  // Ensure goosed is terminated when the app quits
  app.on('will-quit', () => {
    log.info('App quitting, terminating goosed server');
    try_kill_goose();
  });

  log.info(`Goosed server successfully started on port ${port}`);
  return [port, dir, goosedProcess];
};
