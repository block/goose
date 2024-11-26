import path from 'node:path';
import { spawn } from 'child_process';
import { existsSync } from 'fs';
import log from './utils/logger';
import os from 'node:os';

// Start the goosed binary
export const start = (app) => {
  // In development, the binary is in src/bin
  // In production, it will be in the resources/bin directory
  const isDev = process.env.NODE_ENV === 'development';
  let goosedPath: string;

  if (isDev) {
    // In development, use the absolute path from the project root
    goosedPath = path.join(process.cwd(), 'src', 'bin', process.platform === 'win32' ? 'goosed.exe' : 'goosed');
  } else {
    // In production, use the path relative to the app resources
    goosedPath = path.join(process.resourcesPath, 'bin', process.platform === 'win32' ? 'goosed.exe' : 'goosed');
  }

  log.info(`Starting goosed from: ${goosedPath}`);
  
  // Check if the binary exists
  if (!existsSync(goosedPath)) {
    log.error(`goosed binary not found at path: ${goosedPath}`);
    return;
  }

  const homeDir = os.homedir();

  // Optional: Ensure the home directory exists (it should, but for safety)
  if (!existsSync(homeDir)) {
    log.error(`Home directory does not exist: ${homeDir}`);
    return;
  }

  console.log("Starting goosed in: ", homeDir);

  // Spawn the goosed process with the user's home directory as cwd
  const goosedProcess = spawn(goosedPath, [], { cwd: homeDir });


  goosedProcess.stdout.on('data', (data) => {
    log.info(`goosed stdout: ${data.toString()}`);
  });

  goosedProcess.stderr.on('data', (data) => {
    log.error(`goosed stderr: ${data.toString()}`);
  });

  goosedProcess.on('close', (code) => {
    log.info(`goosed process exited with code ${code}`);
  });

  goosedProcess.on('error', (err) => {
    log.error('Failed to start goosed:', err);
  });

  // Ensure goosed is terminated when the app quits
  app.on('will-quit', () => {
    log.info('App quitting, terminating goosed server');
    goosedProcess.kill();
  });
};