import { Page, _electron as electron } from '@playwright/test';

const isDebug = () => process.env.DEBUG_TESTS === '1' || process.env.DEBUG_TESTS === 'true';

export const debugLog = (message: string) => {
  if (isDebug()) {
    console.log(message);
  }
};

export function attachAppDebugLogs(electronApp: Awaited<ReturnType<typeof electron.launch>>) {
  const appProcess = electronApp.process();
  appProcess?.stdout?.on('data', (data) => {
    debugLog(`Electron stdout: ${data.toString()}`);
  });
  appProcess?.stderr?.on('data', (data) => {
    debugLog(`Electron stderr: ${data.toString()}`);
  });
}

export function attachPageDebugLogs(page: Page) {
  page.on('console', (msg) => {
    debugLog(`Renderer console [${msg.type()}]: ${msg.text()}`);
  });
  page.on('pageerror', (err) => {
    debugLog(`Renderer pageerror: ${err.message}`);
  });
  page.on('crash', () => {
    debugLog('Renderer crash event');
  });
  page.on('close', () => {
    debugLog('Renderer page close event');
  });
}
