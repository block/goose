import { spawn } from 'child_process';
import { createServer } from 'net';
import os from 'node:os';
import { getBinaryPath } from './utils/binaryPath';
import log from './utils/logger';

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

// Function to fetch agent version from the server
const fetchAgentVersion = async (port: number): Promise<string> => {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/api/agent/versions`);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const data = await response.json();
    return data.current_version;
  } catch (error) {
    log.error('Failed to fetch agent version:', error);
    return 'unknown';
  }
};

// Check if goosed server is ready by polling the status endpoint
const checkServerStatus = async (port: number, maxAttempts: number = 60, interval: number = 100): Promise<boolean> => {
  const statusUrl = `http://127.0.0.1:${port}/status`;
  log.info(`Checking goosed server status at ${statusUrl}`);

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const response = await fetch(statusUrl);
      if (response.ok) {
        log.info(`goosed server is ready`);
        return true;
      }
    } catch (error) {
      // Expected error when server isn't ready yet
      if (attempt === maxAttempts) {
        log.error(`goosed server failed to respond after ${maxAttempts} attempts:`, error);
      }
    }
    await new Promise(resolve => setTimeout(resolve, interval));
  }
  return false;
};

export const startGoosed = async (app, dir=null, env={}, maxRestartAttempts = 5): Promise<[number, string, string]> => {
  // In will use this later to determine if we should start process
  const isDev = process.env.NODE_ENV === 'development';

  // we default to running goosed in home dir - if not specified
  const homeDir = os.homedir();
  if (!dir) {
    dir = homeDir;
  }
  
  // Skip starting goosed if configured in dev mode
  if (isDev && !app.isPackaged && process.env.VITE_START_EMBEDDED_SERVER === 'no') {
    log.info('Skipping starting goosed in development mode');
    return [3000, dir, 'dev'];
  }

  // Get the goosed binary path using the shared utility
  const goosedPath = getBinaryPath(app, 'goosed');
  const port = await findAvailablePort();

  log.info(`Starting goosed from: ${goosedPath} on port ${port} in dir ${dir}`);
  
  // Define additional environment variables
  const additionalEnv = {
    HOME: homeDir,
    USERPROFILE: homeDir,
    GOOSE_SERVER__PORT: String(port),
    GOOSE_SERVER__SECRET_KEY: process.env.GOOSE_SERVER__SECRET_KEY,
    ...env
  };

  // Merge parent environment with additional environment variables
  const processEnv = { ...process.env, ...additionalEnv };

  let restartAttempts = 0;
  let goosedProcess = null;

  const startProcess = async () => {
    goosedProcess = spawn(goosedPath, ["agent"], {
      cwd: dir,
      env: processEnv,
      stdio: ["ignore", "pipe", "pipe"]
    });

    goosedProcess.stdout.on('data', (data) => {
      log.info(`goosed stdout for port ${port} and dir ${dir}: ${data.toString()}`);
    });

    goosedProcess.stderr.on('data', (data) => {
      log.error(`goosed stderr for port ${port} and dir ${dir}: ${data.toString()}`);
    });

    goosedProcess.on('error', (err) => {
      log.error(`Failed to start goosed on port ${port} and dir ${dir}`, err);
    });

    const isReady = await checkServerStatus(port);

    if (!isReady) {
      throw new Error('Server failed to start');
    }
  };

  // Initial start
  try {
    await startProcess();
  } catch (error) {
    log.error('Initial start failed:', error);
    throw error;
  }

  // Handle crashes by attempting to restart a max number of five times
  goosedProcess.on('close', async (code) => {
    log.info(`goosed process exited with code ${code} for port ${port} and dir ${dir}`);
    if (code !== 0 && restartAttempts < maxRestartAttempts) {
      restartAttempts++;
      log.info(`Attempting restart ${restartAttempts}/${maxRestartAttempts}`);
      try {
        await startProcess();
      } catch (error) {
        log.error(`Restart attempt ${restartAttempts} failed:`, error);
      }
    }
  });

  // Ensure goosed is terminated when the app quits
  app.on('will-quit', () => {
    log.info('App quitting, terminating goosed server');
    if (goosedProcess) {
      goosedProcess.kill();
    }
  });

  // Wait for the server to start and fetch the agent version
  await new Promise(resolve => setTimeout(resolve, 1000)); // Give the server time to start
  const agentVersion = await fetchAgentVersion(port);

  log.info(`Goosed server successfully started on port ${port}`);
  return [port, dir, agentVersion];
};