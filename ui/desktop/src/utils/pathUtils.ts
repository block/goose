import path from 'node:path';
import fs from 'node:fs';
import os from 'node:os';
import Electron from 'electron';
import log from './logger';

export const getBinaryPath = (app: Electron.App, binaryName: string): string => {
  // On Windows, rely on PATH we just patched in ensureWinShims for command-line tools
  // but use explicit resources/bin path for goosed.exe
  if (process.platform === 'win32') {
    // For goosed.exe, always use the explicit resources/bin path
    if (binaryName === 'goosed') {
      return path.join(process.resourcesPath, 'bin', 'goosed.exe');
    }

    // Map binary names to their Windows equivalents
    const windowsBinaryMap: Record<string, string> = {
      npx: 'npx.cmd',
      uvx: 'uvx.exe',
    };

    // For other binaries, use Windows-specific extensions if available
    const windowsBinary = windowsBinaryMap[binaryName] || binaryName;
    return windowsBinary;
  }

  // For non-Windows platforms, use the original logic
  const possiblePaths: string[] = [];
  addPaths(false, possiblePaths, binaryName, app);

  for (const binPath of possiblePaths) {
    try {
      // Security: Resolve the path and validate it's within expected directories
      const resolvedPath = path.resolve(binPath);

      // Ensure the resolved path doesn't contain suspicious sequences
      if (
        resolvedPath.includes('..') ||
        resolvedPath.includes(';') ||
        resolvedPath.includes('|') ||
        resolvedPath.includes('&')
      ) {
        log.error(`Suspicious path detected, skipping: ${resolvedPath}`);
        continue;
      }

      if (fs.existsSync(resolvedPath)) {
        // Additional security check: ensure it's a regular file
        const stats = fs.statSync(resolvedPath);
        if (stats.isFile()) {
          return resolvedPath;
        } else {
          log.error(`Path exists but is not a regular file: ${resolvedPath}`);
        }
      }
    } catch (error) {
      log.error(`Error checking path ${binPath}:`, error);
    }
  }

  throw new Error(
    `Could not find ${binaryName} binary in any of the expected locations: ${possiblePaths.join(
      ', '
    )}`
  );
};

const addPaths = (
  isWindows: boolean,
  possiblePaths: string[],
  executableName: string,
  app: Electron.App
): void => {
  if (!app.isPackaged) {
    possiblePaths.push(
      path.join(process.cwd(), 'src', 'bin', executableName),
      path.join(process.cwd(), 'bin', executableName),
      path.join(process.cwd(), '..', '..', 'target', 'release', executableName)
    );
  } else {
    possiblePaths.push(
      path.join(process.resourcesPath, 'bin', executableName),
      path.join(app.getAppPath(), 'resources', 'bin', executableName)
    );

    if (isWindows) {
      possiblePaths.push(
        path.join(process.resourcesPath, executableName),
        path.join(app.getAppPath(), 'resources', executableName),
        path.join(app.getPath('exe'), '..', 'bin', executableName)
      );
    }
  }
};

/**
 * Expands tilde (~) to the user's home directory
 * @param filePath - The file path that may contain tilde
 * @returns The expanded path with tilde replaced by home directory
 */
export function expandTilde(filePath: string): string {
  if (!filePath || typeof filePath !== 'string') return filePath;
  // Support "~", "~/..." and "~\\..." on Windows
  if (filePath === '~') {
    return os.homedir();
  }
  if (filePath.startsWith('~/') || (process.platform === 'win32' && filePath.startsWith('~\\'))) {
    // Remove the leading "~" and any separator that follows, then join
    const remainder = filePath.slice(2);
    return path.join(os.homedir(), remainder);
  }
  if (filePath.startsWith('~')) {
    // Generic fallback: replace only the first leading tilde
    return path.join(os.homedir(), filePath.slice(1));
  }
  return filePath;
}
