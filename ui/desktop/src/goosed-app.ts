/**
 * Electron-specific wrapper for goosed process management.
 * This thin wrapper adds Electron logging and app lifecycle integration.
 */

import { app } from 'electron';
import log from './utils/logger';

import { status } from './api';
import { Client } from './api/client';
import { ExternalGoosedConfig } from './utils/settings';
import {
  findAvailablePort as findAvailablePortUtil,
  startGoosed as startGoosedCore,
  isFatalError,
  Logger,
  GoosedResult,
} from './goosed';

// Create a logger adapter from electron-log
const electronLogger: Logger = {
  info: (...args) => log.info(...args),
  error: (...args) => log.error(...args),
};

/**
 * Find an available port (with logging).
 */
export const findAvailablePort = async (): Promise<number> => {
  const port = await findAvailablePortUtil();
  log.info(`Found available port: ${port}`);
  return port;
};

/**
 * Check if goosed server is ready by polling the status endpoint.
 * Uses the API client for proper authentication.
 */
export const checkServerStatus = async (client: Client, errorLog: string[]): Promise<boolean> => {
  const interval = 100; // ms
  const maxAttempts = 100; // 10s

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    if (errorLog.some(isFatalError)) {
      log.error('Detected fatal error in server logs');
      return false;
    }

    try {
      await status({ client });
      return true;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, interval));
    }
  }

  log.error(`Server failed to respond after ${(interval * maxAttempts) / 1000} seconds`);
  return false;
};

export interface StartGoosedParams {
  dir?: string;
  serverSecret: string;
  env?: Record<string, string | undefined>;
  externalGoosed?: ExternalGoosedConfig;
}

export interface StartGoosedResult {
  baseUrl: string;
  workingDir: string;
  process: import('child_process').ChildProcess | null;
  errorLog: string[];
}

/**
 * Start or connect to a goosed server.
 * This wraps the core startGoosed with Electron-specific behavior:
 * - Uses electron-log for logging
 * - Registers cleanup on app quit
 */
export const startGoosed = async (params: StartGoosedParams): Promise<StartGoosedResult> => {
  const { dir, serverSecret, env, externalGoosed } = params;

  const result: GoosedResult = await startGoosedCore({
    dir,
    isPackaged: app.isPackaged,
    resourcesPath: app.isPackaged ? process.resourcesPath : undefined,
    serverSecret,
    env,
    externalGoosed,
    logger: electronLogger,
  });

  // Register cleanup on app quit
  app.on('will-quit', () => {
    log.info('App quitting, terminating goosed server');
    result.cleanup();
  });

  return {
    baseUrl: result.baseUrl,
    workingDir: result.workingDir,
    process: result.process,
    errorLog: result.errorLog,
  };
};
