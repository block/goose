import path from 'node:path';
import { execSync, spawn } from 'child_process';
import { existsSync } from 'fs';
import log from './utils/logger';
import os from 'node:os';
import { createServer } from 'net';
import { loadZshEnv } from './utils/loadEnv';


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
export const startGoosed = async (app, dir=null): Promise<[number, string]> => {
  // In will use this later to determine if we should start process
  const isDev = process.env.NODE_ENV === 'development';

  // we default to running goosed in home dir - if not specified
  const homeDir = os.homedir();
  if (!dir) {
    dir = homeDir;
  }
  
  let goosedPath: string;

  if (isDev && !app.isPackaged) {
    if (process.env.VITE_START_EMBEDDED_SERVER === 'no') {
      log.info('Skipping starting goosed in development mode');
      return [3000, dir];
    }
    // In development, use the absolute path from the project root
    goosedPath = path.join(process.cwd(), 'src', 'bin', process.platform === 'win32' ? 'goosed.exe' : 'goosed');
  } else {
    // In production, use the path relative to the app resources
    goosedPath = path.join(process.resourcesPath, 'bin', process.platform === 'win32' ? 'goosed.exe' : 'goosed');
  }
  const port = await findAvailablePort();

  // in case we want it
  //const isPackaged = app.isPackaged;
  

  log.info(`Starting goosed from: ${goosedPath} on port ${port} in dir ${dir}` );
  
  // Define additional environment variables
  const additionalEnv = {
    // Set HOME for UNIX-like systems
    HOME: homeDir,
    // Set USERPROFILE for Windows
    USERPROFILE: homeDir,

    // start with the port specified 
    GOOSE_SERVER__PORT: String(port),
  };

  // Merge parent environment with additional environment variables
  const env = { ...process.env, ...additionalEnv };

  // Spawn the goosed process with the user's home directory as cwd
  const goosedProcess = spawn(goosedPath, [], { cwd: dir, env: env, stdio: ["ignore", "pipe", "pipe"] });

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
  });

  // Ensure goosed is terminated when the app quits
  // TODO will need to do it at tab level next
  app.on('will-quit', () => {
    log.info('App quitting, terminating goosed server');
    goosedProcess.kill();
  });

  return [port, dir];
};


