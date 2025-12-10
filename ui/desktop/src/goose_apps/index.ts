import { app, BrowserWindow, ipcMain } from 'electron';
import path from 'node:path';
import { GooseApp, resumeAgent, startAgent } from '../api';
import { handleMCPRequest } from './mcpRequests';
import { injectMCPClient } from './injectMcpClient';
import { Client } from '../api/client';

export interface InlineAppContext {
  sessionId: string;
  extensionName: string;
}

const appContexts = new Map<number, { gapp: GooseApp; html: string, sessionId: string }>();

let handlersRegistered = false;
export function registerMCPAppHandlers(goosedClients :Map<number, Client>) {
  if (handlersRegistered) return;

  ipcMain.handle('get-app-html', async (event) => {
    const windowId = event.sender.id;
    const context = appContexts.get(windowId);
    if (!context) {
      throw new Error('App context not found');
    }
    return context.html;
  });

  ipcMain.handle('mcp-request', async (event, msg, inlineContext?: InlineAppContext) => {
    const windowId = BrowserWindow.fromWebContents(event.sender)?.id;
    if (!windowId) {
      throw new Error('Window not found');
    }

    const client = goosedClients.get(windowId);
    if (!client) {
      throw new Error('Client not found for window');
    }

    if (inlineContext) {
      const gapp: GooseApp = {
        name: inlineContext.extensionName,
        html: '',
        width: null,
        height: null,
        resizable: true,
        prd: '',
        description: null,
      };
      return handleMCPRequest(msg, gapp, inlineContext.sessionId, client);
    } else {
      const context = appContexts.get(windowId);
      if (!context) {
        throw Error('Context not found for windowId');
      }
      return handleMCPRequest(msg, context.gapp, context.sessionId, client);
    }
  });
}

export async function launchGooseApp(gapp: GooseApp, client:Client): Promise<BrowserWindow> {
  const desiredContentWidth = gapp.width || 800;
  const desiredContentHeight = gapp.height || 600;

  const appWindow = new BrowserWindow({
    title: gapp.name,
    width: desiredContentWidth,
    height: desiredContentHeight,
    resizable: gapp.resizable ?? true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      webSecurity: true,
      preload: path.join(__dirname, 'preload.js'),
    },
  });

  const appHtmlWithMCP = injectMCPClient(gapp);

  const startResponse = await startAgent({
    client,
    body: {
      working_dir: app.getPath('home'),
    },
    throwOnError: true,
  });

  const sessionId = startResponse.data.id;

  await resumeAgent({
    client,
    body: {
      session_id: sessionId,
      load_model_and_extensions: true,
    },
    throwOnError: true,
  });

  appContexts.set(appWindow.webContents.id, {
    gapp,
    html: appHtmlWithMCP,
    sessionId,
  });

  appContexts.set(appWindow.webContents.id, { gapp, html: appHtmlWithMCP, sessionId });

  appWindow.on('close', () => {
    appContexts.delete(appWindow.webContents.id);
  });

  const containerPath = app.isPackaged
    ? path.join(process.resourcesPath, 'goose_apps/container.html')
    : path.join(__dirname, '../../src/goose_apps/container.html');

  await appWindow.loadFile(containerPath);
  appWindow.setTitle(gapp.name);
  appWindow.setContentSize(gapp.width || 800, gapp.height || 600);

  appWindow.show();

  return appWindow;
}
