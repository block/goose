import { app, BrowserWindow } from 'electron';
import path from 'node:path';
import { Buffer } from 'node:buffer';
import { GooseApp } from '../api';
import fs from 'fs';

export async function launchGooseApp(gapp: GooseApp): Promise<void> {
  const appWindow = new BrowserWindow({
    title: gapp.name,
    width: gapp.width || 800,
    height: gapp.height || 600,
    resizable: gapp.resizable ?? true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      webSecurity: true,
    },
  });

  const appHtmlPath = app.isPackaged
    ? path.join(process.resourcesPath, 'assets/container.html')
    : path.join(__dirname, '../../src/goose_apps/assets/container.html');

  const encodedImplementation = Buffer.from(gapp.jsImplementation!).toString('base64');
  const queryParams = new URLSearchParams({
    appName: gapp.name,
    implementation: encodedImplementation,
  });

  console.log('__dirname:', __dirname);
  console.log('appHtmlPath:', appHtmlPath);
  console.log('Does file exist?', fs.existsSync(appHtmlPath));

  await appWindow.loadFile(appHtmlPath, {
    search: queryParams.toString(),
  });

  appWindow.show();
}
