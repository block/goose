import { app, BrowserWindow } from 'electron';
import path from 'node:path';
import { Buffer } from 'node:buffer';

export async function launchGooseApp(appName: string, jsImplementation: string): Promise<void> {
  console.log(`Launching Goose app: ${appName}`);
  const appWindow = new BrowserWindow({
    title: `Goose App - ${appName}`,
    width: 1200,
    height: 800,
    minWidth: 800,
    minHeight: 600,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      webSecurity: true,
    },
  });

  const appHtmlPath = app.isPackaged
    ? path.join(process.resourcesPath, 'assets/container.html')
    : path.join(__dirname, '../../src/goose_apps/assets/container.html');

  const encodedImplementation = Buffer.from(jsImplementation).toString('base64');
  const queryParams = new URLSearchParams({
    appName,
    implementation: encodedImplementation,
  });

  await appWindow.loadFile(appHtmlPath, {
    search: queryParams.toString(),
  });

  appWindow.show();
}
