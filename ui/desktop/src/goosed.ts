import { spawn } from 'child_process';
import { createServer } from 'net';
import os from 'node:os';
import { getBinaryPath } from './utils/binaryPath';
import log from './utils/logger';
import { ChildProcessByStdio } from 'node:child_process';
import { Readable } from 'node:stream';

// Find an available port to start goosed on
export const findAvailablePort = (): Promise<number> => {
  return new Promise((resolve, reject) => {
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
  if (!dir) {
    dir = homeDir;
  }

  // Get the goosed binary path using the shared utility
  const goosedPath = getBinaryPath(app, 'goosed');
  const port = await findAvailablePort();

  // in case we want it
  //const isPackaged = app.isPackaged;
  log.info(`Starting goosed from: ${goosedPath} on port ${port} in dir ${dir}`);

  // Define additional environment variables
  const additionalEnv = {
    // Set HOME for UNIX-like systems
    HOME: homeDir,
    // Set USERPROFILE for Windows
    USERPROFILE: homeDir,

    // start with the port specified
    GOOSE_PORT: String(port),

    GOOSE_SERVER__SECRET_KEY: process.env.GOOSE_SERVER__SECRET_KEY,

    // Add any additional environment variables passed in
    ...env,
  };

  // Merge parent environment with additional environment variables
  const processEnv = { ...process.env, ...additionalEnv };

  // Spawn the goosed process with the user's home directory as cwd
  const goosedProcess = spawn(goosedPath, ['agent'], {
    cwd: dir,
    env: processEnv,
    stdio: ['ignore', 'pipe', 'pipe'],
  });

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
    goosedProcess.kill();
    throw new Error(`Goosed server failed to start on port ${port}`);
  }

  // Ensure goosed is terminated when the app quits
  // TODO will need to do it at tab level next
  app.on('will-quit', () => {
    log.info('App quitting, terminating goosed server');
    goosedProcess.kill();
  });

  log.info(`Goosed server successfully started on port ${port}`);
  return [port, dir, goosedProcess];
};
