import path from 'node:path';
import fs from 'node:fs';
import Electron from 'electron';
import log from './logger';

export const getBinaryPath = (app: Electron.App, binaryName: string): string => {
  const isWindows = process.platform === 'win32';

  const possiblePaths = [];
  if (isWindows) {
    addPaths(isWindows, possiblePaths, `${binaryName}.exe`, app);
    addPaths(isWindows, possiblePaths, `${binaryName}.cmd`, app);
  } else {
    addPaths(isWindows, possiblePaths, binaryName, app);
  }

  for (const binPath of possiblePaths) {
    try {
      if (fs.existsSync(binPath)) {
        return binPath;
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
  possiblePaths: any[],
  executableName: string,
  app: Electron.App
): void => {
  const isDev = process.env.NODE_ENV === 'development';
  const isPackaged = app.isPackaged;
  if (isDev && !isPackaged) {
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
