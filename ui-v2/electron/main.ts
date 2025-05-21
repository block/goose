import * as fs from 'fs/promises';
import * as path from 'path';

import { app, BrowserWindow, ipcMain, clipboard, dialog, session } from 'electron';
import type {
  IpcMainInvokeEvent,
  OnHeadersReceivedListenerDetails,
  HeadersReceivedResponse,
} from 'electron';

const isDevelopment = !app.isPackaged;
const isHeadless = process.env.HEADLESS === 'true';

// Enable sandbox before app is ready
app.enableSandbox();

if (isHeadless) {
  app.disableHardwareAcceleration();
}

async function createWindow() {
  // Handle different preload paths for dev and prod
  const preloadPath = isDevelopment
    ? path.join(app.getAppPath(), '.vite/build/preload/preload.js')
    : path.join(__dirname, 'preload.js');

  // Create the browser window with headless options when needed
  const mainWindow = new BrowserWindow({
    titleBarStyle: process.platform === 'darwin' ? 'hidden' : 'default',
    trafficLightPosition: process.platform === 'darwin' ? { x: 16, y: 10 } : undefined,
    frame: false,
    width: 1200,
    height: 800,
    minWidth: 800,
    ...(isHeadless
      ? {
          show: false,
          webPreferences: {
            nodeIntegration: false,
            contextIsolation: true,
            webSecurity: true,
            preload: preloadPath,
            sandbox: true,
            offscreen: true,
          },
        }
      : {
          webPreferences: {
            nodeIntegration: false,
            contextIsolation: true,
            webSecurity: true,
            preload: preloadPath,
            sandbox: true,
          },
        }),
  });

  // Set up CSP
  session.defaultSession.webRequest.onHeadersReceived(
    (
      details: OnHeadersReceivedListenerDetails,
      callback: (response: HeadersReceivedResponse) => void
    ) => {
      callback({
        responseHeaders: {
          ...details.responseHeaders,
          'Content-Security-Policy': [
            isDevelopment
              ? `
                default-src 'self';
                script-src 'self' 'unsafe-inline';
                style-src 'self' 'unsafe-inline';
                connect-src 'self' ws://localhost:3001 http://localhost:3001;
                img-src 'self' data: https:;
                font-src 'self' data: https://cash-f.squarecdn.com;
              `
                  .replace(/\s+/g, ' ')
                  .trim()
              : `
                default-src 'self';
                script-src 'self';
                style-src 'self' 'unsafe-inline';
                img-src 'self' data: https:;
                font-src 'self' data: https://cash-f.squarecdn.com;
              `
                  .replace(/\s+/g, ' ')
                  .trim(),
          ],
        },
      });
    }
  );

  // Set up IPC handlers
  ipcMain.handle('clipboard-copy', async (_: IpcMainInvokeEvent, text: string) => {
    clipboard.writeText(text);
  });

  ipcMain.handle('clipboard-read', async () => {
    return clipboard.readText();
  });

  interface SaveFileParams {
    content: string;
    fileName: string;
  }

  interface SaveFileResult {
    success: boolean;
    path?: string;
  }

  ipcMain.handle(
    'save-file',
    async (
      _: IpcMainInvokeEvent,
      { content, fileName }: SaveFileParams
    ): Promise<SaveFileResult> => {
      const { filePath } = await dialog.showSaveDialog({
        defaultPath: fileName,
      });

      if (filePath) {
        await fs.writeFile(filePath, content, 'utf8');
        return { success: true, path: filePath };
      }
      return { success: false };
    }
  );

  // Load the app
  if (isDevelopment) {
    mainWindow.loadURL('http://localhost:3001/');
    if (!isHeadless) {
      mainWindow.webContents.openDevTools();
    }
  } else {
    // In production, load from the asar archive
    mainWindow.loadFile(path.join(__dirname, 'renderer/index.html'));
  }
}

app.whenReady().then(() => {
  createWindow().catch(console.error);

  app.on('activate', function () {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow().catch(console.error);
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
