import { autoUpdater, UpdateInfo } from 'electron-updater';
import { BrowserWindow, ipcMain, nativeImage, Tray } from 'electron';
import * as path from 'path';
import log from './logger';

let updateAvailable = false;
let trayRef: Tray | null = null;

// Configure auto-updater
export function setupAutoUpdater(tray?: Tray) {
  if (tray) {
    trayRef = tray;
  }
  
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
  
  // Check for updates on startup
  setTimeout(() => {
    log.info('Checking for updates on startup...');
    autoUpdater.checkForUpdates().catch(err => {
      log.error('Error checking for updates on startup:', err);
    });
  }, 5000); // Wait 5 seconds after app starts

  // Handle update events
  autoUpdater.on('checking-for-update', () => {
    log.info('Checking for update...');
    sendStatusToWindow('checking-for-update');
  });

  autoUpdater.on('update-available', (info: UpdateInfo) => {
    log.info('Update available:', info);
    updateAvailable = true;
    updateTrayIcon(true);
    sendStatusToWindow('update-available', info);
  });

  autoUpdater.on('update-not-available', (info: UpdateInfo) => {
    log.info('Update not available:', info);
    updateAvailable = false;
    updateTrayIcon(false);
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

function updateTrayIcon(hasUpdate: boolean) {
  if (!trayRef) return;
  
  const isDev = process.env.NODE_ENV === 'development';
  let iconPath: string;
  
  if (hasUpdate) {
    // Use icon with update indicator
    if (isDev) {
      iconPath = path.join(process.cwd(), 'src', 'images', 'iconTemplateUpdate.png');
    } else {
      iconPath = path.join(process.resourcesPath, 'images', 'iconTemplateUpdate.png');
    }
    trayRef.setToolTip('Goose - Update Available');
  } else {
    // Use normal icon
    if (isDev) {
      iconPath = path.join(process.cwd(), 'src', 'images', 'iconTemplate.png');
    } else {
      iconPath = path.join(process.resourcesPath, 'images', 'iconTemplate.png');
    }
    trayRef.setToolTip('Goose');
  }
  
  const icon = nativeImage.createFromPath(iconPath);
  if (process.platform === 'darwin') {
    // Mark as template for macOS to handle dark/light mode
    icon.setTemplateImage(true);
  }
  trayRef.setImage(icon);
}

// Export functions to manage tray reference
export function setTrayRef(tray: Tray) {
  trayRef = tray;
  // Update icon based on current update status
  updateTrayIcon(updateAvailable);
}

export function getUpdateAvailable(): boolean {
  return updateAvailable;
}