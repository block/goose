import { BrowserWindow } from 'electron';
import path from 'node:path';

declare const MAIN_WINDOW_VITE_DEV_SERVER_URL: string;
declare const MAIN_WINDOW_VITE_NAME: string;

let featureFlagsWindow: BrowserWindow | null = null;

export const createFeatureFlagsWindow = () => {
  // Don't create multiple windows
  if (featureFlagsWindow) {
    featureFlagsWindow.focus();
    return;
  }

  featureFlagsWindow = new BrowserWindow({
    width: 800,
    height: 700,
    title: 'Feature Flags',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
    },
    titleBarStyle: 'hidden',
    trafficLightPosition: { x: 20, y: 20 },
  });

  const launcherParams = '?window=featureFlags';
  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    featureFlagsWindow.loadURL(`${MAIN_WINDOW_VITE_DEV_SERVER_URL}${launcherParams}`);
  } else {
    featureFlagsWindow.loadFile(
      path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html${launcherParams}`)
    );
  }

  featureFlagsWindow.on('closed', () => {
    featureFlagsWindow = null;
  });

  // Log any load failures
  featureFlagsWindow.webContents.on('did-fail-load', (event, errorCode, errorDescription) => {
    console.error('Failed to load feature flags window:', errorCode, errorDescription);
  });
};