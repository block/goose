import path from 'node:path';
import Electron from 'electron';

export const getBinaryPath = (app: Electron.App, binaryName: string): string => {
  const isDev = process.env.NODE_ENV === 'development';
  const isPackaged = app.isPackaged;
  const isWindows = process.platform === 'win32';
  const executableName = isWindows ? `${binaryName}.exe` : binaryName;

  if (isDev && !isPackaged) {
    // In development, use the absolute path from the project root
    return path.join(process.cwd(), 'src', 'bin', executableName);
  } else {
    if (isWindows) {
      // On Windows in production, the exe should be in the same directory as the app
      return path.join(path.dirname(app.getPath('exe')), executableName);
    } else {
      // On other platforms in production, use the path relative to app resources
      return path.join(process.resourcesPath, 'bin', executableName);
    }
  }
};
