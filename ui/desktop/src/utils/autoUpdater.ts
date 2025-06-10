import { autoUpdater, UpdateInfo } from 'electron-updater';
import { BrowserWindow, ipcMain } from 'electron';
import log from './logger';

// Configure auto-updater
export function setupAutoUpdater() {
  // Set the feed URL for GitHub releases
  autoUpdater.setFeedURL({
    provider: 'github',
    owner: 'block',
    repo: 'goose',
    releaseType: 'release'
  });

  // Configure auto-updater settings
  autoUpdater.autoDownload = false; // We'll trigger downloads manually
  autoUpdater.autoInstallOnAppQuit = true;
  
  // Set logger
  autoUpdater.logger = log;

  // Handle update events
  autoUpdater.on('checking-for-update', () => {
    log.info('Checking for update...');
    sendStatusToWindow('checking-for-update');
  });

  autoUpdater.on('update-available', (info: UpdateInfo) => {
    log.info('Update available:', info);
    sendStatusToWindow('update-available', info);
  });

  autoUpdater.on('update-not-available', (info: UpdateInfo) => {
    log.info('Update not available:', info);
    sendStatusToWindow('update-not-available', info);
  });

  autoUpdater.on('error', (err) => {
    log.error('Error in auto-updater:', err);
    // Handle connection errors more gracefully
    if (err.message.includes('ERR_CONNECTION_REFUSED') || err.message.includes('ENOTFOUND')) {
      sendStatusToWindow('error', 'Unable to check for updates. Please check your internet connection.');
    } else if (err.message.includes('HttpError: 404')) {
      // When no releases are found, assume current version is up to date
      sendStatusToWindow('update-not-available', { version: autoUpdater.currentVersion.version });
    } else {
      sendStatusToWindow('error', err.message);
    }
  });

  autoUpdater.on('download-progress', (progressObj) => {
    let log_message = 'Download speed: ' + progressObj.bytesPerSecond;
    log_message = log_message + ' - Downloaded ' + progressObj.percent + '%';
    log_message = log_message + ' (' + progressObj.transferred + '/' + progressObj.total + ')';
    log.info(log_message);
    sendStatusToWindow('download-progress', progressObj);
  });

  autoUpdater.on('update-downloaded', (info: UpdateInfo) => {
    log.info('Update downloaded:', info);
    sendStatusToWindow('update-downloaded', info);
  });

  // IPC handlers for renderer process
  ipcMain.handle('check-for-updates', async () => {
    try {
      // Ensure auto-updater is properly initialized
      if (!autoUpdater.currentVersion) {
        throw new Error('Auto-updater not initialized. Please restart the application.');
      }
      
      const result = await autoUpdater.checkForUpdates();
      return {
        updateInfo: result?.updateInfo,
        error: null
      };
    } catch (error) {
      log.error('Error checking for updates:', error);
      let errorMessage = 'Unknown error';
      
      if (error instanceof Error) {
        if (error.message.includes('ERR_CONNECTION_REFUSED') || error.message.includes('ENOTFOUND')) {
          errorMessage = 'Unable to check for updates. Please check your internet connection.';
        } else if (error.message.includes('HttpError: 404')) {
          // When no releases are found, treat as up to date
          // This will trigger the update-not-available event
          sendStatusToWindow('update-not-available', { version: autoUpdater.currentVersion.version });
          return {
            updateInfo: null,
            error: null
          };
        } else {
          errorMessage = error.message;
        }
      }
      
      return {
        updateInfo: null,
        error: errorMessage
      };
    }
  });

  ipcMain.handle('download-update', async () => {
    try {
      await autoUpdater.downloadUpdate();
      return { success: true, error: null };
    } catch (error) {
      log.error('Error downloading update:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  });

  ipcMain.handle('install-update', () => {
    autoUpdater.quitAndInstall(false, true);
  });

  ipcMain.handle('get-current-version', () => {
    return autoUpdater.currentVersion.version;
  });
}

interface UpdaterEvent {
  event: string;
  data?: unknown;
}

function sendStatusToWindow(event: string, data?: unknown) {
  const windows = BrowserWindow.getAllWindows();
  windows.forEach((win) => {
    win.webContents.send('updater-event', { event, data } as UpdaterEvent);
  });
}