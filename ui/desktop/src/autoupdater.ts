import { app, BrowserWindow, ipcMain, IpcMainEvent } from 'electron';
import { autoUpdater } from 'electron-updater';
import log from './utils/logger';
import { UPDATES_ENABLED } from './updates';

const IPC_UPDATE_DOWNLOADED = 'update-downloaded';
const IPC_INSTALL_UPDATE = 'install-update';

interface UpdaterLogger {
  info: (...args: unknown[]) => void;
  warn: (...args: unknown[]) => void;
  error: (...args: unknown[]) => void;
}

export function setupAutoUpdater(mainWindow: BrowserWindow | null) {
  // Only run on macOS packaged apps
  if (process.platform !== 'darwin') {
    log.info('[AutoUpdater] Skipping setup: not macOS');
    return;
  }
  if (!app.isPackaged) {
    log.info('[AutoUpdater] Skipping setup: app is not packaged');
    return;
  }

  log.info('[AutoUpdater] Initializing updater');

  try {
    // Configure logger for electron-updater
    const updaterLogger: UpdaterLogger = {
      info: (...args: unknown[]) => log.info(...(args as [unknown])),
      warn: (...args: unknown[]) => log.warn(...(args as [unknown])),
      error: (...args: unknown[]) => log.error(...(args as [unknown])),
    };
    autoUpdater.logger = updaterLogger as unknown as UpdaterLogger;
    autoUpdater.autoDownload = !!UPDATES_ENABLED; // Only auto-download when flag enabled

    autoUpdater.on('checking-for-update', () => {
      log.info('[AutoUpdater] Checking for update...');
    });

    autoUpdater.on('update-available', (info) => {
      log.info('[AutoUpdater] Update available:', info.version);
    });

    autoUpdater.on('update-not-available', (_info) => {
      log.info('[AutoUpdater] No update available');
    });

    autoUpdater.on('error', (err) => {
      log.error('[AutoUpdater] Error in auto-updater:', err);
    });

    autoUpdater.on('download-progress', (progressObj) => {
      const percent = Math.round(progressObj.percent || 0);
      log.info(`[AutoUpdater] Download progress: ${percent}%`);
    });

    // When downloaded, do not immediately quit. Inform renderer so it can show a compact restart button
    autoUpdater.on('update-downloaded', (info) => {
      log.info('[AutoUpdater] Update downloaded; notifying renderer to show restart button');
      try {
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send(IPC_UPDATE_DOWNLOADED, { version: info.version });
        } else {
          log.warn('[AutoUpdater] No main window available to notify about downloaded update');
        }
      } catch (sendErr) {
        log.error('[AutoUpdater] Failed to send IPC update-downloaded:', sendErr);
      }
    });

    // Listen for renderer request to install
    ipcMain.on(IPC_INSTALL_UPDATE, (_event: IpcMainEvent) => {
      log.info('[AutoUpdater] Received install request from renderer; calling quitAndInstall');
      try {
        // quitAndInstall will restart and install the update
        autoUpdater.quitAndInstall();
      } catch (qeiErr) {
        log.error('[AutoUpdater] quitAndInstall error:', qeiErr);
      }
    });

    // Kick off a check for updates
    log.info('[AutoUpdater] Starting checkForUpdatesAndNotify');
    autoUpdater.checkForUpdates().catch((err) => {
      log.error('[AutoUpdater] checkForUpdates error:', err);
    });
  } catch (err) {
    log.error('[AutoUpdater] Failed to initialize updater:', err);
  }
}

export { IPC_UPDATE_DOWNLOADED, IPC_INSTALL_UPDATE };
