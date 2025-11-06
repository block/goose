import { autoUpdater, UpdateInfo } from 'electron-updater';
import {
  BrowserWindow,
  ipcMain,
  nativeImage,
  Tray,
  shell,
  app,
  dialog,
  Menu,
  MenuItemConstructorOptions,
} from 'electron';
import * as path from 'path';
import * as fs from 'fs/promises';
import log from './logger';
import { githubUpdater } from './githubUpdater';
import { loadRecentDirs } from './recentDirs';

let updateAvailable = false;
let trayRef: Tray | null = null;
let isUsingGitHubFallback = false;
let githubUpdateInfo: {
  latestVersion?: string;
  downloadUrl?: string;
  releaseUrl?: string;
  downloadPath?: string;
  extractedPath?: string;
} = {};

// Store update state
let lastUpdateState: { updateAvailable: boolean; latestVersion?: string } | null = null;

// Track if IPC handlers have been registered
let ipcUpdateHandlersRegistered = false;

// Register IPC handlers (only once)
export function registerUpdateIpcHandlers() {
  if (ipcUpdateHandlersRegistered) {
    return;
  }

  log.info('Registering update IPC handlers...');
  ipcUpdateHandlersRegistered = true;

  // IPC handlers for renderer process
  ipcMain.handle('check-for-updates', async () => {
    try {
      log.info('Manual check for updates requested');

      // Reset fallback flag
      isUsingGitHubFallback = false;
      githubUpdateInfo = {};

      // Ensure auto-updater is properly initialized
      if (!autoUpdater.currentVersion) {
        log.error('Auto-updater currentVersion is null/undefined');
        throw new Error('Auto-updater not initialized. Please restart the application.');
      }

      log.info(
        `About to check for updates with currentVersion: ${JSON.stringify(autoUpdater.currentVersion)}`
      );
      log.info(`Feed URL: ${autoUpdater.getFeedURL()}`);

      const result = await autoUpdater.checkForUpdates();
      log.info('Auto-updater checkForUpdates result:', result);

      return {
        updateInfo: result?.updateInfo,
        error: null,
      };
    } catch (error) {
      log.error('Error checking for updates:', error);
      log.error('Manual check error details:', {
        message: error instanceof Error ? error.message : 'Unknown error',
        stack: error instanceof Error ? error.stack : 'No stack',
        name: error instanceof Error ? error.name : 'Unknown',
        code:
          error instanceof Error && 'code' in error
            ? (error as Error & { code: unknown }).code
            : undefined,
        toString: error?.toString(),
      });

      // If electron-updater fails, try GitHub API fallback
      if (
        error instanceof Error &&
        (error.message.includes('HttpError: 404') ||
          error.message.includes('ERR_CONNECTION_REFUSED') ||
          error.message.includes('ENOTFOUND') ||
          error.message.includes('No published versions'))
      ) {
        log.info('Using GitHub API fallback in check-for-updates...');
        log.info('Manual fallback triggered by error:', error.message);
        isUsingGitHubFallback = true;

        try {
          const result = await githubUpdater.checkForUpdates();

          if (result.error) {
            return {
              updateInfo: null,
              error: result.error,
            };
          }

          // Store GitHub update info
          if (result.updateAvailable) {
            githubUpdateInfo = {
              latestVersion: result.latestVersion,
              downloadUrl: result.downloadUrl,
              releaseUrl: result.releaseUrl,
            };

            updateAvailable = true;
            lastUpdateState = { updateAvailable: true, latestVersion: result.latestVersion };
            updateTrayIcon(true);
            sendStatusToWindow('update-available', { version: result.latestVersion });
          } else {
            updateAvailable = false;
            lastUpdateState = { updateAvailable: false };
            updateTrayIcon(false);
            sendStatusToWindow('update-not-available', {
              version: autoUpdater.currentVersion.version,
            });
          }

          return {
            updateInfo: null,
            error: null,
          };
        } catch (fallbackError) {
          log.error('GitHub fallback also failed:', fallbackError);
          return {
            updateInfo: null,
            error: 'Unable to check for updates. Please check your internet connection.',
          };
        }
      }

      return {
        updateInfo: null,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  });

  ipcMain.handle('download-update', async () => {
    try {
      if (isUsingGitHubFallback && githubUpdateInfo.downloadUrl && githubUpdateInfo.latestVersion) {
        log.info('Using GitHub fallback for download...');

        const result = await githubUpdater.downloadUpdate(
          githubUpdateInfo.downloadUrl,
          githubUpdateInfo.latestVersion,
          (percent) => {
            sendStatusToWindow('download-progress', { percent });
          }
        );

        if (result.success && result.downloadPath) {
          githubUpdateInfo.downloadPath = result.downloadPath;
          githubUpdateInfo.extractedPath = result.extractedPath;
          sendStatusToWindow('update-downloaded', { version: githubUpdateInfo.latestVersion });
          return { success: true, error: null };
        } else {
          throw new Error(result.error || 'Download failed');
        }
      } else {
        // Use electron-updater
        await autoUpdater.downloadUpdate();
        return { success: true, error: null };
      }
    } catch (error) {
      log.error('Error downloading update:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  });

  ipcMain.handle('install-update', async () => {
    if (isUsingGitHubFallback) {
      // For GitHub fallback, we need to handle the installation differently
      log.info('Installing update from GitHub fallback...');

      try {
        // Use the stored extracted path if available, otherwise download path
        const updatePath = githubUpdateInfo.extractedPath || githubUpdateInfo.downloadPath;

        if (!updatePath) {
          throw new Error('Update file path not found. Please download the update first.');
        }

        // Check if the update path exists
        try {
          await fs.access(updatePath);
        } catch {
          throw new Error('Update file not found. Please download the update first.');
        }

        // Show dialog to inform user about manual installation
        const dialogResult = (await dialog.showMessageBox({
          type: 'info',
          title: 'Update Downloaded',
          message: 'The update has been downloaded to your Downloads folder.',
          detail: `Please extract the zip file and move the Goose app to your Applications folder to complete the update.`,
          buttons: ['Open Downloads', 'Cancel'],
          defaultId: 0,
          cancelId: 1,
        })) as unknown as { response: number };

        if (dialogResult.response === 0) {
          // Open the extracted folder or show the zip file
          shell.showItemInFolder(updatePath);

          // Optionally quit the app so user can replace it
          setTimeout(() => {
            app.quit();
          }, 1000);
        }
      } catch (error) {
        log.error('Error installing GitHub update:', error);
        throw error;
      }
    } else {
      // Use electron-updater's built-in install
      autoUpdater.quitAndInstall(false, true);
    }
  });

  ipcMain.handle('get-current-version', () => {
    return autoUpdater.currentVersion.version;
  });

  ipcMain.handle('get-update-state', () => {
    return lastUpdateState;
  });
}

// Configure auto-updater
export function setupAutoUpdater(tray?: Tray) {
  if (tray) {
    trayRef = tray;
  }

  log.info('Setting up auto-updater...');
  log.info(`Current app version: ${app.getVersion()}`);
  log.info(`Platform: ${process.platform}, Arch: ${process.arch}`);
  log.info(`ENABLE_DEV_UPDATES: ${process.env.ENABLE_DEV_UPDATES}`);
  log.info(`App is packaged: ${app.isPackaged}`);
  log.info(`App path: ${app.getAppPath()}`);
  log.info(`Resources path: ${process.resourcesPath}`);

  // Set the feed URL for GitHub releases
  const feedConfig = {
    provider: 'github' as const,
    owner: 'block',
    repo: 'goose',
    releaseType: 'release' as const,
  };

  log.info('Setting feed URL with config:', feedConfig);
  autoUpdater.setFeedURL(feedConfig);

  // Log the feed URL after setting it
  try {
    const feedUrl = autoUpdater.getFeedURL();
    log.info(`Feed URL set to: ${feedUrl}`);
  } catch (e) {
    log.error('Error getting feed URL:', e);
  }

  // Configure auto-updater settings
  autoUpdater.autoDownload = false; // We'll trigger downloads manually
  autoUpdater.autoInstallOnAppQuit = true;

  // Enable updates in development mode for testing
  if (process.env.ENABLE_DEV_UPDATES === 'true') {
    log.info('Enabling dev updates config');
    autoUpdater.forceDevUpdateConfig = true;
  }

  // Additional debugging for release builds
  if (app.isPackaged) {
    log.info('App is packaged - this is a release build');
    // Try to get more info about the updater configuration
    try {
      log.info(`Auto-updater channel: ${autoUpdater.channel}`);
      log.info(`Auto-updater allowPrerelease: ${autoUpdater.allowPrerelease}`);
      log.info(`Auto-updater allowDowngrade: ${autoUpdater.allowDowngrade}`);
    } catch (e) {
      log.error('Error getting auto-updater properties:', e);
    }
  } else {
    log.info('App is not packaged - this is a development build');
  }

  // Set logger
  autoUpdater.logger = log;

  log.info('Auto-updater setup completed');

  // Check for updates on startup
  setTimeout(() => {
    log.info('Checking for updates on startup...');
    log.info(`autoUpdater.currentVersion: ${JSON.stringify(autoUpdater.currentVersion)}`);
    log.info(`autoUpdater.getFeedURL(): ${autoUpdater.getFeedURL()}`);

    autoUpdater.checkForUpdates().catch((err) => {
      log.error('Error checking for updates on startup:', err);
      log.error('Error details:', {
        message: err.message,
        stack: err.stack,
        name: err.name,
        code: 'code' in err ? err.code : undefined,
      });

      // If electron-updater fails, try GitHub API as fallback
      if (
        err.message.includes('HttpError: 404') ||
        err.message.includes('ERR_CONNECTION_REFUSED') ||
        err.message.includes('ENOTFOUND') ||
        err.message.includes('No published versions')
      ) {
        log.info('Using GitHub API fallback for startup update check...');
        log.info('Fallback triggered by error containing:', err.message);
        isUsingGitHubFallback = true;

        githubUpdater
          .checkForUpdates()
          .then((result) => {
            if (result.error) {
              sendStatusToWindow('error', result.error);
            } else if (result.updateAvailable) {
              // Store GitHub update info
              githubUpdateInfo = {
                latestVersion: result.latestVersion,
                downloadUrl: result.downloadUrl,
                releaseUrl: result.releaseUrl,
              };

              updateAvailable = true;
              lastUpdateState = { updateAvailable: true, latestVersion: result.latestVersion };
              updateTrayIcon(true);
              sendStatusToWindow('update-available', { version: result.latestVersion });
            } else {
              updateAvailable = false;
              lastUpdateState = { updateAvailable: false };
              updateTrayIcon(false);
              sendStatusToWindow('update-not-available', {
                version: autoUpdater.currentVersion.version,
              });
            }
          })
          .catch((fallbackError) => {
            log.error('GitHub fallback also failed on startup:', fallbackError);
          });
      }
    });
  }, 5000); // Wait 5 seconds after app starts

  // Handle update events
  autoUpdater.on('checking-for-update', () => {
    log.info('Auto-updater: Checking for update...');
    log.info(`Auto-updater: Feed URL during check: ${autoUpdater.getFeedURL()}`);
    sendStatusToWindow('checking-for-update');
  });

  autoUpdater.on('update-available', (info: UpdateInfo) => {
    log.info('Update available:', info);
    updateAvailable = true;
    lastUpdateState = { updateAvailable: true, latestVersion: info.version };
    updateTrayIcon(true);
    sendStatusToWindow('update-available', info);
  });

  autoUpdater.on('update-not-available', (info: UpdateInfo) => {
    log.info('Update not available:', info);
    updateAvailable = false;
    lastUpdateState = { updateAvailable: false };
    updateTrayIcon(false);
    sendStatusToWindow('update-not-available', info);
  });

  autoUpdater.on('error', async (err) => {
    log.error('Error in auto-updater:', err);
    log.error('Auto-updater error details:', {
      message: err.message,
      stack: err.stack,
      name: err.name,
      code: 'code' in err ? err.code : undefined,
      toString: err.toString(),
    });

    // Check if this is a 404 error (missing update files) or connection error
    if (
      err.message.includes('HttpError: 404') ||
      err.message.includes('ERR_CONNECTION_REFUSED') ||
      err.message.includes('ENOTFOUND') ||
      err.message.includes('No published versions')
    ) {
      log.info('Falling back to GitHub API for update check...');
      log.info('Fallback triggered by error:', err.message);
      isUsingGitHubFallback = true;

      try {
        const result = await githubUpdater.checkForUpdates();

        if (result.error) {
          sendStatusToWindow('error', result.error);
        } else if (result.updateAvailable) {
          // Store GitHub update info
          githubUpdateInfo = {
            latestVersion: result.latestVersion,
            downloadUrl: result.downloadUrl,
            releaseUrl: result.releaseUrl,
          };

          updateAvailable = true;
          updateTrayIcon(true);
          sendStatusToWindow('update-available', { version: result.latestVersion });
        } else {
          updateAvailable = false;
          updateTrayIcon(false);
          sendStatusToWindow('update-not-available', {
            version: autoUpdater.currentVersion.version,
          });
        }
      } catch (fallbackError) {
        log.error('GitHub fallback also failed:', fallbackError);
        sendStatusToWindow(
          'error',
          'Unable to check for updates. Please check your internet connection.'
        );
      }
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

  const isDev = !app.isPackaged;
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

  // Update tray menu when icon changes
  updateTrayMenu(hasUpdate);
}

// Function to open settings and scroll to update section
function openUpdateSettings() {
  const windows = BrowserWindow.getAllWindows();
  if (windows.length > 0) {
    const mainWindow = windows[0];
    mainWindow.show();
    mainWindow.focus();
    // Send message to open settings and scroll to update section
    mainWindow.webContents.send('set-view', 'settings', 'update');
  }
}

// Export function to update tray menu
export function updateTrayMenu(hasUpdate: boolean) {
  if (!trayRef) return;

  const menuItems: MenuItemConstructorOptions[] = [];

  // Add update menu item if update is available
  if (hasUpdate) {
    menuItems.push({
      label: 'Update Available...',
      click: openUpdateSettings,
    });
  }

  menuItems.push(
    {
      label: 'Show Window',
      click: async () => {
        const windows = BrowserWindow.getAllWindows();
        if (windows.length === 0) {
          log.info('No windows are open, creating a new one...');
          // Get recent directories for the new window
          const recentDirs = loadRecentDirs();
          const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

          // Emit event to create new window (handled in main.ts)
          ipcMain.emit('create-chat-window', {}, undefined, openDir);
          return;
        }

        // Show all windows with offset
        const initialOffsetX = 30;
        const initialOffsetY = 30;

        windows.forEach((win: BrowserWindow, index: number) => {
          const currentBounds = win.getBounds();
          const newX = currentBounds.x + initialOffsetX * index;
          const newY = currentBounds.y + initialOffsetY * index;

          win.setBounds({
            x: newX,
            y: newY,
            width: currentBounds.width,
            height: currentBounds.height,
          });

          if (!win.isVisible()) {
            win.show();
          }

          win.focus();
        });
      },
    },
    { type: 'separator' },
    { label: 'Quit', click: () => app.quit() }
  );

  const contextMenu = Menu.buildFromTemplate(menuItems);
  trayRef.setContextMenu(contextMenu);
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
