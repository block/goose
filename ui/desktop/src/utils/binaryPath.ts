import path from 'node:path';
import fs from 'node:fs';
import Electron from 'electron';
import log from './logger';
import { execSync } from 'child_process';

const checkWindowsDependency = (dependency: string): boolean => {
  try {
    execSync(`where ${dependency}`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
};

export const getBinaryPath = (app: Electron.App, binaryName: string): string => {
  const isDev = process.env.NODE_ENV === 'development';
  const isPackaged = app.isPackaged;
  const isWindows = process.platform === 'win32';
  const executableName = isWindows ? `${binaryName}.exe` : binaryName;

  // Windows-specific handling
  if (isWindows) {
    const requiredDeps = ['npx', 'uvx'];
    const missingDeps = requiredDeps.filter((dep) => !checkWindowsDependency(dep));

    if (missingDeps.length > 0) {
      const error = `Required dependencies not found on Windows: ${missingDeps.join(', ')}. Please install them manually.`;
      log.error(error);
      throw new Error(error);
    }

    // On Windows, if dependencies are installed, return the command name directly
    // These will be resolved through PATH
    if (binaryName === 'npx' || binaryName === 'uvx') {
      log.info(`Using system-installed ${binaryName} on Windows`);
      return binaryName;
    }
  }

  // For non-Windows systems or other binaries, use the regular path resolution
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
