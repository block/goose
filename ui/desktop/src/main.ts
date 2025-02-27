import { spawn } from 'child_process';
import 'dotenv/config';
import {
  app,
  BrowserWindow,
  dialog,
  globalShortcut,
  ipcMain,
  Menu,
  MenuItem,
  Notification,
  powerSaveBlocker,
  Tray,
} from 'electron';
import started from 'electron-squirrel-startup';
import path from 'node:path';
import { startGoosed } from './goosed';
import { getBinaryPath } from './utils/binaryPath';
import { loadShellEnv } from './utils/loadEnv';
import log from './utils/logger';
import { addRecentDir, loadRecentDirs } from './utils/recentDirs';
import {
  createEnvironmentMenu,
  EnvToggles,
  loadSettings,
  saveSettings,
  updateEnvironmentVariables,
} from './utils/settings';
import * as crypto from 'crypto';
import * as electron from 'electron';
import { exec as execCallback } from 'child_process';
import { promisify } from 'util';

const exec = promisify(execCallback);

if (started) app.quit();

app.setAsDefaultProtocolClient('goose');

// Triggered when the user opens "goose://..." links
app.on('open-url', async (event, url) => {
  event.preventDefault();

  // Get existing window or create new one
  let targetWindow: BrowserWindow;
  const existingWindows = BrowserWindow.getAllWindows();

  if (existingWindows.length > 0) {
    targetWindow = existingWindows[0];
    if (targetWindow.isMinimized()) targetWindow.restore();
    targetWindow.focus();
  } else {
    const recentDirs = loadRecentDirs();
    const openDir = recentDirs.length > 0 ? recentDirs[0] : null;
    targetWindow = await createChat(app, undefined, openDir);
  }

  // Wait for window to be ready before sending the extension URL
  if (!targetWindow.webContents.isLoading()) {
    targetWindow.webContents.send('add-extension', url);
  } else {
    targetWindow.webContents.once('did-finish-load', () => {
      targetWindow.webContents.send('add-extension', url);
    });
  }
});

declare var MAIN_WINDOW_VITE_DEV_SERVER_URL: string;
declare var MAIN_WINDOW_VITE_NAME: string;

// State for environment variable toggles
let envToggles: EnvToggles = loadSettings().envToggles;

// Parse command line arguments
const parseArgs = () => {
  const args = process.argv.slice(2); // Remove first two elements (electron and script path)
  let dirPath = null;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--dir' && i + 1 < args.length) {
      dirPath = args[i + 1];
      break;
    }
  }

  return { dirPath };
};

const getGooseProvider = () => {
  loadShellEnv(app.isPackaged);
  //{env-macro-start}//
  //needed when goose is bundled for a specific provider
  //{env-macro-end}//
  return [process.env.GOOSE_PROVIDER, process.env.GOOSE_MODEL];
};

const generateSecretKey = () => {
  const key = crypto.randomBytes(32).toString('hex');
  process.env.GOOSE_SERVER__SECRET_KEY = key;
  return key;
};

let [provider, model] = getGooseProvider();

let appConfig = {
  GOOSE_PROVIDER: provider,
  GOOSE_MODEL: model,
  GOOSE_API_HOST: 'http://127.0.0.1',
  GOOSE_PORT: 0,
  GOOSE_WORKING_DIR: '',
  secretKey: generateSecretKey(),
};

// Track windows by ID
let windowCounter = 0;
const windowMap = new Map<number, BrowserWindow>();

const createChat = async (app, query?: string, dir?: string, version?: string) => {
  // Apply current environment settings before creating chat
  updateEnvironmentVariables(envToggles);

  const [port, working_dir, goosedProcess] = await startGoosed(app, dir);

  const mainWindow = new BrowserWindow({
    titleBarStyle: process.platform === 'darwin' ? 'hidden' : 'default',
    trafficLightPosition: process.platform === 'darwin' ? { x: 16, y: 10 } : undefined,
    vibrancy: process.platform === 'darwin' ? 'window' : undefined,
    frame: process.platform === 'darwin' ? false : true,
    width: 750,
    height: 800,
    minWidth: 650,
    resizable: true,
    transparent: false,
    useContentSize: true,
    icon: path.join(__dirname, '../images/icon'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      additionalArguments: [
        JSON.stringify({
          ...appConfig,
          GOOSE_PORT: port,
          GOOSE_WORKING_DIR: working_dir,
          REQUEST_DIR: dir,
        }),
      ],
      partition: 'persist:goose', // Add this line to ensure persistence
    },
  });

  // Handle new window creation for links
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    // Open all links in external browser
    if (url.startsWith('http:') || url.startsWith('https:')) {
      require('electron').shell.openExternal(url);
      return { action: 'deny' };
    }
    return { action: 'allow' };
  });

  // Load the index.html of the app.
  const queryParam = query ? `?initialQuery=${encodeURIComponent(query)}` : '';
  const primaryDisplay = electron.screen.getPrimaryDisplay();
  const { width } = primaryDisplay.workAreaSize;

  // Increment window counter to track number of windows
  const windowId = ++windowCounter;
  const direction = windowId % 2 === 0 ? 1 : -1; // Alternate direction
  const initialOffset = 50;

  // Set window position with alternating offset strategy
  const baseXPosition = Math.round(width / 2 - mainWindow.getSize()[0] / 2);
  const xOffset = direction * initialOffset * Math.floor(windowId / 2);
  mainWindow.setPosition(baseXPosition + xOffset, 100);

  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(`${MAIN_WINDOW_VITE_DEV_SERVER_URL}${queryParam}`);
  } else {
    // In production, we need to use a proper file protocol URL with correct base path
    const indexPath = path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`);
    console.log('Loading production path:', indexPath);
    mainWindow.loadFile(indexPath, {
      search: queryParam ? queryParam.slice(1) : undefined,
    });
  }

  // DevTools shortcut management
  const registerDevToolsShortcut = (window: BrowserWindow) => {
    globalShortcut.register('Alt+Command+I', () => {
      window.webContents.openDevTools();
    });
  };

  const unregisterDevToolsShortcut = () => {
    globalShortcut.unregister('Alt+Command+I');
  };

  // Install MCP Extension shortcut
  const registerMCPExtensionsShortcut = () => {
    globalShortcut.register('Shift+Command+Y', () => {
      const defaultUrl =
          'goose://extension?cmd=npx&arg=-y&arg=%40modelcontextprotocol%2Fserver-github&id=github&name=GitHub&description=Repository%20management%2C%20file%20operations%2C%20and%20GitHub%20API%20integration&env=GITHUB_TOKEN%3DGitHub%20personal%20access%20token';

      const result = dialog.showMessageBoxSync({
        type: 'question',
        buttons: ['Install', 'Edit URL', 'Cancel'],
        defaultId: 0,
        cancelId: 2,
        title: 'Install MCP Extension',
        message: 'Install MCP Extension',
        detail: `Current extension URL:\n\n${defaultUrl}`,
      });

      if (result === 0) {
        // User clicked Install
        const mockEvent = {
          preventDefault: () => {
            console.log('Default handling prevented.');
          },
        };
        app.emit('open-url', mockEvent, defaultUrl);
      } else if (result === 1) {
        // User clicked Edit URL
        // Create a simple input dialog
        const win = new BrowserWindow({
          width: 800,
          height: 120,
          frame: false,
          transparent: false,
          resizable: false,
          minimizable: false,
          maximizable: false,
          parent: BrowserWindow.getFocusedWindow(),
          modal: true,
          show: false,
          webPreferences: {
            nodeIntegration: true,
            contextIsolation: false,
          },
        });

        win.loadURL(`data:text/html,
        <html>
          <body style="margin: 20px; font-family: system-ui;">
            <input type="text" id="url" value="${defaultUrl}" style="width: 100%; padding: 8px; margin-bottom: 10px;">
            <div style="text-align: right;">
              <button onclick="window.close()" style="margin-right: 10px;">Cancel</button>
              <button onclick="submit()" style="min-width: 80px;">Install</button>
            </div>
            <script>
              function submit() {
                require('electron').ipcRenderer.send('install-extension-url', document.getElementById('url').value);
              }
              // Handle Enter key
              document.getElementById('url').addEventListener('keypress', (e) => {
                if (e.key === 'Enter') submit();
              });
              // Focus the input
              document.getElementById('url').focus();
              document.getElementById('url').select();
            </script>
          </body>
        </html>
      `);

        win.once('ready-to-show', () => {
          win.show();
        });

        // Handle the URL submission
        ipcMain.once('install-extension-url', (event, url) => {
          win.close();
          const mockEvent = {
            preventDefault: () => {
              console.log('Default handling prevented.');
            },
          };
          if (url && url.trim()) {
            app.emit('open-url', mockEvent, url);
          }
        });
      }
    });
  };

  const unRegisterMCPExtensionsShortcut = () => {
    globalShortcut.unregister('Shift+Command+Y');
  };

  // Register shortcuts when window is focused
  mainWindow.on('focus', () => {
    registerDevToolsShortcut(mainWindow);
    registerMCPExtensionsShortcut();
    // Register reload shortcut
    globalShortcut.register('CommandOrControl+R', () => {
      mainWindow.reload();
    });
  });

  // Unregister shortcuts when window loses focus
  mainWindow.on('blur', () => {
    unregisterDevToolsShortcut();
    unRegisterMCPExtensionsShortcut();
    globalShortcut.unregister('CommandOrControl+R');
  });

  windowMap.set(windowId, mainWindow);
  mainWindow.on('closed', () => {
    windowMap.delete(windowId);
    unregisterDevToolsShortcut();
    goosedProcess.kill();
  });
  return mainWindow;
};

const createTray = () => {
  const isDev = process.env.NODE_ENV === 'development';
  let iconPath: string;

  if (isDev) {
    iconPath = path.join(process.cwd(), 'src', 'images', 'iconTemplate.png');
  } else {
    iconPath = path.join(process.resourcesPath, 'images', 'iconTemplate.png');
  }

  const tray = new Tray(iconPath);

  const contextMenu = Menu.buildFromTemplate([
    { label: 'Show Window', click: showWindow },
    { type: 'separator' },
    { label: 'Quit', click: () => app.quit() },
  ]);

  tray.setToolTip('Goose');
  tray.setContextMenu(contextMenu);
};

const showWindow = () => {
  const windows = BrowserWindow.getAllWindows();

  if (windows.length === 0) {
    log.info('No windows are currently open.');
    return;
  }

  // Define the initial offset values
  const initialOffsetX = 30;
  const initialOffsetY = 30;

  // Iterate over all windows
  windows.forEach((win, index) => {
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
};

const buildRecentFilesMenu = () => {
  const recentDirs = loadRecentDirs();
  return recentDirs.map((dir) => ({
    label: dir,
    click: () => {
      createChat(app, undefined, dir);
    },
  }));
};

const openDirectoryDialog = async (replaceWindow: boolean = false) => {
  const result = await dialog.showOpenDialog({
    properties: ['openDirectory'],
  });

  if (!result.canceled && result.filePaths.length > 0) {
    addRecentDir(result.filePaths[0]);
    if (replaceWindow) {
      BrowserWindow.getFocusedWindow().close();
    }
    createChat(app, undefined, result.filePaths[0]);
  }
};

// Global error handler
const handleFatalError = (error: Error) => {
  const windows = BrowserWindow.getAllWindows();
  windows.forEach((win) => {
    win.webContents.send('fatal-error', error.message || 'An unexpected error occurred');
  });
};

process.on('uncaughtException', (error) => {
  console.error('Uncaught Exception:', error);
  handleFatalError(error);
});

process.on('unhandledRejection', (error) => {
  console.error('Unhandled Rejection:', error);
  handleFatalError(error instanceof Error ? error : new Error(String(error)));
});

// Add file/directory selection handler
ipcMain.handle('select-file-or-directory', async () => {
  const result = await dialog.showOpenDialog({
    properties: ['openFile', 'openDirectory'],
  });

  if (!result.canceled && result.filePaths.length > 0) {
    return result.filePaths[0];
  }
  return null;
});

ipcMain.handle('check-ollama', async () => {
  try {
    return new Promise((resolve) => {
      // Run `ps` and filter for "ollama"
      exec('ps aux | grep -iw "[o]llama"', (error, stdout, stderr) => {
        if (error) {
          console.error('Error executing ps command:', error);
          return resolve(false); // Process is not running
        }

        if (stderr) {
          console.error('Standard error output from ps command:', stderr);
          return resolve(false); // Process is not running
        }

        console.log('Raw stdout from ps command:', stdout);

        // Trim and check if output contains a match
        const trimmedOutput = stdout.trim();
        console.log('Trimmed stdout:', trimmedOutput);

        const isRunning = trimmedOutput.length > 0; // True if there's any output
        resolve(isRunning); // Resolve true if running, false otherwise
      });
    });
  } catch (err) {
    console.error('Error checking for Ollama:', err);
    return false; // Return false on error
  }
});

app.whenReady().then(async () => {
  // Test error feature - only enabled with GOOSE_TEST_ERROR=true
  if (process.env.GOOSE_TEST_ERROR === 'true') {
    console.log('Test error feature enabled, will throw error in 5 seconds');
    setTimeout(() => {
      console.log('Throwing test error now...');
      throw new Error('Test error: This is a simulated fatal error after 5 seconds');
    }, 5000);
  }

  // Parse command line arguments
  const { dirPath } = parseArgs();

  createTray();
  const recentDirs = loadRecentDirs();
  let openDir = dirPath || (recentDirs.length > 0 ? recentDirs[0] : null);
  createChat(app, undefined, openDir);

  // Get the existing menu
  const menu = Menu.getApplicationMenu();

  // Add Environment menu items to View menu
  const viewMenu = menu.items.find((item) => item.label === 'View');
  if (viewMenu) {
    viewMenu.submenu.append(new MenuItem({ type: 'separator' }));
    viewMenu.submenu.append(
      new MenuItem({
        label: 'Environment',
        submenu: Menu.buildFromTemplate(
          createEnvironmentMenu(envToggles, (newToggles) => {
            envToggles = newToggles;
            saveSettings({ envToggles: newToggles });
            updateEnvironmentVariables(newToggles);
          })
        ),
      })
    );
  }

  const fileMenu = menu?.items.find((item) => item.label === 'File');

  // open goose to specific dir and set that as its working space
  fileMenu.submenu.append(
    new MenuItem({
      label: 'Open Directory...',
      accelerator: 'CmdOrCtrl+O',
      click() {
        openDirectoryDialog();
      },
    })
  );

  // Add Recent Files submenu
  const recentFilesSubmenu = buildRecentFilesMenu();
  if (recentFilesSubmenu.length > 0) {
    fileMenu.submenu.append(new MenuItem({ type: 'separator' }));
    fileMenu.submenu.append(
      new MenuItem({
        label: 'Recent Directories',
        submenu: recentFilesSubmenu,
      })
    );
  }

  // Add menu items to File menu
  if (fileMenu && fileMenu.submenu) {
    fileMenu.submenu.append(
      new MenuItem({
        label: 'New Chat Window',
        accelerator: 'CmdOrCtrl+N',
        click() {
          ipcMain.emit('create-chat-window');
        },
      })
    );
  }

  Menu.setApplicationMenu(menu);

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createChat(app);
    }
  });

  ipcMain.on('create-chat-window', (_, query, dir, version) => {
    if (!dir?.trim()) {
      const recentDirs = loadRecentDirs();
      dir = recentDirs.length > 0 ? recentDirs[0] : null;
    }
    createChat(app, query, dir, version);
  });

  ipcMain.on('directory-chooser', (_, replace: boolean = false) => {
    openDirectoryDialog(replace);
  });

  ipcMain.on('notify', (event, data) => {
    console.log('NOTIFY', data);
    new Notification({ title: data.title, body: data.body }).show();
  });

  ipcMain.on('logInfo', (_, info) => {
    log.info('from renderer:', info);
  });

  ipcMain.on('reload-app', () => {
    app.relaunch();
    app.exit(0);
  });

  let powerSaveBlockerId: number | null = null;

  ipcMain.handle('start-power-save-blocker', () => {
    log.info('Starting power save blocker...');
    if (powerSaveBlockerId === null) {
      powerSaveBlockerId = powerSaveBlocker.start('prevent-display-sleep');
      log.info('Started power save blocker');
      return true;
    }
    return false;
  });

  ipcMain.handle('stop-power-save-blocker', () => {
    log.info('Stopping power save blocker...');
    if (powerSaveBlockerId !== null) {
      powerSaveBlocker.stop(powerSaveBlockerId);
      powerSaveBlockerId = null;
      log.info('Stopped power save blocker');
      return true;
    }
    return false;
  });

  // Handle binary path requests
  ipcMain.handle('get-binary-path', (event, binaryName) => {
    return getBinaryPath(app, binaryName);
  });

  // Handle metadata fetching from main process
  ipcMain.handle('fetch-metadata', async (_, url) => {
    try {
      const response = await fetch(url, {
        headers: {
          'User-Agent': 'Mozilla/5.0 (compatible; Goose/1.0)',
        },
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      return await response.text();
    } catch (error) {
      console.error('Error fetching metadata:', error);
      throw error;
    }
  });

  ipcMain.on('open-in-chrome', (_, url) => {
    // On macOS, use the 'open' command with Chrome
    if (process.platform === 'darwin') {
      spawn('open', ['-a', 'Google Chrome', url]);
    } else if (process.platform === 'win32') {
      // On Windows, start is built-in command of cmd.exe
      spawn('cmd.exe', ['/c', 'start', '', 'chrome', url]);
    } else {
      // On Linux, use xdg-open with chrome
      spawn('xdg-open', [url]);
    }
  });
});

// Quit when all windows are closed, except on macOS.
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});