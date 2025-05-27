import {
  app,
  session,
  BrowserWindow,
  dialog,
  ipcMain,
  Menu,
  MenuItem,
  Notification,
  powerSaveBlocker,
  Tray,
  App,
  globalShortcut,
} from 'electron';
import { Buffer } from 'node:buffer';
import started from 'electron-squirrel-startup';
import path from 'node:path';
import { spawn } from 'child_process';
import 'dotenv/config';
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
import * as yaml from 'yaml';

if (started) app.quit();

app.setAsDefaultProtocolClient('goose');

// Only apply single instance lock on Windows where it's needed for deep links
let gotTheLock = true;
if (process.platform === 'win32') {
  gotTheLock = app.requestSingleInstanceLock();

  if (!gotTheLock) {
    app.quit();
  } else {
    app.on('second-instance', (event, commandLine) => {
      const protocolUrl = commandLine.find((arg) => arg.startsWith('goose://'));
      if (protocolUrl) {
        const parsedUrl = new URL(protocolUrl);

        // If it's a bot/recipe URL, handle it directly by creating a new window
        if (parsedUrl.hostname === 'bot' || parsedUrl.hostname === 'recipe') {
          app.whenReady().then(() => {
            const recentDirs = loadRecentDirs();
            const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

            let recipeConfig = null;
            const configParam = parsedUrl.searchParams.get('config');
            if (configParam) {
              try {
                recipeConfig = JSON.parse(Buffer.from(configParam, 'base64').toString('utf-8'));
              } catch (e) {
                console.error('Failed to parse bot config:', e);
              }
            }

            createChat(app, undefined, openDir, undefined, undefined, recipeConfig);
          });
          return; // Skip the rest of the handler
        }

        // For non-bot URLs, continue with normal handling
        handleProtocolUrl(protocolUrl);
      }

      // Only focus existing windows for non-bot/recipe URLs
      const existingWindows = BrowserWindow.getAllWindows();
      if (existingWindows.length > 0) {
        const mainWindow = existingWindows[0];
        if (mainWindow.isMinimized()) {
          mainWindow.restore();
        }
        mainWindow.focus();
      }
    });
  }

  // Handle protocol URLs on Windows startup
  const protocolUrl = process.argv.find((arg) => arg.startsWith('goose://'));
  if (protocolUrl) {
    app.whenReady().then(() => {
      handleProtocolUrl(protocolUrl);
    });
  }
}

let firstOpenWindow: BrowserWindow;
let pendingDeepLink = null;

async function handleProtocolUrl(url: string) {
  if (!url) return;

  pendingDeepLink = url;

  const parsedUrl = new URL(url);
  const recentDirs = loadRecentDirs();
  const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

  if (parsedUrl.hostname === 'bot' || parsedUrl.hostname === 'recipe') {
    // For bot/recipe URLs, skip existing window processing
    // and let processProtocolUrl handle it entirely
    processProtocolUrl(parsedUrl, null);
  } else {
    // For other URL types, reuse existing window if available
    const existingWindows = BrowserWindow.getAllWindows();
    if (existingWindows.length > 0) {
      firstOpenWindow = existingWindows[0];
      if (firstOpenWindow.isMinimized()) {
        firstOpenWindow.restore();
      }
      firstOpenWindow.focus();
    } else {
      firstOpenWindow = await createChat(app, undefined, openDir);
    }

    if (firstOpenWindow) {
      const webContents = firstOpenWindow.webContents;
      if (webContents.isLoadingMainFrame()) {
        webContents.once('did-finish-load', () => {
          processProtocolUrl(parsedUrl, firstOpenWindow);
        });
      } else {
        processProtocolUrl(parsedUrl, firstOpenWindow);
      }
    }
  }
}

function processProtocolUrl(parsedUrl: URL, window: BrowserWindow) {
  const recentDirs = loadRecentDirs();
  const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

  if (parsedUrl.hostname === 'extension') {
    window.webContents.send('add-extension', pendingDeepLink);
  } else if (parsedUrl.hostname === 'sessions') {
    window.webContents.send('open-shared-session', pendingDeepLink);
  } else if (parsedUrl.hostname === 'bot' || parsedUrl.hostname === 'recipe') {
    let recipeConfig = null;
    const configParam = parsedUrl.searchParams.get('config');
    if (configParam) {
      try {
        recipeConfig = JSON.parse(Buffer.from(configParam, 'base64').toString('utf-8'));
      } catch (e) {
        console.error('Failed to parse bot config:', e);
      }
    }
    // Create a new window and ignore the passed-in window
    createChat(app, undefined, openDir, undefined, undefined, recipeConfig);
  }
  pendingDeepLink = null;
}

app.on('open-url', async (event, url) => {
  if (process.platform !== 'win32') {
    const parsedUrl = new URL(url);
    const recentDirs = loadRecentDirs();
    const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

    // Handle bot/recipe URLs by directly creating a new window
    if (parsedUrl.hostname === 'bot' || parsedUrl.hostname === 'recipe') {
      let recipeConfig = null;
      const configParam = parsedUrl.searchParams.get('config');
      if (configParam) {
        try {
          recipeConfig = JSON.parse(Buffer.from(configParam, 'base64').toString('utf-8'));
        } catch (e) {
          console.error('Failed to parse bot config:', e);
        }
      }

      // Create a new window directly
      await createChat(app, undefined, openDir, undefined, undefined, recipeConfig);
      return; // Skip the rest of the handler
    }

    // For non-bot URLs, continue with normal handling
    pendingDeepLink = url;

    const existingWindows = BrowserWindow.getAllWindows();
    if (existingWindows.length > 0) {
      firstOpenWindow = existingWindows[0];
      if (firstOpenWindow.isMinimized()) firstOpenWindow.restore();
      firstOpenWindow.focus();
    } else {
      firstOpenWindow = await createChat(app, undefined, openDir);
    }

    if (parsedUrl.hostname === 'extension') {
      firstOpenWindow.webContents.send('add-extension', pendingDeepLink);
    } else if (parsedUrl.hostname === 'sessions') {
      firstOpenWindow.webContents.send('open-shared-session', pendingDeepLink);
    }
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
  return [process.env.GOOSE_DEFAULT_PROVIDER, process.env.GOOSE_DEFAULT_MODEL];
};

const generateSecretKey = () => {
  const key = crypto.randomBytes(32).toString('hex');
  process.env.GOOSE_SERVER__SECRET_KEY = key;
  return key;
};

const getSharingUrl = () => {
  // checks app env for sharing url
  loadShellEnv(app.isPackaged); // will try to take it from the zshrc file
  // if GOOSE_BASE_URL_SHARE is found, we will set process.env.GOOSE_BASE_URL_SHARE, otherwise we return what it is set
  // to in the env at bundle time
  return process.env.GOOSE_BASE_URL_SHARE;
};

const getVersion = () => {
  // checks app env for sharing url
  loadShellEnv(app.isPackaged); // will try to take it from the zshrc file
  // to in the env at bundle time
  return process.env.GOOSE_VERSION;
};

let [provider, model] = getGooseProvider();
console.log('[main] Got provider and model:', { provider, model });

let sharingUrl = getSharingUrl();

let gooseVersion = getVersion();

let appConfig = {
  GOOSE_DEFAULT_PROVIDER: provider,
  GOOSE_DEFAULT_MODEL: model,
  GOOSE_API_HOST: 'http://127.0.0.1',
  GOOSE_PORT: 0,
  GOOSE_WORKING_DIR: '',
  // If GOOSE_ALLOWLIST_WARNING env var is not set, defaults to false (strict blocking mode)
  GOOSE_ALLOWLIST_WARNING: process.env.GOOSE_ALLOWLIST_WARNING === 'true',
  secretKey: generateSecretKey(),
};

console.log('[main] Created appConfig:', appConfig);

// Track windows by ID
let windowCounter = 0;
const windowMap = new Map<number, BrowserWindow>();

interface RecipeConfig {
  id: string;
  title: string;
  description: string;
  instructions: string;
  activities: string[];
  prompt: string;
}

const createChat = async (
  app: App,
  query?: string,
  dir?: string,
  version?: string,
  resumeSessionId?: string,
  recipeConfig?: RecipeConfig, // Bot configuration
  viewType?: string // View type
) => {
  // Initialize variables for process and configuration
  let port = 0;
  let working_dir = '';
  let goosedProcess = null;

  if (viewType === 'recipeEditor') {
    // For recipeEditor, get the port from existing windows' config
    const existingWindows = BrowserWindow.getAllWindows();
    if (existingWindows.length > 0) {
      // Get the config from localStorage through an existing window
      try {
        const config = await existingWindows[0].webContents.executeJavaScript(
          `window.electron.getConfig()`
        );
        if (config) {
          port = config.GOOSE_PORT;
          working_dir = config.GOOSE_WORKING_DIR;
        }
      } catch (e) {
        console.error('Failed to get config from localStorage:', e);
      }
    }
    if (port === 0) {
      console.error('No existing Goose process found for recipeEditor');
      throw new Error('Cannot create recipeEditor window: No existing Goose process found');
    }
  } else {
    // Apply current environment settings before creating chat
    updateEnvironmentVariables(envToggles);
    // Start new Goosed process for regular windows
    [port, working_dir, goosedProcess] = await startGoosed(app, dir);
  }

  const mainWindow = new BrowserWindow({
    titleBarStyle: process.platform === 'darwin' ? 'hidden' : 'default',
    trafficLightPosition: process.platform === 'darwin' ? { x: 16, y: 20 } : undefined,
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
      spellcheck: true,
      preload: path.join(__dirname, 'preload.js'),
      additionalArguments: [
        JSON.stringify({
          ...appConfig, // Use the potentially updated appConfig
          GOOSE_PORT: port, // Ensure this specific window gets the correct port
          GOOSE_WORKING_DIR: working_dir,
          REQUEST_DIR: dir,
          GOOSE_BASE_URL_SHARE: sharingUrl,
          GOOSE_VERSION: gooseVersion,
          recipeConfig: recipeConfig,
        }),
      ],
      partition: 'persist:goose', // Add this line to ensure persistence
    },
  });

  // Enable spellcheck / right and ctrl + click on mispelled word
  //
  // NOTE: We could use webContents.session.availableSpellCheckerLanguages to include
  // all languages in the list of spell checked words, but it diminishes the times you
  // get red squigglies back for mispelled english words. Given the rest of Goose only
  // renders in english right now, this feels like the correct set of language codes
  // for the moment.
  //
  // TODO: Load language codes from a setting if we ever have i18n/l10n
  mainWindow.webContents.session.setSpellCheckerLanguages(['en-US', 'en-GB']);
  mainWindow.webContents.on('context-menu', (event, params) => {
    const menu = new Menu();

    // Add each spelling suggestion
    for (const suggestion of params.dictionarySuggestions) {
      menu.append(
        new MenuItem({
          label: suggestion,
          click: () => mainWindow.webContents.replaceMisspelling(suggestion),
        })
      );
    }

    // Allow users to add the misspelled word to the dictionary
    if (params.misspelledWord) {
      menu.append(
        new MenuItem({
          label: 'Add to dictionary',
          click: () =>
            mainWindow.webContents.session.addWordToSpellCheckerDictionary(params.misspelledWord),
        })
      );
    }

    menu.popup();
  });

  // Store config in localStorage for future windows
  const windowConfig = {
    ...appConfig, // Use the potentially updated appConfig here as well
    GOOSE_PORT: port, // Ensure this specific window's config gets the correct port
    GOOSE_WORKING_DIR: working_dir,
    REQUEST_DIR: dir,
    GOOSE_BASE_URL_SHARE: sharingUrl,
    recipeConfig: recipeConfig,
  };

  // We need to wait for the window to load before we can access localStorage
  mainWindow.webContents.on('did-finish-load', () => {
    const configStr = JSON.stringify(windowConfig).replace(/'/g, "\\'");
    mainWindow.webContents.executeJavaScript(`
      localStorage.setItem('gooseConfig', '${configStr}')
    `);
  });

  console.log('[main] Creating window with config:', windowConfig);

  // Handle new window creation for links
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    // Open all links in external browser
    if (url.startsWith('http:') || url.startsWith('https:')) {
      electron.shell.openExternal(url);
      return { action: 'deny' };
    }
    return { action: 'allow' };
  });

  // Load the index.html of the app.
  let queryParams = '';
  if (query) {
    queryParams = `?initialQuery=${encodeURIComponent(query)}`;
  }

  // Add resumeSessionId to query params if provided
  if (resumeSessionId) {
    queryParams = queryParams
      ? `${queryParams}&resumeSessionId=${encodeURIComponent(resumeSessionId)}`
      : `?resumeSessionId=${encodeURIComponent(resumeSessionId)}`;
  }

  // Add view type to query params if provided
  if (viewType) {
    queryParams = queryParams
      ? `${queryParams}&view=${encodeURIComponent(viewType)}`
      : `?view=${encodeURIComponent(viewType)}`;
  }

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
    mainWindow.loadURL(`${MAIN_WINDOW_VITE_DEV_SERVER_URL}${queryParams}`);
  } else {
    // In production, we need to use a proper file protocol URL with correct base path
    const indexPath = path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`);
    console.log('Loading production path:', indexPath);
    mainWindow.loadFile(indexPath, {
      search: queryParams ? queryParams.slice(1) : undefined,
    });
  }

  // Set up local keyboard shortcuts that only work when the window is focused
  mainWindow.webContents.on('before-input-event', (event, input) => {
    if (input.key === 'r' && input.meta) {
      mainWindow.reload();
      event.preventDefault();
    }

    if (input.key === 'i' && input.alt && input.meta) {
      mainWindow.webContents.openDevTools();
      event.preventDefault();
    }
  });

  windowMap.set(windowId, mainWindow);
  // Handle window closure
  mainWindow.on('closed', () => {
    windowMap.delete(windowId);
    if (goosedProcess) {
      goosedProcess.kill();
    }
  });
  return mainWindow;
};

// Track tray instance
let tray: Tray | null = null;

const createTray = () => {
  const isDev = process.env.NODE_ENV === 'development';
  let iconPath: string;

  if (isDev) {
    iconPath = path.join(process.cwd(), 'src', 'images', 'iconTemplate.png');
  } else {
    iconPath = path.join(process.resourcesPath, 'images', 'iconTemplate.png');
  }

  tray = new Tray(iconPath);

  const contextMenu = Menu.buildFromTemplate([
    { label: 'Show Window', click: showWindow },
    { type: 'separator' },
    { label: 'Quit', click: () => app.quit() },
  ]);

  tray.setToolTip('Goose');
  tray.setContextMenu(contextMenu);

  // On Windows, clicking the tray icon should show the window
  if (process.platform === 'win32') {
    tray.on('click', showWindow);
  }
};

const showWindow = async () => {
  const windows = BrowserWindow.getAllWindows();

  if (windows.length === 0) {
    log.info('No windows are open, creating a new one...');
    const recentDirs = loadRecentDirs();
    const openDir = recentDirs.length > 0 ? recentDirs[0] : null;
    await createChat(app, undefined, openDir);
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
    properties: ['openFile', 'openDirectory'],
  });

  if (!result.canceled && result.filePaths.length > 0) {
    addRecentDir(result.filePaths[0]);
    const currentWindow = BrowserWindow.getFocusedWindow();
    await createChat(app, undefined, result.filePaths[0]);
    if (replaceWindow) {
      currentWindow.close();
    }
  }
  return result;
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

ipcMain.on('react-ready', () => {
  console.log('React ready event received');

  if (pendingDeepLink) {
    console.log('Processing pending deep link:', pendingDeepLink);
    handleProtocolUrl(pendingDeepLink);
  } else {
    console.log('No pending deep link to process');
  }

  // We don't need to handle pending deep links here anymore
  // since we're handling them in the window creation flow
  console.log('[main] React ready - window is prepared for deep links');
});

// Handle directory chooser
ipcMain.handle('directory-chooser', (_event, replace: boolean = false) => {
  return openDirectoryDialog(replace);
});

// Add file/directory selection handler
ipcMain.handle('select-file-or-directory', async () => {
  const result = await dialog.showOpenDialog({
    properties: process.platform === 'darwin' ? ['openFile', 'openDirectory'] : ['openFile'],
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
      const ps = spawn('ps', ['aux']);
      const grep = spawn('grep', ['-iw', '[o]llama']);

      let output = '';
      let errorOutput = '';

      // Pipe ps output to grep
      ps.stdout.pipe(grep.stdin);

      grep.stdout.on('data', (data) => {
        output += data.toString();
      });

      grep.stderr.on('data', (data) => {
        errorOutput += data.toString();
      });

      grep.on('close', (code) => {
        if (code !== null && code !== 0 && code !== 1) {
          // grep returns 1 when no matches found
          console.error('Error executing grep command:', errorOutput);
          return resolve(false);
        }

        console.log('Raw stdout from ps|grep command:', output);
        const trimmedOutput = output.trim();
        console.log('Trimmed stdout:', trimmedOutput);

        const isRunning = trimmedOutput.length > 0;
        resolve(isRunning);
      });

      ps.on('error', (error) => {
        console.error('Error executing ps command:', error);
        resolve(false);
      });

      grep.on('error', (error) => {
        console.error('Error executing grep command:', error);
        resolve(false);
      });

      // Close ps stdin when done
      ps.stdout.on('end', () => {
        grep.stdin.end();
      });
    });
  } catch (err) {
    console.error('Error checking for Ollama:', err);
    return false;
  }
});

// Handle binary path requests
ipcMain.handle('get-binary-path', (_event, binaryName) => {
  return getBinaryPath(app, binaryName);
});

ipcMain.handle('read-file', (_event, filePath) => {
  return new Promise((resolve) => {
    const cat = spawn('cat', [filePath]);
    let output = '';
    let errorOutput = '';

    cat.stdout.on('data', (data) => {
      output += data.toString();
    });

    cat.stderr.on('data', (data) => {
      errorOutput += data.toString();
    });

    cat.on('close', (code) => {
      if (code !== 0) {
        // File not found or error
        resolve({ file: '', filePath, error: errorOutput || null, found: false });
        return;
      }
      resolve({ file: output, filePath, error: null, found: true });
    });

    cat.on('error', (error) => {
      console.error('Error reading file:', error);
      resolve({ file: '', filePath, error, found: false });
    });
  });
});

ipcMain.handle('write-file', (_event, filePath, content) => {
  return new Promise((resolve) => {
    // Create a write stream to the file
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const fs = require('fs');
    try {
      fs.writeFileSync(filePath, content, { encoding: 'utf8' });
      resolve(true);
    } catch (error) {
      console.error('Error writing to file:', error);
      resolve(false);
    }
  });
});

// Handle allowed extensions list fetching
ipcMain.handle('get-allowed-extensions', async () => {
  try {
    const allowList = await getAllowList();
    return allowList;
  } catch (error) {
    console.error('Error fetching allowed extensions:', error);
    throw error;
  }
});

const createNewWindow = async (app: App, dir?: string | null) => {
  const recentDirs = loadRecentDirs();
  const openDir = dir || (recentDirs.length > 0 ? recentDirs[0] : undefined);
  createChat(app, undefined, openDir);
};

const focusWindow = () => {
  const windows = BrowserWindow.getAllWindows();
  if (windows.length > 0) {
    windows.forEach((win) => {
      win.show();
    });
    windows[windows.length - 1].webContents.send('focus-input');
  } else {
    createNewWindow(app);
  }
};

const registerGlobalHotkey = (accelerator: string) => {
  // Unregister any existing shortcuts first
  globalShortcut.unregisterAll();

  try {
    const ret = globalShortcut.register(accelerator, () => {
      focusWindow();
    });

    if (!ret) {
      console.error('Failed to register global hotkey');
      return false;
    }
    return true;
  } catch (e) {
    console.error('Error registering global hotkey:', e);
    return false;
  }
};

app.whenReady().then(async () => {
  // Add CSP headers to all sessions
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          "default-src 'self';" +
            // Allow inline styles since we use them in our React components
            "style-src 'self' 'unsafe-inline';" +
            // Scripts only from our app
            "script-src 'self';" +
            // Images from our app and data: URLs (for base64 images)
            "img-src 'self' data: https:;" +
            // Connect to our local API and specific external services
            "connect-src 'self' http://127.0.0.1:*" +
            // Don't allow any plugins
            "object-src 'none';" +
            // Don't allow any frames
            "frame-src 'none';" +
            // Font sources
            "font-src 'self';" +
            // Media sources
            "media-src 'none';" +
            // Form actions
            "form-action 'none';" +
            // Base URI restriction
            "base-uri 'self';" +
            // Manifest files
            "manifest-src 'self';" +
            // Worker sources
            "worker-src 'self';" +
            // Upgrade insecure requests
            'upgrade-insecure-requests;',
        ],
      },
    });
  });

  // Register the default global hotkey
  registerGlobalHotkey('CommandOrControl+Alt+Shift+G');

  session.defaultSession.webRequest.onBeforeSendHeaders((details, callback) => {
    details.requestHeaders['Origin'] = 'http://localhost:5173';
    callback({ cancel: false, requestHeaders: details.requestHeaders });
  });

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
  createNewWindow(app, dirPath);

  // Get the existing menu
  const menu = Menu.getApplicationMenu();

  // App menu
  const appMenu = menu?.items.find((item) => item.label === 'Goose');
  if (appMenu?.submenu) {
    // add Settings to app menu after About
    appMenu.submenu.insert(1, new MenuItem({ type: 'separator' }));
    appMenu.submenu.insert(
      1,
      new MenuItem({
        label: 'Settings',
        accelerator: 'CmdOrCtrl+,',
        click() {
          const focusedWindow = BrowserWindow.getFocusedWindow();
          if (focusedWindow) focusedWindow.webContents.send('set-view', 'settings');
        },
      })
    );
    appMenu.submenu.insert(1, new MenuItem({ type: 'separator' }));
  }

  // Add Find submenu to Edit menu
  const editMenu = menu?.items.find((item) => item.label === 'Edit');
  if (editMenu?.submenu) {
    // Find the index of Select All to insert after it
    const selectAllIndex = editMenu.submenu.items.findIndex((item) => item.label === 'Select All');

    // Create Find submenu
    const findSubmenu = Menu.buildFromTemplate([
      {
        label: 'Find…',
        accelerator: process.platform === 'darwin' ? 'Command+F' : 'Control+F',
        click() {
          const focusedWindow = BrowserWindow.getFocusedWindow();
          if (focusedWindow) focusedWindow.webContents.send('find-command');
        },
      },
      {
        label: 'Find Next',
        accelerator: process.platform === 'darwin' ? 'Command+G' : 'Control+G',
        click() {
          const focusedWindow = BrowserWindow.getFocusedWindow();
          if (focusedWindow) focusedWindow.webContents.send('find-next');
        },
      },
      {
        label: 'Find Previous',
        accelerator: process.platform === 'darwin' ? 'Shift+Command+G' : 'Shift+Control+G',
        click() {
          const focusedWindow = BrowserWindow.getFocusedWindow();
          if (focusedWindow) focusedWindow.webContents.send('find-previous');
        },
      },
      {
        label: 'Use Selection for Find',
        accelerator: process.platform === 'darwin' ? 'Command+E' : null,
        click() {
          const focusedWindow = BrowserWindow.getFocusedWindow();
          if (focusedWindow) focusedWindow.webContents.send('use-selection-find');
        },
        visible: process.platform === 'darwin', // Only show on Mac
      },
    ]);

    // Add Find submenu to Edit menu
    editMenu.submenu.insert(
      selectAllIndex + 1,
      new MenuItem({
        label: 'Find',
        submenu: findSubmenu,
      })
    );
  }

  // Add Environment menu items to View menu
  const viewMenu = menu?.items.find((item) => item.label === 'View');
  if (viewMenu?.submenu) {
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

  if (fileMenu?.submenu) {
    fileMenu.submenu.insert(
      0,
      new MenuItem({
        label: 'New Chat Window',
        accelerator: process.platform === 'darwin' ? 'Cmd+N' : 'Ctrl+N',
        click() {
          ipcMain.emit('create-chat-window');
        },
      })
    );

    // Open goose to specific dir and set that as its working space
    fileMenu.submenu.insert(
      1,
      new MenuItem({
        label: 'Open Directory...',
        accelerator: 'CmdOrCtrl+O',
        click: () => openDirectoryDialog(),
      })
    );

    // Add Recent Files submenu
    const recentFilesSubmenu = buildRecentFilesMenu();
    if (recentFilesSubmenu.length > 0) {
      fileMenu.submenu.insert(
        2,
        new MenuItem({
          label: 'Recent Directories',
          submenu: recentFilesSubmenu,
        })
      );
    }

    fileMenu.submenu.insert(3, new MenuItem({ type: 'separator' }));

    // The Close Window item is here.

    // Add menu item to tell the user about the keyboard shortcut
    fileMenu.submenu.append(
      new MenuItem({
        label: 'Focus Goose Window',
        accelerator: 'CmdOrCtrl+Alt+Shift+G',
        click() {
          focusWindow();
        },
      })
    );
  }

  // on macOS, the topbar is hidden
  if (menu && process.platform !== 'darwin') {
    let helpMenu = menu.items.find((item) => item.label === 'Help');

    // If Help menu doesn't exist, create it and add it to the menu
    if (!helpMenu) {
      helpMenu = new MenuItem({
        label: 'Help',
        submenu: Menu.buildFromTemplate([]), // Start with an empty submenu
      });
      // Find a reasonable place to insert the Help menu, usually near the end
      const insertIndex = menu.items.length > 0 ? menu.items.length - 1 : 0;
      menu.items.splice(insertIndex, 0, helpMenu);
    }

    // Ensure the Help menu has a submenu before appending
    if (helpMenu.submenu) {
      // Add a separator before the About item if the submenu is not empty
      if (helpMenu.submenu.items.length > 0) {
        helpMenu.submenu.append(new MenuItem({ type: 'separator' }));
      }

      // Create the About Goose menu item with a submenu
      const aboutGooseMenuItem = new MenuItem({
        label: 'About Goose',
        submenu: Menu.buildFromTemplate([]), // Start with an empty submenu for About
      });

      // Add the Version menu item (display only) to the About Goose submenu
      if (aboutGooseMenuItem.submenu) {
        aboutGooseMenuItem.submenu.append(
          new MenuItem({
            label: `Version ${gooseVersion || app.getVersion()}`,
            enabled: false,
          })
        );
      }

      helpMenu.submenu.append(aboutGooseMenuItem);
    }
  }

  if (menu) {
    Menu.setApplicationMenu(menu);
  }

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createChat(app);
    }
  });

  ipcMain.on(
    'create-chat-window',
    (_, query, dir, version, resumeSessionId, recipeConfig, viewType) => {
      if (!dir?.trim()) {
        const recentDirs = loadRecentDirs();
        dir = recentDirs.length > 0 ? recentDirs[0] : null;
      }

      // Log the recipeConfig for debugging
      console.log('Creating chat window with recipeConfig:', recipeConfig);

      // Pass recipeConfig as part of viewOptions when viewType is recipeEditor
      createChat(app, query, dir, version, resumeSessionId, recipeConfig, viewType);
    }
  );

  ipcMain.on('notify', (_event, data) => {
    try {
      // Validate notification data
      if (!data || typeof data !== 'object') {
        console.error('Invalid notification data');
        return;
      }

      // Validate title and body
      if (typeof data.title !== 'string' || typeof data.body !== 'string') {
        console.error('Invalid notification title or body');
        return;
      }

      // Limit the length of title and body
      const MAX_LENGTH = 1000;
      if (data.title.length > MAX_LENGTH || data.body.length > MAX_LENGTH) {
        console.error('Notification title or body too long');
        return;
      }

      // Remove any HTML tags for security
      const sanitizeText = (text: string) => text.replace(/<[^>]*>/g, '');

      console.log('NOTIFY', data);
      new Notification({
        title: sanitizeText(data.title),
        body: sanitizeText(data.body),
      }).show();
    } catch (error) {
      console.error('Error showing notification:', error);
    }
  });

  ipcMain.on('logInfo', (_event, info) => {
    try {
      // Validate log info
      if (info === undefined || info === null) {
        console.error('Invalid log info: undefined or null');
        return;
      }

      // Convert to string if not already
      const logMessage = String(info);

      // Limit log message length
      const MAX_LENGTH = 10000; // 10KB limit
      if (logMessage.length > MAX_LENGTH) {
        console.error('Log message too long');
        return;
      }

      // Log the sanitized message
      log.info('from renderer:', logMessage);
    } catch (error) {
      console.error('Error logging info:', error);
    }
  });

  ipcMain.on('reload-app', (event) => {
    // Get the window that sent the event
    const window = BrowserWindow.fromWebContents(event.sender);
    if (window) {
      window.reload();
    }
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
  ipcMain.handle('get-binary-path', (_event, binaryName) => {
    return getBinaryPath(app, binaryName);
  });

  // Handle metadata fetching from main process
  ipcMain.handle('fetch-metadata', async (_event, url) => {
    try {
      // Validate URL
      const parsedUrl = new URL(url);

      // Only allow http and https protocols
      if (!['http:', 'https:'].includes(parsedUrl.protocol)) {
        throw new Error('Invalid URL protocol. Only HTTP and HTTPS are allowed.');
      }

      const response = await fetch(url, {
        headers: {
          'User-Agent': 'Mozilla/5.0 (compatible; Goose/1.0)',
        },
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      // Set a reasonable size limit (e.g., 10MB)
      const MAX_SIZE = 10 * 1024 * 1024; // 10MB
      const contentLength = parseInt(response.headers.get('content-length') || '0');
      if (contentLength > MAX_SIZE) {
        throw new Error('Response too large');
      }

      const text = await response.text();
      if (text.length > MAX_SIZE) {
        throw new Error('Response too large');
      }

      return text;
    } catch (error) {
      console.error('Error fetching metadata:', error);
      throw error;
    }
  });

  ipcMain.on('open-in-chrome', (_event, url) => {
    try {
      // Validate URL
      const parsedUrl = new URL(url);

      // Only allow http and https protocols
      if (!['http:', 'https:'].includes(parsedUrl.protocol)) {
        console.error('Invalid URL protocol. Only HTTP and HTTPS are allowed.');
        return;
      }

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
    } catch (error) {
      console.error('Error opening URL in Chrome:', error);
    }
  });
});

/**
 * Fetches the allowed extensions list from the remote YAML file if GOOSE_ALLOWLIST is set.
 * If the ALLOWLIST is not set, any are allowed. If one is set, it will warn if the deeplink
 * doesn't match a command from the list.
 * If it fails to load, then it will return an empty list.
 * If the format is incorrect, it will return an empty list.
 * Format of yaml is:
 *
 ```yaml:
 extensions:
  - id: slack
    command: uvx mcp_slack
  - id: knowledge_graph_memory
    command: npx -y @modelcontextprotocol/server-memory
  ```
 *
 * @returns A promise that resolves to an array of extension commands that are allowed.
 */
async function getAllowList(): Promise<string[]> {
  if (!process.env.GOOSE_ALLOWLIST) {
    return [];
  }

  try {
    // Fetch the YAML file
    const response = await fetch(process.env.GOOSE_ALLOWLIST);

    if (!response.ok) {
      throw new Error(
        `Failed to fetch allowed extensions: ${response.status} ${response.statusText}`
      );
    }

    // Parse the YAML content
    const yamlContent = await response.text();
    const parsedYaml = yaml.parse(yamlContent);

    // Extract the commands from the extensions array
    if (parsedYaml && parsedYaml.extensions && Array.isArray(parsedYaml.extensions)) {
      const commands = parsedYaml.extensions.map(
        (ext: { id: string; command: string }) => ext.command
      );
      console.log(`Fetched ${commands.length} allowed extension commands`);
      return commands;
    } else {
      console.error('Invalid YAML structure:', parsedYaml);
      return [];
    }
  } catch (error) {
    console.error('Error in getAllowList:', error);
    throw error;
  }
}

app.on('will-quit', () => {
  // Unregister all shortcuts when quitting
  globalShortcut.unregisterAll();
});

// Quit when all windows are closed, except on macOS or if we have a tray icon.
app.on('window-all-closed', () => {
  // Only quit if we're not on macOS or don't have a tray icon
  if (process.platform !== 'darwin' || !tray) {
    app.quit();
  }
});
