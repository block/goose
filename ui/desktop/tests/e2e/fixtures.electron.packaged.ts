import { test as base, Page, _electron as electron } from '@playwright/test';
import * as fs from 'fs';
import { join } from 'path';

import { debugLog, attachAppDebugLogs, attachPageDebugLogs } from './helpers/debug-log';
import { withVisualDelayPage } from './helpers/visual-delay';
import { isVideoRecording, enableCursorHighlight, trimVideosInDirectory } from './helpers/video';
import {
  createIsolatedGoosePathRoot,
  buildLaunchOptions,
  waitForRootWindow,
  closeElectronApp,
} from './helpers/electron-launch';

type GooseTestFixtures = {
  goosePage: Page;
};

export const test = base.extend<GooseTestFixtures>({
  goosePage: async ({}, use, testInfo) => {
    testInfo.setTimeout(Math.max(testInfo.timeout, 120000));

    let electronApp: Awaited<ReturnType<typeof electron.launch>> | null = null;
    let page: Page | null = null;
    let videoDir: string | undefined;
    let videoTrimStartMs = 0;
    const tempDir = createIsolatedGoosePathRoot();

    try {
      videoDir = isVideoRecording() ? testInfo.outputPath('videos') : undefined;
      const launchOptions = buildLaunchOptions(tempDir, videoDir);
      debugLog(`Launching direct Electron for test: ${testInfo.title}`);

      electronApp = await electron.launch(launchOptions);
      await electronApp.evaluate(({ ipcMain }, chooserDir) => {
        ipcMain.removeHandler('directory-chooser');
        ipcMain.handle('directory-chooser', async () => ({
          canceled: false,
          filePaths: [chooserDir],
        }));
      }, tempDir);
      attachAppDebugLogs(electronApp);

      const recordingStartMs = Date.now();
      const rootWindow = await waitForRootWindow(electronApp, 30000).catch(async () => {
        debugLog('Root-ready window not found quickly; falling back to first window.');
        return await electronApp!.firstWindow({ timeout: 5000 });
      });
      page = rootWindow;
      debugLog(`Selected app window URL: ${page.url()}`);
      attachPageDebugLogs(page);
      const rootReadyElapsedMs = Date.now() - recordingStartMs;
      if (isVideoRecording()) {
        await enableCursorHighlight(page);
        page.on('domcontentloaded', async () => {
          await enableCursorHighlight(page!);
        });
      }
      videoTrimStartMs = Math.max(0, rootReadyElapsedMs - 300);
      await use(withVisualDelayPage(page));
    } finally {
      if (electronApp) {
        await closeElectronApp(electronApp);
      }

      if (videoDir && videoTrimStartMs > 0) {
        await trimVideosInDirectory(videoDir, videoTrimStartMs);
      }

      if (videoDir && fs.existsSync(videoDir)) {
        for (const file of fs.readdirSync(videoDir).filter(f => f.endsWith('.webm'))) {
          await testInfo.attach('video', {
            path: join(videoDir, file),
            contentType: 'video/webm',
          });
        }
      }

      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  },
});

