import path from 'node:path';
import Electron from 'electron';

export const getBinaryPath = (app: Electron.App, binaryName: string): string => {
  const isDev = process.env.NODE_ENV === 'development';
  const isPackaged = app.isPackaged;

  if (isDev && !isPackaged) {
    // In development, use the absolute path from the project root
    return path.join(
      process.cwd(),
      'src',
      'bin',
      process.platform === 'win32' ? `${binaryName}.exe` : binaryName
    );
  } else {
    // In production, use the path relative to the app resources
    return path.join(
      process.resourcesPath,
      'bin',
      process.platform === 'win32' ? `${binaryName}.exe` : binaryName
    );
  }
};
