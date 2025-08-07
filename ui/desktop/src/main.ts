import type { OpenDialogOptions, OpenDialogReturnValue } from 'electron';
import {
  app,
  App,
  BrowserWindow,
  dialog,
  Event,
  globalShortcut,
  ipcMain,
  Menu,
  MenuItem,
  Notification,
  powerSaveBlocker,
  session,
  shell,
  Tray,
} from 'electron';
import { Buffer } from 'node:buffer';
import { MouseUpEvent } from './types/electron';
import fs from 'node:fs/promises';
import fsSync from 'node:fs';
import started from 'electron-squirrel-startup';
import path from 'node:path';
import os from 'node:os';
import { spawn } from 'child_process';
import 'dotenv/config';
import { startGoosed } from './goosed';
import { expandTilde, getBinaryPath } from './utils/pathUtils';
import { loadShellEnv } from './utils/loadEnv';
import log from './utils/logger';
import { ensureWinShims } from './utils/winShims';
import { addRecentDir, loadRecentDirs } from './utils/recentDirs';
import {
  EnvToggles,
  loadSettings,
  saveSettings,
  SchedulingEngine,
  updateEnvironmentVariables,
  updateSchedulingEngineEnvironment,
} from './utils/settings';
import * as crypto from 'crypto';
// import electron from "electron";
import * as yaml from 'yaml';
import windowStateKeeper from 'electron-window-state';
import {
  getUpdateAvailable,
  registerUpdateIpcHandlers,
  setTrayRef,
  setupAutoUpdater,
  updateTrayMenu,
} from './utils/autoUpdater';
import { UPDATES_ENABLED } from './updates';
import { Recipe } from './recipe';
import './utils/recipeHash';

// API URL constructor for main process before window is ready
function getApiUrlMain(endpoint: string, dynamicPort: number): string {
  const host = process.env.GOOSE_API_HOST || 'http://127.0.0.1';
  const port = dynamicPort || process.env.GOOSE_PORT;
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${host}:${port}${cleanEndpoint}`;
}

// When opening the app with a deeplink, the window is still initializing so we have to duplicate some window dependant logic here.
async function decodeRecipeMain(deeplink: string, port: number): Promise<Recipe | null> {
  try {
    const response = await fetch(getApiUrlMain('/recipes/decode', port), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ deeplink }),
    });

    if (response.ok) {
      const data = await response.json();
      return data.recipe;
    }
  } catch (error) {
    console.error('Failed to decode recipe:', error);
  }
  return null;
}

// Updater functions (moved here to keep updates.ts minimal for release replacement)
function shouldSetupUpdater(): boolean {
  // Setup updater if either the flag is enabled OR dev updates are enabled
  return UPDATES_ENABLED || process.env.ENABLE_DEV_UPDATES === 'true';
}

// Define temp directory for pasted images
const gooseTempDir = path.join(app.getPath('temp'), 'goose-pasted-images');

// Function to ensure the temporary directory exists
async function ensureTempDirExists(): Promise<string> {
  try {
    // Check if the path already exists
    try {
      const stats = await fs.stat(gooseTempDir);

      // If it exists but is not a directory, remove it and recreate
      if (!stats.isDirectory()) {
        await fs.unlink(gooseTempDir);
        await fs.mkdir(gooseTempDir, { recursive: true });
      }

      // Startup cleanup: remove old files and any symlinks
      const files = await fs.readdir(gooseTempDir);
      const now = Date.now();
      const MAX_AGE = 24 * 60 * 60 * 1000; // 24 hours in milliseconds

      for (const file of files) {
        const filePath = path.join(gooseTempDir, file);
        try {
          const fileStats = await fs.lstat(filePath);

          // Always remove symlinks
          if (fileStats.isSymbolicLink()) {
            console.warn(
              `[Main] Found symlink in temp directory during startup: ${filePath}. Removing it.`
            );
            await fs.unlink(filePath);
            continue;
          }

          // Remove old files (older than 24 hours)
          if (fileStats.isFile()) {
            const fileAge = now - fileStats.mtime.getTime();
            if (fileAge > MAX_AGE) {
              console.log(
                `[Main] Removing old temp file during startup: ${filePath} (age: ${Math.round(fileAge / (60 * 60 * 1000))} hours)`
              );
              await fs.unlink(filePath);
            }
          }
        } catch (fileError) {
          // If we can't stat the file, try to remove it anyway
          console.warn(`[Main] Could not stat file ${filePath}, attempting to remove:`, fileError);
          try {
            await fs.unlink(filePath);
          } catch (unlinkError) {
            console.error(`[Main] Failed to remove problematic file ${filePath}:`, unlinkError);
          }
        }
      }
    } catch (error) {
      if (error && typeof error === 'object' && 'code' in error && error.code === 'ENOENT') {
        // Directory doesn't exist, create it
        await fs.mkdir(gooseTempDir, { recursive: true });
      } else {
        throw error;
      }
    }

    // Set proper permissions on the directory (0755 = rwxr-xr-x)
    await fs.chmod(gooseTempDir, 0o755);

    console.log('[Main] Temporary directory for pasted images ensured:', gooseTempDir);
  } catch (error) {
    console.error('[Main] Failed to create temp directory:', gooseTempDir, error);
    throw error; // Propagate error
  }
  return gooseTempDir;
}

if (started) app.quit();

// In development mode, force registration as the default protocol client
// In production, register normally
if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
  // Development mode - force registration
  console.log('[Main] Development mode: Forcing protocol registration for goose://');
  app.setAsDefaultProtocolClient('goose');

  if (process.platform === 'darwin') {
    try {
      // Reset the default handler to ensure dev version takes precedence
      spawn('open', ['-a', process.execPath, '--args', '--reset-protocol-handler', 'goose'], {
        detached: true,
        stdio: 'ignore',
      });
    } catch (error) {
      console.warn('[Main] Could not reset protocol handler:', error);
    }
  }
} else {
  // Production mode - normal registration
  app.setAsDefaultProtocolClient('goose');
}

// Only apply single instance lock on Windows where it's needed for deep links
let gotTheLock = true;
if (process.platform === 'win32') {
  gotTheLock = app.requestSingleInstanceLock();

  if (!gotTheLock) {
    app.quit();
  } else {
    app.on('second-instance', (_event, commandLine) => {
      const protocolUrl = commandLine.find((arg) => arg.startsWith('goose://'));
      if (protocolUrl) {
        const parsedUrl = new URL(protocolUrl);
        // If it's a bot/recipe URL, handle it directly by creating a new window
        if (parsedUrl.hostname === 'bot' || parsedUrl.hostname === 'recipe') {
          app.whenReady().then(async () => {
            const recentDirs = loadRecentDirs();
            const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

            const recipeDeeplink = parsedUrl.searchParams.get('config');
            const scheduledJobId = parsedUrl.searchParams.get('scheduledJob');

            createChat(
              app,
              undefined,
              openDir || undefined,
              undefined,
              undefined,
              undefined,
              undefined,
              recipeDeeplink || undefined,
              scheduledJobId || undefined
            );
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
let pendingDeepLink: string | null = null;

async function handleProtocolUrl(url: string) {
  if (!url) return;

  pendingDeepLink = url;

  const parsedUrl = new URL(url);
  const recentDirs = loadRecentDirs();
  const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

  if (parsedUrl.hostname === 'bot' || parsedUrl.hostname === 'recipe') {
    // For bot/recipe URLs, get existing window or create new one
    const existingWindows = BrowserWindow.getAllWindows();
    const targetWindow =
      existingWindows.length > 0
        ? existingWindows[0]
        : await createChat(app, undefined, openDir || undefined);
    await processProtocolUrl(parsedUrl, targetWindow);
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
      firstOpenWindow = await createChat(app, undefined, openDir || undefined);
    }

    if (firstOpenWindow) {
      const webContents = firstOpenWindow.webContents;
      if (webContents.isLoadingMainFrame()) {
        webContents.once('did-finish-load', async () => {
          await processProtocolUrl(parsedUrl, firstOpenWindow);
        });
      } else {
        await processProtocolUrl(parsedUrl, firstOpenWindow);
      }
    }
  }
}

async function processProtocolUrl(parsedUrl: URL, window: BrowserWindow) {
  const recentDirs = loadRecentDirs();
  const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

  if (parsedUrl.hostname === 'extension') {
    window.webContents.send('add-extension', pendingDeepLink);
  } else if (parsedUrl.hostname === 'sessions') {
    window.webContents.send('open-shared-session', pendingDeepLink);
  } else if (parsedUrl.hostname === 'bot' || parsedUrl.hostname === 'recipe') {
    const recipeDeeplink = parsedUrl.searchParams.get('config');
    const scheduledJobId = parsedUrl.searchParams.get('scheduledJob');

    // Create a new window and ignore the passed-in window
    createChat(
      app,
      undefined,
      openDir || undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      recipeDeeplink || undefined,
      scheduledJobId || undefined
    );
  }
  pendingDeepLink = null;
}

app.on('open-url', async (_event, url) => {
  if (process.platform !== 'win32') {
    const parsedUrl = new URL(url);
    const recentDirs = loadRecentDirs();
    const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

    // Handle bot/recipe URLs by directly creating a new window
    console.log('[Main] Received open-url event:', url);
    if (parsedUrl.hostname === 'bot' || parsedUrl.hostname === 'recipe') {
      console.log('[Main] Detected bot/recipe URL, creating new chat window');
      const recipeDeeplink = parsedUrl.searchParams.get('config');
      const scheduledJobId = parsedUrl.searchParams.get('scheduledJob');

      // Create a new window directly
      await createChat(
        app,
        undefined,
        openDir || undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        recipeDeeplink || undefined,
        scheduledJobId || undefined
      );
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
      firstOpenWindow = await createChat(app, undefined, openDir || undefined);
    }

    if (parsedUrl.hostname === 'extension') {
      firstOpenWindow.webContents.send('add-extension', pendingDeepLink);
    } else if (parsedUrl.hostname === 'sessions') {
      firstOpenWindow.webContents.send('open-shared-session', pendingDeepLink);
    }
  }
});

// Handle macOS drag-and-drop onto dock icon
app.on('will-finish-launching', () => {
  if (process.platform === 'darwin') {
    app.setAboutPanelOptions({
      applicationName: 'Goose',
      applicationVersion: app.getVersion(),
    });
  }
});

// Handle drag-and-drop onto dock icon
app.on('open-file', async (event, filePath) => {
  event.preventDefault();
  await handleFileOpen(filePath);
});

// Handle multiple files/folders
app.on('open-files', async (event: Event, filePaths: string[]) => {
  event.preventDefault();
  for (const filePath of filePaths) {
    await handleFileOpen(filePath);
  }
});

async function handleFileOpen(filePath: string) {
  try {
    if (!filePath || typeof filePath !== 'string') {
      return;
    }

    const stats = fsSync.lstatSync(filePath);
    let targetDir = filePath;

    // If it's a file, use its parent directory
    if (stats.isFile()) {
      targetDir = path.dirname(filePath);
    }

    // Add to recent directories
    addRecentDir(targetDir);

    // Create new window for the directory
    const newWindow = await createChat(app, undefined, targetDir);

    // Focus the new window
    if (newWindow) {
      newWindow.show();
      newWindow.focus();
      newWindow.moveTop();
    }
  } catch (error) {
    console.error('Failed to handle file open:', error);

    // Show user-friendly error notification
    new Notification({
      title: 'Goose',
      body: `Could not open directory: ${path.basename(filePath)}`,
    }).show();
  }
}

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
  return [
    process.env.GOOSE_DEFAULT_PROVIDER,
    process.env.GOOSE_DEFAULT_MODEL,
    process.env.GOOSE_PREDEFINED_MODELS,
  ];
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

const [provider, model, predefinedModels] = getGooseProvider();

const sharingUrl = getSharingUrl();

const gooseVersion = getVersion();

const SERVER_SECRET = process.env.GOOSE_EXTERNAL_BACKEND
  ? 'test'
  : crypto.randomBytes(32).toString('hex');

let appConfig = {
  GOOSE_DEFAULT_PROVIDER: provider,
  GOOSE_DEFAULT_MODEL: model,
  GOOSE_PREDEFINED_MODELS: predefinedModels,
  GOOSE_API_HOST: 'http://127.0.0.1',
  GOOSE_PORT: 0,
  GOOSE_WORKING_DIR: '',
  // If GOOSE_ALLOWLIST_WARNING env var is not set, defaults to false (strict blocking mode)
  GOOSE_ALLOWLIST_WARNING: process.env.GOOSE_ALLOWLIST_WARNING === 'true',
};

// Track windows by ID
let windowCounter = 0;
const windowMap = new Map<number, BrowserWindow>();

// Track power save blockers per window
const windowPowerSaveBlockers = new Map<number, number>(); // windowId -> blockerId

const createChat = async (
  app: App,
  query?: string,
  dir?: string,
  _version?: string,
  resumeSessionId?: string,
  recipe?: Recipe, // Recipe configuration when already loaded, takes precedence over deeplink
  viewType?: string,
  recipeDeeplink?: string, // Raw deeplink used as a fallback when recipe is not loaded. Required on new windows as we need to wait for the window to load before decoding.
  scheduledJobId?: string // Scheduled job ID if applicable
) => {
  // Initialize variables for process and configuration
  let port = 0;
  let working_dir = '';
  let goosedProcess: import('child_process').ChildProcess | null = null;

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

    // Apply scheduling engine setting
    const settings = loadSettings();
    updateSchedulingEngineEnvironment(settings.schedulingEngine);

    // Start new Goosed process for regular windows
    // Pass through scheduling engine environment variables
    const envVars = {
      GOOSE_SCHEDULER_TYPE: process.env.GOOSE_SCHEDULER_TYPE,
    };
    const [newPort, newWorkingDir, newGoosedProcess] = await startGoosed(
      app,
      SERVER_SECRET,
      dir,
      envVars
    );
    port = newPort;
    working_dir = newWorkingDir;
    goosedProcess = newGoosedProcess;
  }

  // Create window config with loading state for recipe deeplinks
  let isLoadingRecipe = false;
  if (!recipe && recipeDeeplink) {
    isLoadingRecipe = true;
    console.log('[Main] Creating window with recipe loading state for deeplink:', recipeDeeplink);
  }

  // Load and manage window state
  const mainWindowState = windowStateKeeper({
    defaultWidth: 940, // large enough to show the sidebar on launch
    defaultHeight: 800,
  });

  const mainWindow = new BrowserWindow({
    titleBarStyle: process.platform === 'darwin' ? 'hidden' : 'default',
    trafficLightPosition: process.platform === 'darwin' ? { x: 20, y: 16 } : undefined,
    vibrancy: process.platform === 'darwin' ? 'window' : undefined,
    frame: process.platform !== 'darwin',
    x: mainWindowState.x,
    y: mainWindowState.y,
    width: mainWindowState.width,
    height: mainWindowState.height,
    minWidth: 750,
    resizable: true,
    useContentSize: true,
    icon: path.join(__dirname, '../images/icon'),
    webPreferences: {
      spellcheck: true,
      preload: path.join(__dirname, 'preload.js'),
      // Enable features needed for Web Speech API
      webSecurity: true,
      nodeIntegration: false,
      contextIsolation: true,
      additionalArguments: [
        JSON.stringify({
          ...appConfig, // Use the potentially updated appConfig
          GOOSE_PORT: port, // Ensure this specific window gets the correct port
          GOOSE_WORKING_DIR: working_dir,
          REQUEST_DIR: dir,
          GOOSE_BASE_URL_SHARE: sharingUrl,
          GOOSE_VERSION: gooseVersion,
          recipe: recipe,
        }),
      ],
      partition: 'persist:goose', // Add this line to ensure persistence
    },
  });

  // Let windowStateKeeper manage the window
  mainWindowState.manage(mainWindow);

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
  mainWindow.webContents.on('context-menu', (_event, params) => {
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
    recipe: recipe,
  };

  // We need to wait for the window to load before we can access localStorage
  mainWindow.webContents.on('did-finish-load', () => {
    const configStr = JSON.stringify(windowConfig).replace(/'/g, "\\'");
    mainWindow.webContents
      .executeJavaScript(
        `
      (function() {
        function setConfig() {
          try {
            if (window.localStorage) {
              localStorage.setItem('gooseConfig', '${configStr}');
              return true;
            }
          } catch (e) {
            console.warn('localStorage access failed:', e);
          }
          return false;
        }

        if (!setConfig()) {
          setTimeout(() => {
            if (!setConfig()) {
              console.error('Failed to set localStorage after retry - continuing without localStorage config');
            }
          }, 100);
        }
      })();
    `
      )
      .catch((error) => {
        console.error('Failed to execute localStorage script:', error);
      });
  });

  // Handle new window creation for links
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    // Open all links in external browser
    if (url.startsWith('http:') || url.startsWith('https:')) {
      shell.openExternal(url);
      return { action: 'deny' };
    }
    return { action: 'allow' };
  });

  // Handle new-window events (alternative approach for external links)
  mainWindow.webContents.on('new-window', (event, url) => {
    event.preventDefault();
    shell.openExternal(url);
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

  // Increment window counter to track number of windows
  const windowId = ++windowCounter;

  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(`${MAIN_WINDOW_VITE_DEV_SERVER_URL}${queryParams}`);
  } else {
    // In production, we need to use a proper file protocol URL with correct base path
    const indexPath = path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`);
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

  mainWindow.on('app-command', (e, cmd) => {
    if (cmd === 'browser-backward') {
      mainWindow.webContents.send('mouse-back-button-clicked');
      e.preventDefault();
    }
  });

  mainWindow.webContents.on('mouse-up', (_event: MouseUpEvent, mouseButton: number) => {
    // MouseButton 3 is the back button.
    if (mouseButton === 3) {
      mainWindow.webContents.send('mouse-back-button-clicked');
    }
  });

  windowMap.set(windowId, mainWindow);

  // Handle recipe decoding in the background after window is created
  if (isLoadingRecipe && recipeDeeplink) {
    console.log('[Main] Starting background recipe decoding for:', recipeDeeplink);

    // Decode recipe asynchronously after window is created
    decodeRecipeMain(recipeDeeplink, port)
      .then((decodedRecipe) => {
        if (decodedRecipe) {
          console.log('[Main] Recipe decoded successfully, updating window config');

          // Handle scheduled job parameters if present
          if (scheduledJobId) {
            decodedRecipe.scheduledJobId = scheduledJobId;
            decodedRecipe.isScheduledExecution = true;
          }

          // Update the window config with the decoded recipe
          const updatedConfig = {
            ...windowConfig,
            recipe: decodedRecipe,
          };

          // Send the decoded recipe to the renderer process
          mainWindow.webContents.send('recipe-decoded', decodedRecipe);

          // Update localStorage with the decoded recipe
          const configStr = JSON.stringify(updatedConfig).replace(/'/g, "\\'");
          mainWindow.webContents
            .executeJavaScript(
              `
            try {
              localStorage.setItem('gooseConfig', '${configStr}');
              console.log('[Renderer] Recipe decoded and config updated');
            } catch (e) {
              console.error('[Renderer] Failed to update config with decoded recipe:', e);
            }
          `
            )
            .catch((error) => {
              console.error('[Main] Failed to update localStorage with decoded recipe:', error);
            });
        } else {
          console.error('[Main] Failed to decode recipe from deeplink');
          // Send error to renderer
          mainWindow.webContents.send('recipe-decode-error', 'Failed to decode recipe');
        }
      })
      .catch((error) => {
        console.error('[Main] Error decoding recipe:', error);
        // Send error to renderer
        mainWindow.webContents.send('recipe-decode-error', error.message || 'Unknown error');
      });
  }

  // Handle window closure
  mainWindow.on('closed', () => {
    windowMap.delete(windowId);

    if (windowPowerSaveBlockers.has(windowId)) {
      const blockerId = windowPowerSaveBlockers.get(windowId)!;
      try {
        powerSaveBlocker.stop(blockerId);
        console.log(
          `[Main] Stopped power save blocker ${blockerId} for closing window ${windowId}`
        );
      } catch (error) {
        console.error(
          `[Main] Failed to stop power save blocker ${blockerId} for window ${windowId}:`,
          error
        );
      }
      windowPowerSaveBlockers.delete(windowId);
    }

    if (goosedProcess && typeof goosedProcess === 'object' && 'kill' in goosedProcess) {
      goosedProcess.kill();
    }
  });
  return mainWindow;
};

// Track tray instance
let tray: Tray | null = null;

const destroyTray = () => {
  if (tray) {
    tray.destroy();
    tray = null;
  }
};

const createTray = () => {
  // If tray already exists, destroy it first
  destroyTray();

  const isDev = process.env.NODE_ENV === 'development';
  let iconPath: string;

  if (isDev) {
    iconPath = path.join(process.cwd(), 'src', 'images', 'iconTemplate.png');
  } else {
    iconPath = path.join(process.resourcesPath, 'images', 'iconTemplate.png');
  }

  tray = new Tray(iconPath);

  // Set tray reference for auto-updater
  setTrayRef(tray);

  // Initially build menu based on update status
  updateTrayMenu(getUpdateAvailable());

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
    await createChat(app, undefined, openDir || undefined);
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

const openDirectoryDialog = async (
  replaceWindow: boolean = false
): Promise<OpenDialogReturnValue> => {
  // Get the current working directory from the focused window
  let defaultPath: string | undefined;
  const currentWindow = BrowserWindow.getFocusedWindow();

  if (currentWindow) {
    try {
      const currentWorkingDir = await currentWindow.webContents.executeJavaScript(
        `window.appConfig ? window.appConfig.get('GOOSE_WORKING_DIR') : null`
      );

      if (currentWorkingDir && typeof currentWorkingDir === 'string') {
        // Verify the directory exists before using it as default
        try {
          const stats = fsSync.lstatSync(currentWorkingDir);
          if (stats.isDirectory()) {
            defaultPath = currentWorkingDir;
          }
        } catch (error) {
          if (error && typeof error === 'object' && 'code' in error) {
            const fsError = error as { code?: string; message?: string };
            if (
              fsError.code === 'ENOENT' ||
              fsError.code === 'EACCES' ||
              fsError.code === 'EPERM'
            ) {
              console.warn(
                `Current working directory not accessible (${fsError.code}): ${currentWorkingDir}, falling back to home directory`
              );
              defaultPath = os.homedir();
            } else {
              console.warn(
                `Unexpected filesystem error (${fsError.code}) for directory ${currentWorkingDir}:`,
                fsError.message
              );
              defaultPath = os.homedir();
            }
          } else {
            console.warn(`Unexpected error checking directory ${currentWorkingDir}:`, error);
            defaultPath = os.homedir();
          }
        }
      }
    } catch (error) {
      console.warn('Failed to get current working directory from window:', error);
    }
  }

  if (!defaultPath) {
    defaultPath = os.homedir();
  }

  const result = (await dialog.showOpenDialog({
    properties: ['openFile', 'openDirectory', 'createDirectory'],
    defaultPath: defaultPath,
  })) as unknown as OpenDialogReturnValue;

  if (!result.canceled && result.filePaths.length > 0) {
    const selectedPath = result.filePaths[0];

    // If a file was selected, use its parent directory
    let dirToAdd = selectedPath;
    try {
      const stats = fsSync.lstatSync(selectedPath);

      // Reject symlinks for security
      if (stats.isSymbolicLink()) {
        console.warn(`Selected path is a symlink, using parent directory for security`);
        dirToAdd = path.dirname(selectedPath);
      } else if (stats.isFile()) {
        dirToAdd = path.dirname(selectedPath);
      }
    } catch (error) {
      console.warn(`Could not stat selected path, using parent directory`);
      dirToAdd = path.dirname(selectedPath); // Fallback to parent directory
    }

    addRecentDir(dirToAdd);
    const currentWindow = BrowserWindow.getFocusedWindow();

    if (replaceWindow && currentWindow) {
      // Replace current window with new one
      await createChat(app, undefined, dirToAdd);
      currentWindow.close();
    } else {
      // Update the working directory in the current window's localStorage
      if (currentWindow) {
        try {
          const updateConfigScript = `
            try {
              const currentConfig = JSON.parse(localStorage.getItem('gooseConfig') || '{}');
              const updatedConfig = {
                ...currentConfig,
                GOOSE_WORKING_DIR: '${dirToAdd.replace(/'/g, "\\'")}',
              };
              localStorage.setItem('gooseConfig', JSON.stringify(updatedConfig));
              
              // Trigger a config update event so the UI can refresh
              window.dispatchEvent(new CustomEvent('goose-config-updated', { 
                detail: { GOOSE_WORKING_DIR: '${dirToAdd.replace(/'/g, "\\'")}' } 
              }));
            } catch (e) {
              console.error('Failed to update working directory in localStorage:', e);
            }
          `;
          await currentWindow.webContents.executeJavaScript(updateConfigScript);
          console.log(`Updated working directory to: ${dirToAdd}`);
        } catch (error) {
          console.error('Failed to update working directory:', error);
          // Fallback: create new window
          await createChat(app, undefined, dirToAdd);
        }
      } else {
        // No current window, create new one
        await createChat(app, undefined, dirToAdd);
      }
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

// Handle scheduling engine settings
ipcMain.handle('get-settings', () => {
  try {
    return loadSettings();
  } catch (error) {
    console.error('Error getting settings:', error);
    return null;
  }
});

ipcMain.handle('get-secret-key', () => {
  return SERVER_SECRET;
});

ipcMain.handle('set-scheduling-engine', async (_event, engine: string) => {
  try {
    const settings = loadSettings();
    settings.schedulingEngine = engine as SchedulingEngine;
    saveSettings(settings);

    // Update the environment variable immediately
    updateSchedulingEngineEnvironment(settings.schedulingEngine);

    return true;
  } catch (error) {
    console.error('Error setting scheduling engine:', error);
    return false;
  }
});

// Handle menu bar icon visibility
ipcMain.handle('set-menu-bar-icon', async (_event, show: boolean) => {
  try {
    const settings = loadSettings();
    settings.showMenuBarIcon = show;
    saveSettings(settings);

    if (show) {
      createTray();
    } else {
      destroyTray();
    }
    return true;
  } catch (error) {
    console.error('Error setting menu bar icon:', error);
    return false;
  }
});

ipcMain.handle('get-menu-bar-icon-state', () => {
  try {
    const settings = loadSettings();
    return settings.showMenuBarIcon ?? true;
  } catch (error) {
    console.error('Error getting menu bar icon state:', error);
    return true;
  }
});

// Handle dock icon visibility (macOS only)
ipcMain.handle('set-dock-icon', async (_event, show: boolean) => {
  try {
    if (process.platform !== 'darwin') return false;

    const settings = loadSettings();
    settings.showDockIcon = show;
    saveSettings(settings);

    if (show) {
      await app.dock.show();
    } else {
      // Only hide the dock if we have a menu bar icon to maintain accessibility
      if (settings.showMenuBarIcon) {
        app.dock.hide();
        setTimeout(() => {
          focusWindow();
        }, 50);
      }
    }
    return true;
  } catch (error) {
    console.error('Error setting dock icon:', error);
    return false;
  }
});

ipcMain.handle('get-dock-icon-state', () => {
  try {
    if (process.platform !== 'darwin') return true;
    const settings = loadSettings();
    return settings.showDockIcon ?? true;
  } catch (error) {
    console.error('Error getting dock icon state:', error);
    return true;
  }
});

// Handle opening system notifications preferences
ipcMain.handle('open-notifications-settings', async () => {
  try {
    if (process.platform === 'darwin') {
      spawn('open', ['x-apple.systempreferences:com.apple.preference.notifications']);
      return true;
    } else if (process.platform === 'win32') {
      // Windows: Open notification settings in Settings app
      spawn('ms-settings:notifications', { shell: true });
      return true;
    } else if (process.platform === 'linux') {
      // Linux: Try different desktop environments
      // GNOME
      try {
        spawn('gnome-control-center', ['notifications']);
        return true;
      } catch (gnomeError) {
        console.log('GNOME control center not found, trying other options');
      }

      // KDE Plasma
      try {
        spawn('systemsettings5', ['kcm_notifications']);
        return true;
      } catch (kdeError) {
        console.log('KDE systemsettings5 not found, trying other options');
      }

      // XFCE
      try {
        spawn('xfce4-settings-manager', ['--socket-id=notifications']);
        return true;
      } catch (xfceError) {
        console.log('XFCE settings manager not found, trying other options');
      }

      // Fallback: Try to open general settings
      try {
        spawn('gnome-control-center');
        return true;
      } catch (fallbackError) {
        console.warn('Could not find a suitable settings application for Linux');
        return false;
      }
    } else {
      console.warn(
        `Opening notification settings is not supported on platform: ${process.platform}`
      );
      return false;
    }
  } catch (error) {
    console.error('Error opening notification settings:', error);
    return false;
  }
});

// Handle quit confirmation setting
ipcMain.handle('set-quit-confirmation', async (_event, show: boolean) => {
  try {
    const settings = loadSettings();
    settings.showQuitConfirmation = show;
    saveSettings(settings);
    return true;
  } catch (error) {
    console.error('Error setting quit confirmation:', error);
    return false;
  }
});

ipcMain.handle('get-quit-confirmation-state', () => {
  try {
    const settings = loadSettings();
    return settings.showQuitConfirmation ?? true;
  } catch (error) {
    console.error('Error getting quit confirmation state:', error);
    return true;
  }
});

// Handle wakelock setting
ipcMain.handle('set-wakelock', async (_event, enable: boolean) => {
  try {
    const settings = loadSettings();
    settings.enableWakelock = enable;
    saveSettings(settings);

    // Stop all existing power save blockers when disabling the setting
    if (!enable) {
      for (const [windowId, blockerId] of windowPowerSaveBlockers.entries()) {
        try {
          powerSaveBlocker.stop(blockerId);
          console.log(
            `[Main] Stopped power save blocker ${blockerId} for window ${windowId} due to wakelock setting disabled`
          );
        } catch (error) {
          console.error(
            `[Main] Failed to stop power save blocker ${blockerId} for window ${windowId}:`,
            error
          );
        }
      }
      windowPowerSaveBlockers.clear();
    }

    return true;
  } catch (error) {
    console.error('Error setting wakelock:', error);
    return false;
  }
});

ipcMain.handle('get-wakelock-state', () => {
  try {
    const settings = loadSettings();
    return settings.enableWakelock ?? false;
  } catch (error) {
    console.error('Error getting wakelock state:', error);
    return false;
  }
});

// Add file/directory selection handler
ipcMain.handle('select-file-or-directory', async (_event, defaultPath?: string) => {
  const dialogOptions: OpenDialogOptions = {
    properties: process.platform === 'darwin' ? ['openFile', 'openDirectory'] : ['openFile'],
  };

  // Set default path if provided
  if (defaultPath) {
    // Expand tilde to home directory
    const expandedPath = expandTilde(defaultPath);

    // Check if the path exists
    try {
      const stats = await fs.stat(expandedPath);
      if (stats.isDirectory()) {
        dialogOptions.defaultPath = expandedPath;
      } else {
        dialogOptions.defaultPath = path.dirname(expandedPath);
      }
    } catch (error) {
      // If path doesn't exist, fall back to home directory and log error
      console.error(`Default path does not exist: ${expandedPath}, falling back to home directory`);
      dialogOptions.defaultPath = os.homedir();
    }
  }

  const result = (await dialog.showOpenDialog(dialogOptions)) as unknown as OpenDialogReturnValue;

  if (!result.canceled && result.filePaths.length > 0) {
    return result.filePaths[0];
  }
  return null;
});

// IPC handler to save data URL to a temporary file
ipcMain.handle('save-data-url-to-temp', async (_event, dataUrl: string, uniqueId: string) => {
  console.log(`[Main] Received save-data-url-to-temp for ID: ${uniqueId}`);
  try {
    // Input validation for uniqueId - only allow alphanumeric characters and hyphens
    if (!uniqueId || !/^[a-zA-Z0-9-]+$/.test(uniqueId) || uniqueId.length > 50) {
      console.error('[Main] Invalid uniqueId format received.');
      return { id: uniqueId, error: 'Invalid uniqueId format' };
    }

    // Input validation for dataUrl
    if (!dataUrl || typeof dataUrl !== 'string' || dataUrl.length > 10 * 1024 * 1024) {
      // 10MB limit
      console.error('[Main] Invalid or too large data URL received.');
      return { id: uniqueId, error: 'Invalid or too large data URL' };
    }

    const tempDir = await ensureTempDirExists();
    const matches = dataUrl.match(/^data:(image\/(png|jpeg|jpg|gif|webp));base64,(.*)$/);

    if (!matches || matches.length < 4) {
      console.error('[Main] Invalid data URL format received.');
      return { id: uniqueId, error: 'Invalid data URL format or unsupported image type' };
    }

    const imageExtension = matches[2]; // e.g., "png", "jpeg"
    const base64Data = matches[3];

    // Validate base64 data
    if (!base64Data || !/^[A-Za-z0-9+/]*={0,2}$/.test(base64Data)) {
      console.error('[Main] Invalid base64 data received.');
      return { id: uniqueId, error: 'Invalid base64 data' };
    }

    const buffer = Buffer.from(base64Data, 'base64');

    // Validate image size (max 5MB)
    if (buffer.length > 5 * 1024 * 1024) {
      console.error('[Main] Image too large.');
      return { id: uniqueId, error: 'Image too large (max 5MB)' };
    }

    const randomString = crypto.randomBytes(8).toString('hex');
    const fileName = `pasted-${uniqueId}-${randomString}.${imageExtension}`;
    const filePath = path.join(tempDir, fileName);

    // Ensure the resolved path is still within the temp directory
    const resolvedPath = path.resolve(filePath);
    const resolvedTempDir = path.resolve(tempDir);
    if (!resolvedPath.startsWith(resolvedTempDir + path.sep)) {
      console.error('[Main] Attempted path traversal detected.');
      return { id: uniqueId, error: 'Invalid file path' };
    }

    await fs.writeFile(filePath, buffer);
    console.log(`[Main] Saved image for ID ${uniqueId} to: ${filePath}`);
    return { id: uniqueId, filePath: filePath };
  } catch (error) {
    console.error(`[Main] Failed to save image to temp for ID ${uniqueId}:`, error);
    return { id: uniqueId, error: error instanceof Error ? error.message : 'Failed to save image' };
  }
});

// IPC handler to serve temporary image files
ipcMain.handle('get-temp-image', async (_event, filePath: string) => {
  console.log(`[Main] Received get-temp-image for path: ${filePath}`);

  // Input validation
  if (!filePath || typeof filePath !== 'string') {
    console.warn('[Main] Invalid file path provided for image serving');
    return null;
  }

  // Ensure the path is within the designated temp directory
  const resolvedPath = path.resolve(filePath);
  const resolvedTempDir = path.resolve(gooseTempDir);

  if (!resolvedPath.startsWith(resolvedTempDir + path.sep)) {
    console.warn(`[Main] Attempted to access file outside designated temp directory: ${filePath}`);
    return null;
  }

  try {
    // Check if it's a regular file first, before trying realpath
    const stats = await fs.lstat(filePath);
    if (!stats.isFile()) {
      console.warn(`[Main] Not a regular file, refusing to serve: ${filePath}`);
      return null;
    }

    // Get the real paths for both the temp directory and the file to handle symlinks properly
    let realTempDir: string;
    let actualPath = filePath;

    try {
      realTempDir = await fs.realpath(gooseTempDir);
      const realPath = await fs.realpath(filePath);

      // Double-check that the real path is still within our real temp directory
      if (!realPath.startsWith(realTempDir + path.sep)) {
        console.warn(
          `[Main] Real path is outside designated temp directory: ${realPath} not in ${realTempDir}`
        );
        return null;
      }
      actualPath = realPath;
    } catch (realpathError) {
      // If realpath fails, use the original path validation
      console.log(
        `[Main] realpath failed for ${filePath}, using original path validation:`,
        realpathError instanceof Error ? realpathError.message : String(realpathError)
      );
    }

    // Read the file and return as base64 data URL
    const fileBuffer = await fs.readFile(actualPath);
    const fileExtension = path.extname(actualPath).toLowerCase().substring(1);

    // Validate file extension
    const allowedExtensions = ['png', 'jpg', 'jpeg', 'gif', 'webp'];
    if (!allowedExtensions.includes(fileExtension)) {
      console.warn(`[Main] Unsupported file extension: ${fileExtension}`);
      return null;
    }

    const mimeType = fileExtension === 'jpg' ? 'image/jpeg' : `image/${fileExtension}`;
    const base64Data = fileBuffer.toString('base64');
    const dataUrl = `data:${mimeType};base64,${base64Data}`;

    console.log(`[Main] Served temp image: ${filePath}`);
    return dataUrl;
  } catch (error) {
    console.error(`[Main] Failed to serve temp image: ${filePath}`, error);
    return null;
  }
});
ipcMain.on('delete-temp-file', async (_event, filePath: string) => {
  console.log(`[Main] Received delete-temp-file for path: ${filePath}`);

  // Input validation
  if (!filePath || typeof filePath !== 'string') {
    console.warn('[Main] Invalid file path provided for deletion');
    return;
  }

  // Ensure the path is within the designated temp directory
  const resolvedPath = path.resolve(filePath);
  const resolvedTempDir = path.resolve(gooseTempDir);

  if (!resolvedPath.startsWith(resolvedTempDir + path.sep)) {
    console.warn(`[Main] Attempted to delete file outside designated temp directory: ${filePath}`);
    return;
  }

  try {
    // Check if it's a regular file first, before trying realpath
    const stats = await fs.lstat(filePath);
    if (!stats.isFile()) {
      console.warn(`[Main] Not a regular file, refusing to delete: ${filePath}`);
      return;
    }

    // Get the real paths for both the temp directory and the file to handle symlinks properly
    let actualPath = filePath;

    try {
      const realTempDir = await fs.realpath(gooseTempDir);
      const realPath = await fs.realpath(filePath);

      // Double-check that the real path is still within our real temp directory
      if (!realPath.startsWith(realTempDir + path.sep)) {
        console.warn(
          `[Main] Real path is outside designated temp directory: ${realPath} not in ${realTempDir}`
        );
        return;
      }
      actualPath = realPath;
    } catch (realpathError) {
      // If realpath fails, use the original path validation
      console.log(
        `[Main] realpath failed for ${filePath}, using original path validation:`,
        realpathError instanceof Error ? realpathError.message : String(realpathError)
      );
    }

    await fs.unlink(actualPath);
    console.log(`[Main] Deleted temp file: ${filePath}`);
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code !== 'ENOENT') {
      // ENOENT means file doesn't exist, which is fine
      console.error(`[Main] Failed to delete temp file: ${filePath}`, error);
    } else {
      console.log(`[Main] Temp file already deleted or not found: ${filePath}`);
    }
  }
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
    // Expand tilde to home directory
    const expandedPath = expandTilde(filePath);

    const cat = spawn('cat', [expandedPath]);
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
        resolve({ file: '', filePath: expandedPath, error: errorOutput || null, found: false });
        return;
      }
      resolve({ file: output, filePath: expandedPath, error: null, found: true });
    });

    cat.on('error', (error) => {
      console.error('Error reading file:', error);
      resolve({ file: '', filePath: expandedPath, error, found: false });
    });
  });
});

ipcMain.handle('write-file', (_event, filePath, content) => {
  return new Promise((resolve) => {
    // Expand tilde to home directory
    const expandedPath = expandTilde(filePath);

    // Create a write stream to the file
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const fsNode = require('fs'); // Using require for fs in this specific handler from original
    try {
      fsNode.writeFileSync(expandedPath, content, { encoding: 'utf8' });
      resolve(true);
    } catch (error) {
      console.error('Error writing to file:', error);
      resolve(false);
    }
  });
});

// Enhanced file operations
ipcMain.handle('ensure-directory', async (_event, dirPath) => {
  try {
    // Expand tilde to home directory
    const expandedPath = expandTilde(dirPath);

    await fs.mkdir(expandedPath, { recursive: true });
    return true;
  } catch (error) {
    console.error('Error creating directory:', error);
    return false;
  }
});

ipcMain.handle('list-files', async (_event, dirPath, extension) => {
  try {
    // Expand tilde to home directory
    const expandedPath = expandTilde(dirPath);

    const files = await fs.readdir(expandedPath);
    if (extension) {
      return files.filter((file) => file.endsWith(extension));
    }
    return files;
  } catch (error) {
    console.error('Error listing files:', error);
    return [];
  }
});

// Handle message box dialogs
ipcMain.handle('show-message-box', async (_event, options) => {
  return dialog.showMessageBox(options);
});

ipcMain.handle('get-allowed-extensions', async () => {
  return await getAllowList();
});

const createNewWindow = async (app: App, dir?: string | null) => {
  const recentDirs = loadRecentDirs();
  const openDir = dir || (recentDirs.length > 0 ? recentDirs[0] : undefined);
  return await createChat(app, undefined, openDir);
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
    globalShortcut.register(accelerator, () => {
      focusWindow();
    });

    // Check if the shortcut was registered successfully
    if (globalShortcut.isRegistered(accelerator)) {
      return true;
    } else {
      console.error('Failed to register global hotkey');
      return false;
    }
  } catch (e) {
    console.error('Error registering global hotkey:', e);
    return false;
  }
};

app.whenReady().then(async () => {
  // Ensure Windows shims are available before any MCP processes are spawned
  await ensureWinShims();

  // Register update IPC handlers once (but don't setup auto-updater yet)
  registerUpdateIpcHandlers();

  // Handle microphone permission requests
  session.defaultSession.setPermissionRequestHandler((_webContents, permission, callback) => {
    console.log('Permission requested:', permission);
    // Allow microphone and media access
    if (permission === 'media') {
      callback(true);
    } else {
      // Default behavior for other permissions
      callback(true);
    }
  });

  // Add CSP headers to all sessions
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy':
          "default-src 'self';" +
          // Allow inline styles since we use them in our React components
          "style-src 'self' 'unsafe-inline';" +
          // Scripts from our app and inline scripts (for theme initialization)
          "script-src 'self' 'unsafe-inline';" +
          // Images from our app and data: URLs (for base64 images)
          "img-src 'self' data: https:;" +
          // Connect to our local API and specific external services
          "connect-src 'self' http://127.0.0.1:* https://api.github.com https://github.com https://objects.githubusercontent.com" +
          // Don't allow any plugins
          "object-src 'none';" +
          // Allow all frames (iframes)
          "frame-src 'self' https: http:;" +
          // Font sources - allow self, data URLs, and external fonts
          "font-src 'self' data: https:;" +
          // Media sources - allow microphone
          "media-src 'self' mediastream:;" +
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

  // Create tray if enabled in settings
  const settings = loadSettings();
  if (settings.showMenuBarIcon) {
    createTray();
  }

  // Handle dock icon visibility (macOS only)
  if (process.platform === 'darwin' && !settings.showDockIcon && settings.showMenuBarIcon) {
    app.dock.hide();
  }

  // Parse command line arguments
  const { dirPath } = parseArgs();

  await createNewWindow(app, dirPath);

  // Setup auto-updater AFTER window is created and displayed (with delay to avoid blocking)
  setTimeout(() => {
    if (shouldSetupUpdater()) {
      log.info('Setting up auto-updater after window creation...');
      try {
        setupAutoUpdater();
      } catch (error) {
        log.error('Error setting up auto-updater:', error);
      }
    }
  }, 2000); // 2 second delay after window is shown

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
        accelerator: process.platform === 'darwin' ? 'Command+E' : undefined,
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
      createNewWindow(app);
    }
  });

  ipcMain.on('create-chat-window', (_, query, dir, version, resumeSessionId, recipe, viewType) => {
    if (!dir?.trim()) {
      const recentDirs = loadRecentDirs();
      dir = recentDirs.length > 0 ? recentDirs[0] : undefined;
    }

    // Log the recipe for debugging
    console.log('Creating chat window with recipe:', recipe);

    // Pass recipe as part of viewOptions when viewType is recipeEditor
    createChat(app, query, dir, version, resumeSessionId, recipe, viewType);
  });

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

  ipcMain.handle('start-power-save-blocker', (event) => {
    const window = BrowserWindow.fromWebContents(event.sender);
    const windowId = window?.id;

    if (windowId && !windowPowerSaveBlockers.has(windowId)) {
      const blockerId = powerSaveBlocker.start('prevent-app-suspension');
      windowPowerSaveBlockers.set(windowId, blockerId);
      console.log(`[Main] Started power save blocker ${blockerId} for window ${windowId}`);
      return true;
    }

    if (windowId && windowPowerSaveBlockers.has(windowId)) {
      console.log(`[Main] Power save blocker already active for window ${windowId}`);
    }

    return false;
  });

  ipcMain.handle('stop-power-save-blocker', (event) => {
    const window = BrowserWindow.fromWebContents(event.sender);
    const windowId = window?.id;

    if (windowId && windowPowerSaveBlockers.has(windowId)) {
      const blockerId = windowPowerSaveBlockers.get(windowId)!;
      powerSaveBlocker.stop(blockerId);
      windowPowerSaveBlockers.delete(windowId);
      console.log(`[Main] Stopped power save blocker ${blockerId} for window ${windowId}`);
      return true;
    }

    return false;
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
      console.error('Error opening URL in browser:', error);
    }
  });

  // Handle app restart
  ipcMain.on('restart-app', () => {
    app.relaunch();
    app.exit(0);
  });

  // Handler for getting app version
  ipcMain.on('get-app-version', (event) => {
    event.returnValue = app.getVersion();
  });

  ipcMain.handle('open-directory-in-explorer', async (_event, path: string) => {
    try {
      return !!(await shell.openPath(path));
    } catch (error) {
      console.error('Error opening directory in explorer:', error);
      return false;
    }
  });
});

async function getAllowList(): Promise<string[]> {
  if (!process.env.GOOSE_ALLOWLIST) {
    return [];
  }

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
}

app.on('will-quit', async () => {
  for (const [windowId, blockerId] of windowPowerSaveBlockers.entries()) {
    try {
      powerSaveBlocker.stop(blockerId);
      console.log(
        `[Main] Stopped power save blocker ${blockerId} for window ${windowId} during app quit`
      );
    } catch (error) {
      console.error(
        `[Main] Failed to stop power save blocker ${blockerId} for window ${windowId}:`,
        error
      );
    }
  }
  windowPowerSaveBlockers.clear();

  // Unregister all shortcuts when quitting
  globalShortcut.unregisterAll();

  try {
    await fs.access(gooseTempDir); // Check if directory exists to avoid error on fs.rm if it doesn't

    // First, check for any symlinks in the directory and refuse to delete them
    let hasSymlinks = false;
    try {
      const files = await fs.readdir(gooseTempDir);
      for (const file of files) {
        const filePath = path.join(gooseTempDir, file);
        const stats = await fs.lstat(filePath);
        if (stats.isSymbolicLink()) {
          console.warn(`[Main] Found symlink in temp directory: ${filePath}. Skipping deletion.`);
          hasSymlinks = true;
          // Delete the individual file but leave the symlink
          continue;
        }

        // Delete regular files individually
        if (stats.isFile()) {
          await fs.unlink(filePath);
        }
      }

      // If no symlinks were found, it's safe to remove the directory
      if (!hasSymlinks) {
        await fs.rm(gooseTempDir, { recursive: true, force: true });
        console.log('[Main] Pasted images temp directory cleaned up successfully.');
      } else {
        console.log(
          '[Main] Cleaned up files in temp directory but left directory intact due to symlinks.'
        );
      }
    } catch (err) {
      console.error('[Main] Error while cleaning up temp directory contents:', err);
    }
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code === 'ENOENT') {
      console.log('[Main] Temp directory did not exist during "will-quit", no cleanup needed.');
    } else {
      console.error(
        '[Main] Failed to clean up pasted images temp directory during "will-quit":',
        error
      );
    }
  }
});

// Quit when all windows are closed, except on macOS or if we have a tray icon.
// Add confirmation dialog when quitting with Cmd+Q (skip in dev mode)
app.on('before-quit', async (event) => {
  // Skip confirmation dialog in development mode
  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    return; // Allow normal quit behavior in dev mode
  }

  // Check if quit confirmation is enabled in settings
  const settings = loadSettings();
  if (!settings.showQuitConfirmation) {
    return; // Allow normal quit behavior if confirmation is disabled
  }

  // Prevent the default quit behavior
  event.preventDefault();

  // Show confirmation dialog
  try {
    const result = (await dialog.showMessageBox({
      type: 'question',
      buttons: ['Quit', 'Cancel'],
      defaultId: 1, // Default to Cancel
      title: 'Confirm Quit',
      message: 'Are you sure you want to quit Goose?',
      detail: 'Any unsaved changes may be lost.',
    })) as unknown as { response: number };

    if (result.response === 0) {
      // User clicked "Quit"
      // Set a flag to avoid showing the dialog again
      app.removeAllListeners('before-quit');
      // Force quit the app
      process.nextTick(() => {
        app.exit(0);
      });
    }
  } catch (error) {
    console.error('Error showing quit dialog:', error);
  }
});

app.on('window-all-closed', () => {
  // Only quit if we're not on macOS or don't have a tray icon
  if (process.platform !== 'darwin' || !tray) {
    app.quit();
  }
});
