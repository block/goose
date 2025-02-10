import { spawn } from 'child_process';
import { createServer } from 'net';
import os from 'node:os';
import path from 'node:path';
import { getBinaryPath } from './utils/binaryPath';
import log from './utils/logger';
import { ChildProcessByStdio } from 'node:child_process';
import { Readable } from 'node:stream';

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
  maxAttempts: number = 60,
  interval: number = 100
): Promise<boolean> => {
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

export const startGoosed = async (
  app,
  dir = null,
  env = {}
): Promise<[number, string, ChildProcessByStdio<null, Readable, Readable>]> => {
  // we default to running goosed in home dir - if not specified
  const homeDir = os.homedir();
  const isWindows = process.platform === 'win32';

  // Ensure dir is properly normalized for the platform
  if (!dir) {
    dir = homeDir;
  }
  dir = path.normalize(dir);

  // Get the goosed binary path using the shared utility
  let goosedPath = getBinaryPath(app, 'goosed');
  const port = await findAvailablePort();

  log.info(`Starting goosed from: ${goosedPath} on port ${port} in dir ${dir}`);

  // Define additional environment variables
  const additionalEnv = {
    // Set HOME for UNIX-like systems
    HOME: homeDir,
    // Set USERPROFILE for Windows
    USERPROFILE: homeDir,
    // Set APPDATA for Windows
    APPDATA: process.env.APPDATA || path.join(homeDir, 'AppData', 'Roaming'),
    // Set LOCAL_APPDATA for Windows
    LOCALAPPDATA: process.env.LOCALAPPDATA || path.join(homeDir, 'AppData', 'Local'),
    // Set PATH to include the binary directory
    PATH: `${path.dirname(goosedPath)}${path.delimiter}${process.env.PATH}`,
    // start with the port specified
    GOOSE_PORT: String(port),
    GOOSE_SERVER__SECRET_KEY: process.env.GOOSE_SERVER__SECRET_KEY,
    // Add any additional environment variables passed in
    ...env,
  };

  // Merge parent environment with additional environment variables
  const processEnv = { ...process.env, ...additionalEnv };

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
  if (isWindows && !goosedPath.toLowerCase().endsWith('.exe')) {
    goosedPath += '.exe';
  }
  log.info(`Binary path resolved to: ${goosedPath}`);

  // Verify binary exists
  try {
    const fs = require('fs');
    const stats = fs.statSync(goosedPath);
    log.info(`Binary exists: ${stats.isFile()}`);
  } catch (error) {
    log.error(`Binary not found at ${goosedPath}:`, error);
    throw new Error(`Binary not found at ${goosedPath}`);
  }

  const spawnOptions = {
    cwd: dir,
    env: processEnv,
    stdio: ['ignore', 'pipe', 'pipe'],
    // Hide terminal window on Windows
    windowsHide: true,
    // Run detached on Windows only to avoid terminal windows
    detached: isWindows,
    // Never use shell to avoid terminal windows
    shell: false,
  };

  // Log spawn options for debugging
  log.info('Spawn options:', JSON.stringify(spawnOptions, null, 2));

  // Spawn the goosed process
  const goosedProcess = spawn(goosedPath, ['agent'], spawnOptions);

  // Only unref on Windows to allow it to run independently of the parent
  if (isWindows) {
    goosedProcess.unref();
  }

  goosedProcess.stdout.on('data', (data) => {
    log.info(`goosed stdout for port ${port} and dir ${dir}: ${data.toString()}`);
  });

  goosedProcess.stderr.on('data', (data) => {
    log.error(`goosed stderr for port ${port} and dir ${dir}: ${data.toString()}`);
  });

  goosedProcess.on('close', (code) => {
    log.info(`goosed process exited with code ${code} for port ${port} and dir ${dir}`);
  });

  goosedProcess.on('error', (err) => {
    log.error(`Failed to start goosed on port ${port} and dir ${dir}`, err);
    throw err; // Propagate the error
  });

  // Wait for the server to be ready
  const isReady = await checkServerStatus(port);
  log.info(`Goosed isReady ${isReady}`);
  if (!isReady) {
    log.error(`Goosed server failed to start on port ${port}`);
    try {
      if (isWindows) {
        // On Windows, use taskkill to forcefully terminate the process tree
        spawn('taskkill', ['/pid', goosedProcess.pid.toString(), '/T', '/F']);
      } else {
        goosedProcess.kill();
      }
    } catch (error) {
      log.error('Error while terminating goosed process:', error);
    }
    throw new Error(`Goosed server failed to start on port ${port}`);
  }

  // Ensure goosed is terminated when the app quits
  // TODO will need to do it at tab level next
  app.on('will-quit', () => {
    log.info('App quitting, terminating goosed server');
    try {
      if (isWindows) {
        // On Windows, use taskkill to forcefully terminate the process tree
        spawn('taskkill', ['/pid', goosedProcess.pid.toString(), '/T', '/F']);
      } else {
        goosedProcess.kill();
      }
    } catch (error) {
      log.error('Error while terminating goosed process:', error);
    }
  });

  log.info(`Goosed server successfully started on port ${port}`);
  return [port, dir, goosedProcess];
};
