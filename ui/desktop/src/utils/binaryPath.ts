import path from 'node:path';
import fs from 'node:fs';
import Electron from 'electron';
import log from './logger';

export const getBinaryPath = (app: Electron.App, binaryName: string): string => {
  const isDev = process.env.NODE_ENV === 'development';
  const isPackaged = app.isPackaged;
  const isWindows = process.platform === 'win32';

  // On Windows, use .cmd for npx and .exe for uvx
  const executableName = isWindows
    ? binaryName === 'npx'
      ? 'npx.cmd'
      : `${binaryName}.exe`
    : binaryName;

  // List of possible paths to check
  const possiblePaths = [];

  if (isDev && !isPackaged) {
    // In development, check multiple possible locations
    possiblePaths.push(
      path.join(process.cwd(), 'src', 'bin', executableName),
      path.join(process.cwd(), 'bin', executableName),
      path.join(process.cwd(), '..', '..', 'target', 'release', executableName)
    );
  } else {
    // In production, check resources paths
    possiblePaths.push(
      path.join(process.resourcesPath, 'bin', executableName),
      path.join(app.getAppPath(), 'resources', 'bin', executableName)
    );
  }

  // Log all paths we're checking
  log.info('Checking binary paths:', possiblePaths);

  // Try each path and return the first one that exists
  for (const binPath of possiblePaths) {
    try {
      if (fs.existsSync(binPath)) {
        log.info(`Found binary at: ${binPath}`);
        return binPath;
      }
    } catch (error) {
      log.error(`Error checking path ${binPath}:`, error);
    }
  }

  // If we get here, we couldn't find the binary
  const error = `Could not find ${binaryName} binary in any of the expected locations: ${possiblePaths.join(', ')}`;
  log.error(error);
  throw new Error(error);
};
