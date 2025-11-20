import type { OpenDialogOptions, OpenDialogReturnValue } from 'electron';
import {
  app,
  App,
  BrowserWindow,
  BrowserView,
  dialog,
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
import { pathToFileURL, format as formatUrl, URLSearchParams } from 'node:url';
import { Buffer } from 'node:buffer';
import fs from 'node:fs/promises';
import fsSync from 'node:fs';
import started from 'electron-squirrel-startup';
import path from 'node:path';
import os from 'node:os';
import { spawn } from 'child_process';
import 'dotenv/config';
import { checkServerStatus, startGoosed } from './goosed';
import { expandTilde, getBinaryPath } from './utils/pathUtils';
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
import { decodeRecipe } from './api';
import { Client, createClient, createConfig } from './api/client';

async function decodeRecipeMain(client: Client, deeplink: string): Promise<Recipe | null> {
  try {
    return (
      await decodeRecipe({
        client,
        throwOnError: true,
        body: { deeplink },
      })
    ).data.recipe;
  } catch (e) {
    console.error('Failed to decode recipe:', e);
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
    } catch {
      console.warn('[Main] Could not reset protocol handler');
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

            const recipeDeeplink = parseRecipeDeeplink(protocolUrl);
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

let windowDeeplinkURL: string | null = null;

app.on('open-url', async (_event, url) => {
  if (process.platform !== 'win32') {
    const parsedUrl = new URL(url);
    const recentDirs = loadRecentDirs();
    const openDir = recentDirs.length > 0 ? recentDirs[0] : null;

    // Handle bot/recipe URLs by directly creating a new window
    console.log('[Main] Received open-url event:', url);
    if (parsedUrl.hostname === 'bot' || parsedUrl.hostname === 'recipe') {
      console.log('[Main] Detected bot/recipe URL, creating new chat window');
      let recipeDeeplink = parseRecipeDeeplink(url);
      if (recipeDeeplink) {
        windowDeeplinkURL = url;
      }
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

// Handle multiple files/folders (macOS only)
if (process.platform === 'darwin') {
  // Use type assertion for non-standard Electron event
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  app.on('open-files' as any, async (event: any, filePaths: string[]) => {
    event.preventDefault();
    for (const filePath of filePaths) {
      await handleFileOpen(filePath);
    }
  });
}

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
  } catch {
    console.error('Failed to handle file open');

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
  let dirPath = null;

  // Remove first two elements in dev mode (electron and script path)
  const args = !dirPath && app.isPackaged ? process.argv : process.argv.slice(2);
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--dir' && i + 1 < args.length) {
      dirPath = args[i + 1];
      break;
    }
  }

  return { dirPath };
};

interface BundledConfig {
  defaultProvider?: string;
  defaultModel?: string;
  predefinedModels?: string;
  baseUrlShare?: string;
  version?: string;
}

const getBundledConfig = (): BundledConfig => {
  //{env-macro-start}//
  //needed when goose is bundled for a specific provider
  //{env-macro-end}//
  return {
    defaultProvider: process.env.GOOSE_DEFAULT_PROVIDER,
    defaultModel: process.env.GOOSE_DEFAULT_MODEL,
    predefinedModels: process.env.GOOSE_PREDEFINED_MODELS,
    baseUrlShare: process.env.GOOSE_BASE_URL_SHARE,
    version: process.env.GOOSE_VERSION,
  };
};

const { defaultProvider, defaultModel, predefinedModels, baseUrlShare, version } =
  getBundledConfig();

const SERVER_SECRET = process.env.GOOSE_EXTERNAL_BACKEND
  ? 'test'
  : crypto.randomBytes(32).toString('hex');

let appConfig = {
  GOOSE_DEFAULT_PROVIDER: defaultProvider,
  GOOSE_DEFAULT_MODEL: defaultModel,
  GOOSE_PREDEFINED_MODELS: predefinedModels,
  GOOSE_API_HOST: 'http://127.0.0.1',
  GOOSE_PORT: 0,
  GOOSE_WORKING_DIR: '',
  // If GOOSE_ALLOWLIST_WARNING env var is not set, defaults to false (strict blocking mode)
  GOOSE_ALLOWLIST_WARNING: process.env.GOOSE_ALLOWLIST_WARNING === 'true',
};

const windowMap = new Map<number, BrowserWindow>();

const goosedClients = new Map<number, Client>();

// Track power save blockers per window
const windowPowerSaveBlockers = new Map<number, number>(); // windowId -> blockerId

// Track running app processes globally so we can clean them up on quit
const runningApps = new Map<string, { process: any; port?: number }>();

// Helper function to clean up all BrowserViews for a window
const cleanupBrowserViews = (window: BrowserWindow) => {
  try {
    if ((window as any).browserViews) {
      const browserViews = (window as any).browserViews as Map<string, BrowserView>;
      console.log(`[Main] Cleaning up ${browserViews.size} BrowserViews for window`);
      
      for (const [viewId, view] of browserViews.entries()) {
        try {
          window.removeBrowserView(view);
          (view as any).destroy?.();
          console.log(`[Main] Cleaned up BrowserView: ${viewId}`);
        } catch (error) {
          console.error(`[Main] Error cleaning up BrowserView ${viewId}:`, error);
        }
      }
      
      browserViews.clear();
      delete (window as any).browserViews;
    }
  } catch (error) {
    console.error('[Main] Error during BrowserView cleanup:', error);
  }
};

const createChat = async (
  app: App,
  _query?: string,
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
  let workingDir = '';
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
          workingDir = config.GOOSE_WORKING_DIR;
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
    workingDir = newWorkingDir;
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
    minWidth: 450,
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
          ...appConfig,
          GOOSE_PORT: port,
          GOOSE_WORKING_DIR: workingDir,
          REQUEST_DIR: dir,
          GOOSE_BASE_URL_SHARE: baseUrlShare,
          GOOSE_VERSION: version,
          recipe: recipe,
        }),
      ],
      partition: 'persist:goose', // Add this line to ensure persistence
    },
  });

  const goosedClient = createClient(
    createConfig({
      baseUrl: `http://127.0.0.1:${port}`,
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': SERVER_SECRET,
      },
    })
  );
  goosedClients.set(mainWindow.id, goosedClient);

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
  // Use type assertion for non-standard Electron event
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  mainWindow.webContents.on('new-window' as any, function (event: any, url: string) {
    event.preventDefault();
    shell.openExternal(url);
  });

  const windowId = mainWindow.id;
  const url = MAIN_WINDOW_VITE_DEV_SERVER_URL
    ? new URL(MAIN_WINDOW_VITE_DEV_SERVER_URL)
    : pathToFileURL(path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`));

  let appPath = '/';
  const routeMap: Record<string, string> = {
    chat: '/',
    pair: '/pair',
    settings: '/settings',
    sessions: '/sessions',
    schedules: '/schedules',
    recipes: '/recipes',
    permission: '/permission',
    ConfigureProviders: '/configure-providers',
    sharedSession: '/shared-session',
    recipeEditor: '/recipe-editor',
    welcome: '/welcome',
  };

  if (viewType) {
    appPath = routeMap[viewType] || '/';
  }
  if (appPath === '/' && (recipe !== undefined || recipeDeeplink !== undefined)) {
    appPath = '/pair';
  }

  let searchParams = new URLSearchParams();
  if (resumeSessionId) {
    searchParams.set('resumeSessionId', resumeSessionId);
    if (appPath === '/') {
      appPath = '/pair';
    }
  }

  // Goose's react app uses HashRouter, so the path + search params follow a #/
  url.hash = `${appPath}?${searchParams.toString()}`;
  let formattedUrl = formatUrl(url);
  log.info('Opening URL: ', formattedUrl);
  mainWindow.loadURL(formattedUrl);

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

  // Handle mouse back button (button 3)
  // Use type assertion for non-standard Electron event
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  mainWindow.webContents.on('mouse-up' as any, function (_event: any, mouseButton: number) {
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
    decodeRecipeMain(goosedClient, recipeDeeplink)
      .then((decodedRecipe) => {
        if (decodedRecipe) {
          console.log('[Main] Recipe decoded successfully, updating window config');

          // Handle scheduled job parameters if present
          if (scheduledJobId) {
            decodedRecipe.scheduledJobId = scheduledJobId;
            decodedRecipe.isScheduledExecution = true;
          }

          // Send the decoded recipe to the renderer process
          mainWindow.webContents.send('recipe-decoded', decodedRecipe);
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

  // Clean up BrowserViews when window reloads
  mainWindow.webContents.on('did-start-loading', () => {
    const currentUrl = mainWindow.webContents.getURL();
    console.log('[Main] ⚠️  Window started loading, URL:', currentUrl);
    console.trace('[Main] Window did-start-loading stack trace');
    
    // Only cleanup if this is actually a full page reload/navigation
    // Skip cleanup for hash changes, query parameter changes, or same-page navigation
    if (currentUrl && !currentUrl.includes('#') && !currentUrl.includes('?')) {
      console.log('[Main] Detected full page reload, cleaning up BrowserViews');
      cleanupBrowserViews(mainWindow);
      
      // Also cleanup child windows on full page reload
      const childWindows = childWebViewerWindows.get(windowId);
      if (childWindows && childWindows.size > 0) {
        console.log('[Main] Detected full page reload, cleaning up child webviewer windows');
        for (const [viewerId, childWindow] of childWindows.entries()) {
          try {
            if (!childWindow.isDestroyed()) {
              childWindow.destroy();
            }
            childWindows.delete(viewerId);
            console.log('[Main] Child webviewer window destroyed during reload:', viewerId);
          } catch (error) {
            console.error('[Main] Error destroying child window during reload:', error);
          }
        }
      }
    } else {
      console.log('[Main] Detected same-page navigation, skipping cleanup');
    }
  });

  // Handle window closure
  mainWindow.on('closed', () => {
    windowMap.delete(windowId);

    // Clean up BrowserViews
    cleanupBrowserViews(mainWindow);

    // Clean up child webviewer windows with more robust cleanup
    const childWindows = childWebViewerWindows.get(windowId);
    if (childWindows) {
      console.log(`[Main] Cleaning up ${childWindows.size} child webviewer windows for main window:`, windowId);
      for (const [viewerId, childWindow] of childWindows.entries()) {
        try {
          // Remove all event listeners first to prevent memory leaks
          childWindow.removeAllListeners();
          if (childWindow.webContents && !childWindow.webContents.isDestroyed()) {
            childWindow.webContents.removeAllListeners();
          }
          
          // Force destroy the child window
          if (!childWindow.isDestroyed()) {
            childWindow.destroy();
          }
          console.log('[Main] Cleaned up child webviewer window:', viewerId);
        } catch (error) {
          console.error('[Main] Error cleaning up child webviewer window:', viewerId, error);
        }
      }
      childWindows.clear(); // Clear the map
      childWebViewerWindows.delete(windowId);
    }

    // Clean up dock window if it exists
    if ((mainWindow as any).dockWindow && !(mainWindow as any).dockWindow.isDestroyed()) {
      console.log('[Main] Destroying dock window on main window close');
      (mainWindow as any).dockWindow.destroy();
      (mainWindow as any).dockWindow = null;
    }

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

const openDirectoryDialog = async (): Promise<OpenDialogReturnValue> => {
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
    } catch {
      console.warn(`Could not stat selected path, using parent directory`);
      dirToAdd = path.dirname(selectedPath); // Fallback to parent directory
    }

    addRecentDir(dirToAdd);

    let recipeDeeplink: string | undefined = undefined;
    if (windowDeeplinkURL) {
      recipeDeeplink = parseRecipeDeeplink(windowDeeplinkURL);
    }
    // Create a new window with the selected directory
    await createChat(
      app,
      undefined,
      dirToAdd,
      undefined,
      undefined,
      undefined,
      undefined,
      recipeDeeplink
    );
  }
  return result;
};

function parseRecipeDeeplink(url: string): string | undefined {
  const parsedUrl = new URL(url);
  let recipeDeeplink = parsedUrl.searchParams.get('config');
  if (recipeDeeplink && !url.includes(recipeDeeplink)) {
    // URLSearchParams decodes + as space, which can break encoded configs
    // Parse raw query to preserve "+" characters in values like config
    const search = parsedUrl.search || '';
    // parse recipe deeplink from search params
    const configMatch = search.match(/(?:[?&])config=([^&]*)/);
    // get recipe deeplink from config match
    let recipeDeeplinkTmp = configMatch ? configMatch[1] : null;
    if (recipeDeeplinkTmp) {
      try {
        recipeDeeplink = decodeURIComponent(recipeDeeplinkTmp);
      } catch {
        // Leave as-is if decoding fails
        return undefined;
      }
    }
  }
  if (recipeDeeplink) {
    return recipeDeeplink;
  }
  return undefined;
}

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
  log.info('React ready event received');

  if (pendingDeepLink) {
    log.info('Processing pending deep link:', pendingDeepLink);
    handleProtocolUrl(pendingDeepLink);
  } else {
    log.info('No pending deep link to process');
  }

  // We don't need to handle pending deep links here anymore
  // since we're handling them in the window creation flow
  log.info('React ready - window is prepared for deep links');
});

// Handle external URL opening
ipcMain.handle('open-external', async (_event, url: string) => {
  try {
    await shell.openExternal(url);
    return true;
  } catch (error) {
    console.error('Error opening external URL:', error);
    throw error;
  }
});

// Handle creating a BrowserView for web content
ipcMain.handle('create-browser-view', async (event, url: string, bounds: { x: number; y: number; width: number; height: number }) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) {
      throw new Error('No main window found');
    }

    console.log('[Main] Creating BrowserView with bounds:', bounds);

    // Create a new BrowserView with enhanced compatibility
    const view = new BrowserView({
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        webSecurity: true,
        allowRunningInsecureContent: false,
        experimentalFeatures: true,
        // Enable additional features for better website compatibility
        webgl: true,
        plugins: false,
        java: false,
        // Set a standard user agent to avoid detection issues
        // This helps with sites that might serve different content to Electron
      },
    });

    // Set the view on the window
    mainWindow.setBrowserView(view);
    
    // The bounds from the renderer are already relative to the viewport
    // BrowserView.setBounds() expects coordinates relative to the window's content area
    // No coordinate transformation needed - just validate and constrain the bounds
    const contentBounds = mainWindow.getContentBounds();
    
    const adjustedBounds = {
      x: Math.max(0, Math.min(bounds.x, contentBounds.width - 100)),
      y: Math.max(0, Math.min(bounds.y, contentBounds.height - 100)),
      width: Math.max(100, Math.min(bounds.width, contentBounds.width - bounds.x)),
      height: Math.max(100, Math.min(bounds.height, contentBounds.height - bounds.y)),
    };
    
    console.log('[Main] Adjusted bounds for BrowserView:', adjustedBounds);
    view.setBounds(adjustedBounds);
    
    // Set a standard user agent to improve compatibility
    view.webContents.setUserAgent('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36');
    
    // Load the URL
    await view.webContents.loadURL(url);
    
    // Store view reference with a truly unique ID
    const viewId = `${Date.now()}-${Math.random().toString(36).substr(2, 9)}-${performance.now().toString(36)}`;
    if (!(mainWindow as any).browserViews) {
      (mainWindow as any).browserViews = new Map();
    }
    (mainWindow as any).browserViews.set(viewId, view);
    
    console.log('[Main] BrowserView created successfully with ID:', viewId);
    return { viewId, success: true };
  } catch (error) {
    console.error('Error creating browser view:', error);
    return { viewId: null, success: false, error: error.message };
  }
});

// Handle updating BrowserView bounds
ipcMain.handle('update-browser-view-bounds', async (event, viewId: string, bounds: { x: number; y: number; width: number; height: number }) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return false;
    }

    const view = (mainWindow as any).browserViews.get(viewId);
    if (view) {
      console.log('[Main] Updating BrowserView bounds:', bounds);
      
      // Validate and constrain bounds to ensure proper containment
      const contentBounds = mainWindow.getContentBounds();
      const adjustedBounds = {
        x: Math.max(0, Math.min(bounds.x, contentBounds.width - 100)),
        y: Math.max(0, Math.min(bounds.y, contentBounds.height - 100)),
        width: Math.max(100, Math.min(bounds.width, contentBounds.width - bounds.x)),
        height: Math.max(100, Math.min(bounds.height, contentBounds.height - bounds.y)),
      };
      
      // Only update if bounds are valid
      if (adjustedBounds.width > 0 && adjustedBounds.height > 0) {
        view.setBounds(adjustedBounds);
        return true;
      } else {
        console.warn('[Main] Invalid bounds for BrowserView update:', adjustedBounds);
        return false;
      }
    }
    return false;
  } catch (error) {
    console.error('Error updating browser view bounds:', error);
    return false;
  }
});

// Handle destroying a BrowserView
ipcMain.handle('destroy-browser-view', async (event, viewId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return false;
    }

    const view = (mainWindow as any).browserViews.get(viewId);
    if (view) {
      mainWindow.removeBrowserView(view);
      (view as any).destroy?.();
      (mainWindow as any).browserViews.delete(viewId);
      return true;
    }
    return false;
  } catch (error) {
    console.error('Error destroying browser view:', error);
    return false;
  }
});

// Handle BrowserView navigation
ipcMain.handle('browser-view-navigate', async (event, viewId: string, url: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return false;
    }

    const view = (mainWindow as any).browserViews.get(viewId);
    if (view) {
      await view.webContents.loadURL(url);
      return true;
    }
    return false;
  } catch (error) {
    console.error('Error navigating browser view:', error);
    return false;
  }
});

// Handle BrowserView back/forward navigation
ipcMain.handle('browser-view-go-back', async (event, viewId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return false;
    }

    const view = (mainWindow as any).browserViews.get(viewId);
    if (view && view.webContents.canGoBack()) {
      view.webContents.goBack();
      return true;
    }
    return false;
  } catch (error) {
    console.error('Error going back in browser view:', error);
    return false;
  }
});

ipcMain.handle('browser-view-go-forward', async (event, viewId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return false;
    }

    const view = (mainWindow as any).browserViews.get(viewId);
    if (view && view.webContents.canGoForward()) {
      view.webContents.goForward();
      return true;
    }
    return false;
  } catch (error) {
    console.error('Error going forward in browser view:', error);
    return false;
  }
});

// Handle BrowserView refresh
ipcMain.handle('browser-view-refresh', async (event, viewId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return false;
    }

    const view = (mainWindow as any).browserViews.get(viewId);
    if (view) {
      view.webContents.reload();
      return true;
    }
    return false;
  } catch (error) {
    console.error('Error refreshing browser view:', error);
    return false;
  }
});

// Handle getting BrowserView navigation state
ipcMain.handle('browser-view-navigation-state', async (event, viewId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return { canGoBack: false, canGoForward: false, isLoading: false, url: '' };
    }

    const view = (mainWindow as any).browserViews.get(viewId);
    if (view) {
      return {
        canGoBack: view.webContents.canGoBack(),
        canGoForward: view.webContents.canGoForward(),
        isLoading: view.webContents.isLoading(),
        url: view.webContents.getURL(),
      };
    }
    return { canGoBack: false, canGoForward: false, isLoading: false, url: '' };
  } catch (error) {
    console.error('Error getting browser view navigation state:', error);
    return { canGoBack: false, canGoForward: false, isLoading: false, url: '' };
  }
});

// Handle creating screenshot backdrop for smooth dock interaction
ipcMain.handle('create-iframe-backdrop', async (event) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return { success: false, error: 'No browser views found' };
    }

    const browserViews = (mainWindow as any).browserViews as Map<string, BrowserView>;
    const backdropData = [];
    
    // Initialize storage for original bounds if it doesn't exist
    if (!(mainWindow as any).browserViewBounds) {
      (mainWindow as any).browserViewBounds = new Map();
    }
    
    for (const [viewId, view] of browserViews.entries()) {
      try {
        // Get current bounds
        const currentBounds = view.getBounds();
        
        // Store the bounds for restoration
        (mainWindow as any).browserViewBounds.set(viewId, currentBounds);
        
        // Only create backdrop for visible BrowserViews
        if (currentBounds.width > 100 && currentBounds.height > 100) {
          try {
            // Capture screenshot of the BrowserView
            const screenshot = await view.webContents.capturePage();
            const screenshotDataUrl = `data:image/png;base64,${screenshot.toPNG().toString('base64')}`;
            
            backdropData.push({
              viewId,
              screenshot: screenshotDataUrl,
              bounds: currentBounds
            });
            
            console.log(`[Main] Created screenshot backdrop for BrowserView: ${viewId}`);
          } catch (screenshotError) {
            console.warn(`[Main] Failed to capture screenshot for ${viewId}, skipping backdrop:`, screenshotError);
          }
          
          // Hide the actual BrowserView
          view.setBounds({ x: 0, y: 0, width: 0, height: 0 });
        }
      } catch (error) {
        console.error(`[Main] Error creating backdrop for BrowserView ${viewId}:`, error);
      }
    }
    
    return { success: true, backdropData };
  } catch (error) {
    console.error('Error creating screenshot backdrop:', error);
    return { success: false, error: error.message };
  }
});

// Handle removing iframe backdrop and restoring BrowserViews
ipcMain.handle('remove-iframe-backdrop', async (event) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return false;
    }

    const browserViews = (mainWindow as any).browserViews as Map<string, BrowserView>;
    const storedBounds = (mainWindow as any).browserViewBounds as Map<string, any> || new Map();
    
    for (const [viewId, view] of browserViews.entries()) {
      try {
        // Restore the original bounds directly
        const originalBounds = storedBounds.get(viewId);
        if (originalBounds && originalBounds.width > 100 && originalBounds.height > 100) {
          view.setBounds(originalBounds);
          console.log(`[Main] Restored BrowserView: ${viewId} from backdrop to original bounds:`, originalBounds);
        } else {
          console.warn(`[Main] No valid stored bounds for BrowserView: ${viewId}, keeping hidden`);
        }
        
      } catch (error) {
        console.error(`[Main] Error restoring BrowserView ${viewId}:`, error);
      }
    }
    
    // Don't clear the stored bounds here - keep them for future use
    console.log(`[Main] Restored ${browserViews.size} BrowserViews from screenshot backdrops`);
    return true;
  } catch (error) {
    console.error('Error removing iframe backdrop:', error);
    return false;
  }
});

// Handle hiding browser views (legacy method - kept for compatibility)
ipcMain.handle('hide-browser-views', async (event) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return false;
    }

    const browserViews = (mainWindow as any).browserViews as Map<string, BrowserView>;
    
    // Initialize storage for original bounds if it doesn't exist
    if (!(mainWindow as any).browserViewBounds) {
      (mainWindow as any).browserViewBounds = new Map();
    }
    
    for (const [viewId, view] of browserViews.entries()) {
      try {
        // Store the current bounds before hiding
        const currentBounds = view.getBounds();
        (mainWindow as any).browserViewBounds.set(viewId, currentBounds);
        
        // Hide the BrowserView by setting bounds to zero
        view.setBounds({ x: 0, y: 0, width: 0, height: 0 });
        console.log(`[Main] Hid BrowserView: ${viewId}, stored bounds:`, currentBounds);
      } catch (error) {
        console.error(`[Main] Error hiding BrowserView ${viewId}:`, error);
      }
    }
    return true;
  } catch (error) {
    console.error('Error hiding browser views:', error);
    return false;
  }
});

// Handle showing browser views (restore original bounds directly)
ipcMain.handle('show-browser-views', async (event) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).browserViews) {
      return false;
    }

    const browserViews = (mainWindow as any).browserViews as Map<string, BrowserView>;
    const storedBounds = (mainWindow as any).browserViewBounds as Map<string, any> || new Map();
    
    for (const [viewId, view] of browserViews.entries()) {
      try {
        // Restore the original bounds directly
        const originalBounds = storedBounds.get(viewId);
        if (originalBounds && originalBounds.width > 100 && originalBounds.height > 100) {
          view.setBounds(originalBounds);
          console.log(`[Main] Restored BrowserView: ${viewId} to original bounds:`, originalBounds);
        } else {
          console.warn(`[Main] No valid stored bounds for BrowserView: ${viewId}, keeping hidden`);
        }
        
      } catch (error) {
        console.error(`[Main] Error showing BrowserView ${viewId}:`, error);
      }
    }
    return true;
  } catch (error) {
    console.error('Error showing browser views:', error);
    return false;
  }
});

// ========================================
// Child WebViewer Window IPC Handlers
// ========================================

// Track child webviewer windows per main window
const childWebViewerWindows = new Map<number, Map<string, BrowserWindow>>(); // mainWindowId -> Map<viewerId, childWindow>

// Handle creating child webviewer window
ipcMain.handle('create-child-webviewer', async (event, url: string, bounds: { x: number; y: number; width: number; height: number }, viewerId?: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) {
      console.log('[Main] No main window found for child webviewer creation');
      return { success: false, error: 'No main window found' };
    }

    const mainWindowId = mainWindow.id;
    const actualViewerId = viewerId || `webviewer-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

    // Initialize child windows map for this main window if needed
    if (!childWebViewerWindows.has(mainWindowId)) {
      childWebViewerWindows.set(mainWindowId, new Map());
    }

    const childWindows = childWebViewerWindows.get(mainWindowId)!;

    // Check if child window already exists for this viewer ID
    if (childWindows.has(actualViewerId)) {
      const existingWindow = childWindows.get(actualViewerId)!;
      if (!existingWindow.isDestroyed()) {
        console.log('[Main] Child webviewer window already exists:', actualViewerId);
        return { success: true, viewerId: actualViewerId, existing: true };
      } else {
        // Clean up destroyed window reference
        childWindows.delete(actualViewerId);
      }
    }

    console.log('[Main] Creating new child webviewer window:', actualViewerId);

    const mainBounds = mainWindow.getBounds();
    const contentBounds = mainWindow.getContentBounds();
    
    // Convert relative bounds to absolute screen coordinates
    // The bounds from React are relative to the content area, not the window frame
    const absoluteBounds = {
      x: contentBounds.x + bounds.x,
      y: contentBounds.y + bounds.y,
      width: Math.max(bounds.width, 300),
      height: Math.max(bounds.height, 200)
    };

    // Constrain child window within main window content bounds
    const constrainedBounds = {
      x: Math.max(contentBounds.x, Math.min(absoluteBounds.x, contentBounds.x + contentBounds.width - 300)),
      y: Math.max(contentBounds.y, Math.min(absoluteBounds.y, contentBounds.y + contentBounds.height - 200)),
      width: Math.min(absoluteBounds.width, contentBounds.width - (absoluteBounds.x - contentBounds.x)),
      height: Math.min(absoluteBounds.height, contentBounds.height - (absoluteBounds.y - contentBounds.y))
    };

    // Create child webviewer window
    const childWindow = new BrowserWindow({
      parent: mainWindow,
      width: constrainedBounds.width,
      height: constrainedBounds.height,
      x: constrainedBounds.x,
      y: constrainedBounds.y,
      frame: false,
      transparent: true,
      alwaysOnTop: false,
      skipTaskbar: true,
      resizable: false, // Prevent user from resizing
      minimizable: false,
      maximizable: false,
      closable: false, // Prevent user from closing manually
      show: false,
      hasShadow: false, // Explicitly disable drop shadow
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        webSecurity: true,
        allowRunningInsecureContent: false,
        experimentalFeatures: true,
        partition: 'persist:webviewer', // Separate partition for webviewer
      },
    });

    // Load the URL
    await childWindow.loadURL(url);

    // Store reference to child window
    childWindows.set(actualViewerId, childWindow);

    // Store the initial relative position (fixed at creation time)
    const initialRelativeX = constrainedBounds.x - mainBounds.x;
    const initialRelativeY = constrainedBounds.y - mainBounds.y;

    // Handle child window position updates when main window moves
    const updateChildPosition = () => {
      if (childWindow && !childWindow.isDestroyed()) {
        const newMainBounds = mainWindow.getBounds();
        
        // Use the fixed initial relative position instead of calculating it dynamically
        childWindow.setPosition(
          newMainBounds.x + initialRelativeX,
          newMainBounds.y + initialRelativeY
        );
      }
    };

    mainWindow.on('move', updateChildPosition);
    mainWindow.on('resize', updateChildPosition);

    // Clean up when child window is destroyed
    childWindow.on('closed', () => {
      childWindows.delete(actualViewerId);
      mainWindow.removeListener('move', updateChildPosition);
      mainWindow.removeListener('resize', updateChildPosition);
      console.log('[Main] Child webviewer window closed:', actualViewerId);
    });

    // Handle navigation events
    childWindow.webContents.on('did-start-loading', () => {
      mainWindow.webContents.send('child-webviewer-loading', actualViewerId, true);
    });

    childWindow.webContents.on('did-finish-load', () => {
      mainWindow.webContents.send('child-webviewer-loading', actualViewerId, false);
      mainWindow.webContents.send('child-webviewer-navigation', actualViewerId, {
        url: childWindow.webContents.getURL(),
        title: childWindow.webContents.getTitle(),
        canGoBack: childWindow.webContents.canGoBack(),
        canGoForward: childWindow.webContents.canGoForward()
      });
    });

    childWindow.webContents.on('did-fail-load', (event, errorCode, errorDescription) => {
      mainWindow.webContents.send('child-webviewer-error', actualViewerId, errorDescription);
    });

    childWindow.webContents.on('page-title-updated', (event, title) => {
      mainWindow.webContents.send('child-webviewer-title', actualViewerId, title);
    });

    // Handle new window creation for links
    childWindow.webContents.setWindowOpenHandler(({ url }) => {
      // Open all links in external browser
      if (url.startsWith('http:') || url.startsWith('https:')) {
        shell.openExternal(url);
        return { action: 'deny' };
      }
      return { action: 'allow' };
    });

    console.log('[Main] Child webviewer window created successfully:', actualViewerId);
    return { success: true, viewerId: actualViewerId };

  } catch (error) {
    console.error('[Main] Error creating child webviewer window:', error);
    return { success: false, error: error.message };
  }
});

// Handle showing child webviewer window
ipcMain.handle('show-child-webviewer', async (event, viewerId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) return false;

    const childWindows = childWebViewerWindows.get(mainWindow.id);
    if (!childWindows) return false;

    const childWindow = childWindows.get(viewerId);
    if (!childWindow || childWindow.isDestroyed()) return false;

    childWindow.show();
    console.log('[Main] Child webviewer window shown:', viewerId);
    return true;
  } catch (error) {
    console.error('[Main] Error showing child webviewer window:', error);
    return false;
  }
});

// Handle hiding child webviewer window
ipcMain.handle('hide-child-webviewer', async (event, viewerId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) return false;

    const childWindows = childWebViewerWindows.get(mainWindow.id);
    if (!childWindows) return false;

    const childWindow = childWindows.get(viewerId);
    if (!childWindow || childWindow.isDestroyed()) return false;

    childWindow.hide();
    console.log('[Main] Child webviewer window hidden:', viewerId);
    return true;
  } catch (error) {
    console.error('[Main] Error hiding child webviewer window:', error);
    return false;
  }
});

// Handle updating child webviewer window bounds
ipcMain.handle('update-child-webviewer-bounds', async (event, viewerId: string, bounds: { x: number; y: number; width: number; height: number }) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) return false;

    const childWindows = childWebViewerWindows.get(mainWindow.id);
    if (!childWindows) return false;

    const childWindow = childWindows.get(viewerId);
    if (!childWindow || childWindow.isDestroyed()) return false;

    const mainBounds = mainWindow.getBounds();
    
    // The bounds from React are already relative to the main window's content area
    // We need to convert them to absolute screen coordinates by adding main window position
    // BUT we also need to account for the window frame (title bar, etc.)
    const contentBounds = mainWindow.getContentBounds();
    const frameOffset = {
      x: contentBounds.x - mainBounds.x,
      y: contentBounds.y - mainBounds.y
    };
    
    // Convert relative bounds to absolute screen coordinates
    const absoluteBounds = {
      x: contentBounds.x + bounds.x,
      y: contentBounds.y + bounds.y,
      width: Math.max(bounds.width, 300),
      height: Math.max(bounds.height, 200)
    };

    // Constrain child window within main window content bounds
    const constrainedBounds = {
      x: Math.max(contentBounds.x, Math.min(absoluteBounds.x, contentBounds.x + contentBounds.width - 300)),
      y: Math.max(contentBounds.y, Math.min(absoluteBounds.y, contentBounds.y + contentBounds.height - 200)),
      width: Math.min(absoluteBounds.width, contentBounds.width - (absoluteBounds.x - contentBounds.x)),
      height: Math.min(absoluteBounds.height, contentBounds.height - (absoluteBounds.y - contentBounds.y))
    };

    childWindow.setBounds(constrainedBounds);
    console.log('[Main] Child webviewer window bounds updated:', viewerId, constrainedBounds);
    return true;
  } catch (error) {
    console.error('[Main] Error updating child webviewer window bounds:', error);
    return false;
  }
});

// Handle child webviewer navigation
ipcMain.handle('child-webviewer-navigate', async (event, viewerId: string, url: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) return false;

    const childWindows = childWebViewerWindows.get(mainWindow.id);
    if (!childWindows) return false;

    const childWindow = childWindows.get(viewerId);
    if (!childWindow || childWindow.isDestroyed()) return false;

    await childWindow.loadURL(url);
    console.log('[Main] Child webviewer navigated to:', url);
    return true;
  } catch (error) {
    console.error('[Main] Error navigating child webviewer:', error);
    return false;
  }
});

// Handle child webviewer back navigation
ipcMain.handle('child-webviewer-go-back', async (event, viewerId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) return false;

    const childWindows = childWebViewerWindows.get(mainWindow.id);
    if (!childWindows) return false;

    const childWindow = childWindows.get(viewerId);
    if (!childWindow || childWindow.isDestroyed()) return false;

    if (childWindow.webContents.canGoBack()) {
      childWindow.webContents.goBack();
      return true;
    }
    return false;
  } catch (error) {
    console.error('[Main] Error going back in child webviewer:', error);
    return false;
  }
});

// Handle child webviewer forward navigation
ipcMain.handle('child-webviewer-go-forward', async (event, viewerId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) return false;

    const childWindows = childWebViewerWindows.get(mainWindow.id);
    if (!childWindows) return false;

    const childWindow = childWindows.get(viewerId);
    if (!childWindow || childWindow.isDestroyed()) return false;

    if (childWindow.webContents.canGoForward()) {
      childWindow.webContents.goForward();
      return true;
    }
    return false;
  } catch (error) {
    console.error('[Main] Error going forward in child webviewer:', error);
    return false;
  }
});

// Handle child webviewer refresh
ipcMain.handle('child-webviewer-refresh', async (event, viewerId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) return false;

    const childWindows = childWebViewerWindows.get(mainWindow.id);
    if (!childWindows) return false;

    const childWindow = childWindows.get(viewerId);
    if (!childWindow || childWindow.isDestroyed()) return false;

    childWindow.webContents.reload();
    return true;
  } catch (error) {
    console.error('[Main] Error refreshing child webviewer:', error);
    return false;
  }
});

// Handle getting child webviewer navigation state
ipcMain.handle('child-webviewer-navigation-state', async (event, viewerId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) return null;

    const childWindows = childWebViewerWindows.get(mainWindow.id);
    if (!childWindows) return null;

    const childWindow = childWindows.get(viewerId);
    if (!childWindow || childWindow.isDestroyed()) return null;

    return {
      canGoBack: childWindow.webContents.canGoBack(),
      canGoForward: childWindow.webContents.canGoForward(),
      isLoading: childWindow.webContents.isLoading(),
      url: childWindow.webContents.getURL(),
      title: childWindow.webContents.getTitle()
    };
  } catch (error) {
    console.error('[Main] Error getting child webviewer navigation state:', error);
    return null;
  }
});

// Handle destroying child webviewer window
ipcMain.handle('destroy-child-webviewer', async (event, viewerId: string) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) return false;

    const childWindows = childWebViewerWindows.get(mainWindow.id);
    if (!childWindows) return false;

    const childWindow = childWindows.get(viewerId);
    if (!childWindow) return false;

    if (!childWindow.isDestroyed()) {
      childWindow.destroy();
    }
    
    childWindows.delete(viewerId);
    console.log('[Main] Child webviewer window destroyed:', viewerId);
    return true;
  } catch (error) {
    console.error('[Main] Error destroying child webviewer window:', error);
    return false;
  }
});

// Add cleanup for child webviewer windows to the existing window closed handler
// This will be added to the createChat function's window.on('closed') handler

// ========================================
// Dock Window IPC Handlers
// ========================================

// Handle creating dock child window
ipcMain.handle('create-dock-window', async (event) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) {
      console.log('[Main] No main window found for dock creation');
      return { success: false, error: 'No main window found' };
    }

    // Check if dock window already exists and is not destroyed
    if ((mainWindow as any).dockWindow && !(mainWindow as any).dockWindow.isDestroyed()) {
      console.log('[Main] Dock window already exists and is valid, returning existing window');
      return { success: true, windowId: (mainWindow as any).dockWindow.id, existing: true };
    }

    // Clean up any destroyed dock window reference
    if ((mainWindow as any).dockWindow && (mainWindow as any).dockWindow.isDestroyed()) {
      console.log('[Main] Cleaning up destroyed dock window reference');
      (mainWindow as any).dockWindow = null;
    }

    console.log('[Main] Creating new dock window...');

    const mainBounds = mainWindow.getBounds();
    
    // Create dock window HTML content
    const dockHtml = `
      <!DOCTYPE html>
      <html>
      <head>
        <meta charset="UTF-8">
        <style>
          body {
            margin: 0;
            padding: 0;
            background: transparent;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            overflow: hidden;
          }
          
          .dock-container {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 20px;
            background: rgba(255, 255, 255, 0.1);
            backdrop-filter: blur(20px);
            border: 1px solid rgba(255, 255, 255, 0.2);
            border-radius: 16px;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
          }
          
          .dock-item {
            width: 48px;
            height: 48px;
            margin: 8px 0;
            border-radius: 12px;
            display: flex;
            align-items: center;
            justify-content: center;
            cursor: pointer;
            transition: all 0.2s ease;
            font-size: 24px;
            border: 1px solid rgba(255, 255, 255, 0.2);
            position: relative;
          }
          
          .dock-item:hover {
            transform: scale(1.1) translateY(-2px);
            box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
          }
          
          .dock-item.web-viewer {
            background: linear-gradient(135deg, #3b82f6, #1d4ed8);
          }
          
          .dock-item.file-viewer {
            background: linear-gradient(135deg, #3b82f6, #1e40af);
          }
          
          .dock-item.app-installer {
            background: linear-gradient(135deg, #3b82f6, #2563eb);
          }
          
          .dock-item.document-editor {
            background: linear-gradient(135deg, #9ca3af, #4b5563);
          }
          
          .dock-item.localhost {
            background: linear-gradient(135deg, #1f2937, #000000);
          }
          
          .dock-item::after {
            content: '';
            position: absolute;
            bottom: -4px;
            left: 50%;
            transform: translateX(-50%);
            width: 4px;
            height: 4px;
            background: rgba(255, 255, 255, 0.6);
            border-radius: 50%;
            opacity: 0;
            transition: opacity 0.2s ease;
          }
          
          .dock-item:hover::after {
            opacity: 1;
          }
        </style>
      </head>
      <body>
        <div class="dock-container">
          <div class="dock-item web-viewer" onclick="window.electronAPI?.dockAddContainer('web-viewer')" title="Web Viewer">🌐</div>
          <div class="dock-item file-viewer" onclick="window.electronAPI?.dockAddContainer('file')" title="File Viewer">📁</div>
          <div class="dock-item app-installer" onclick="window.electronAPI?.dockAddContainer('app-installer')" title="App Installer">📦</div>
          <div class="dock-item document-editor" onclick="window.electronAPI?.dockAddContainer('document-editor')" title="Document Editor">📝</div>
          <div class="dock-item localhost" onclick="window.electronAPI?.dockAddContainer('localhost')" title="Localhost">⚡</div>
        </div>
        
        <script>
          const { ipcRenderer } = require('electron');
          
          // Set up the dock API
          window.electronAPI = {
            dockAddContainer: (containerType, filePath) => {
              console.log('Dock: Adding container', containerType, filePath);
              // Send message to main process
              ipcRenderer.invoke('dock-add-container', containerType, filePath);
            }
          };
        </script>
      </body>
      </html>
    `;

    // Create the dock child window with very high z-index
    const dockWindow = new BrowserWindow({
      parent: mainWindow,
      width: 80,
      height: 320,
      x: mainBounds.x + 20,
      y: mainBounds.y + Math.floor(mainBounds.height / 2) - 160,
      frame: false,
      transparent: true,
      alwaysOnTop: true,
      skipTaskbar: true,
      resizable: false,
      minimizable: false,
      maximizable: false,
      closable: false,
      show: false,
      webPreferences: {
        nodeIntegration: true,
        contextIsolation: false,
        webSecurity: true,
        zoomFactor: 1.0,
      },
    });

    // Set the dock window to have the highest possible z-index
    dockWindow.setAlwaysOnTop(true, 'screen-saver', 1000);
    dockWindow.setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true });
    dockWindow.setFullScreenable(false);

    // Load the dock HTML
    await dockWindow.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(dockHtml)}`);

    // Store reference to dock window
    (mainWindow as any).dockWindow = dockWindow;

    // Handle dock window position updates when main window moves
    const updateDockPosition = () => {
      if (dockWindow && !dockWindow.isDestroyed()) {
        const newBounds = mainWindow.getBounds();
        dockWindow.setPosition(
          newBounds.x + 20,
          newBounds.y + Math.floor(newBounds.height / 2) - 160
        );
      }
    };

    mainWindow.on('move', updateDockPosition);
    mainWindow.on('resize', updateDockPosition);

    // Clean up when dock window is destroyed
    dockWindow.on('closed', () => {
      (mainWindow as any).dockWindow = null;
      mainWindow.removeListener('move', updateDockPosition);
      mainWindow.removeListener('resize', updateDockPosition);
    });

    console.log('[Main] Dock window created successfully');
    return { success: true, windowId: dockWindow.id };

  } catch (error) {
    console.error('[Main] Error creating dock window:', error);
    return { success: false, error: error.message };
  }
});

// Handle showing dock window
ipcMain.handle('show-dock-window', async (event) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).dockWindow) {
      return false;
    }

    const dockWindow = (mainWindow as any).dockWindow;
    if (!dockWindow.isDestroyed()) {
      // Re-enforce the highest z-index when showing
      dockWindow.setAlwaysOnTop(true, 'screen-saver', 1000);
      dockWindow.show();
      dockWindow.focus();
      console.log('[Main] Dock window shown with highest z-index');
      return true;
    }
    return false;
  } catch (error) {
    console.error('[Main] Error showing dock window:', error);
    return false;
  }
});

// Handle hiding dock window
ipcMain.handle('hide-dock-window', async (event) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow || !(mainWindow as any).dockWindow) {
      return false;
    }

    const dockWindow = (mainWindow as any).dockWindow;
    if (!dockWindow.isDestroyed()) {
      dockWindow.hide();
      console.log('[Main] Dock window hidden');
      return true;
    }
    return false;
  } catch (error) {
    console.error('[Main] Error hiding dock window:', error);
    return false;
  }
});

// Handle dock container addition
ipcMain.handle('dock-add-container', async (event, containerType, filePath) => {
  try {
    const mainWindow = BrowserWindow.fromWebContents(event.sender);
    if (!mainWindow) {
      return false;
    }

    console.log('[Main] Dock add container:', containerType, filePath);
    
    // Send message to main window to add the container
    mainWindow.webContents.send('add-container-from-dock', containerType, filePath);
    
    return true;
  } catch (error) {
    console.error('[Main] Error handling dock add container:', error);
    return false;
  }
});

// Handle directory chooser
ipcMain.handle('directory-chooser', (_event) => {
  return openDirectoryDialog();
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

ipcMain.handle('get-goosed-host-port', async (event) => {
  const windowId = BrowserWindow.fromWebContents(event.sender)?.id;
  if (!windowId) {
    return null;
  }
  const client = goosedClients.get(windowId);
  if (!client) {
    return null;
  }
  await checkServerStatus(client);
  return client.getConfig().baseUrl || null;
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
      app.dock?.show();
    } else {
      // Only hide the dock if we have a menu bar icon to maintain accessibility
      if (settings.showMenuBarIcon) {
        app.dock?.hide();
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
      } catch {
        console.log('GNOME control center not found, trying other options');
      }

      // KDE Plasma
      try {
        spawn('systemsettings5', ['kcm_notifications']);
        return true;
      } catch {
        console.log('KDE systemsettings5 not found, trying other options');
      }

      // XFCE
      try {
        spawn('xfce4-settings-manager', ['--socket-id=notifications']);
        return true;
      } catch {
        console.log('XFCE settings manager not found, trying other options');
      }

      // Fallback: Try to open general settings
      try {
        spawn('gnome-control-center');
        return true;
      } catch {
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
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
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

ipcMain.handle('read-file', async (_event, filePath) => {
  try {
    const expandedPath = expandTilde(filePath);
    if (process.platform === 'win32') {
      const buffer = await fs.readFile(expandedPath);
      return { file: buffer.toString('utf8'), filePath: expandedPath, error: null, found: true };
    }
    // Non-Windows: keep previous behavior via cat for parity
    return await new Promise((resolve) => {
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
  } catch (error) {
    console.error('Error reading file:', error);
    return { file: '', filePath: expandTilde(filePath), error, found: false };
  }
});

ipcMain.handle('write-file', async (_event, filePath, content) => {
  try {
    // Expand tilde to home directory
    const expandedPath = expandTilde(filePath);
    await fs.writeFile(expandedPath, content, { encoding: 'utf8' });
    return true;
  } catch (error) {
    console.error('Error writing to file:', error);
    return false;
  }
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

async function appMain() {
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
          // Connect to our local API, localhost apps, and specific external services
          "connect-src 'self' http://127.0.0.1:* http://localhost:* ws://localhost:* wss://localhost:* https://api.github.com https://github.com https://objects.githubusercontent.com" +
          // Don't allow any plugins
          "object-src 'none';" +
          // Allow all frames (iframes) including localhost
          "frame-src 'self' https: http: http://localhost:* ws://localhost:*;" +
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
          // Child sources for web workers and frames
          "child-src 'self' http://localhost:*;" +
          // Upgrade insecure requests (but allow localhost HTTP)
          '',
      },
    });
  });

  // Register the default global hotkey
  registerGlobalHotkey('CommandOrControl+Alt+Shift+G');

  session.defaultSession.webRequest.onBeforeSendHeaders((details, callback) => {
    details.requestHeaders['Origin'] = 'http://localhost:5173';
    callback({ cancel: false, requestHeaders: details.requestHeaders });
  });

  // Create tray if enabled in settings
  const settings = loadSettings();
  if (settings.showMenuBarIcon) {
    createTray();
  }

  // Handle dock icon visibility (macOS only)
  if (process.platform === 'darwin' && !settings.showDockIcon && settings.showMenuBarIcon) {
    app.dock?.hide();
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
            label: `Version ${version || app.getVersion()}`,
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

  // Handle spell checking requests using system spell checker
  ipcMain.handle('spell-check', async (event, word: string) => {
    try {
      console.log('[Main] System spell check request for word:', word);
      
      if (!word || typeof word !== 'string') {
        return true; // Assume correct for invalid input
      }

      // Skip very short words (less than 3 characters)
      if (word.length < 3) {
        return true;
      }

      const cleanWord = word.trim();
      
      try {
        // Use system spell checker based on platform
        if (process.platform === 'darwin') {
          // macOS: Use aspell
          const { spawn } = require('child_process');
          
          return new Promise((resolve) => {
            const aspellProcess = spawn('aspell', ['-a'], { 
              stdio: ['pipe', 'pipe', 'pipe'],
              timeout: 3000 
            });
            
            let output = '';
            
            aspellProcess.stdout.on('data', (data) => {
              output += data.toString();
            });

            aspellProcess.on('close', (code) => {
              // Parse aspell output
              const lines = output.split('\n').filter(line => line.trim());
              let isCorrect = true;
              
              for (const line of lines) {
                if (line.startsWith('*')) {
                  // Word is correct
                  isCorrect = true;
                  break;
                } else if (line.startsWith('&') || line.startsWith('#')) {
                  // Word is misspelled
                  isCorrect = false;
                  break;
                }
              }
              
              console.log('[Main] macOS aspell spell check result for', word, ':', isCorrect);
              resolve(isCorrect);
            });

            aspellProcess.on('error', (error) => {
              console.error('[Main] aspell error:', error);
              resolve(true); // Default to correct if aspell not available
            });

            aspellProcess.stdin.write(cleanWord + '\n');
            aspellProcess.stdin.end();

            setTimeout(() => {
              aspellProcess.kill();
              resolve(true);
            }, 3000);
          });
          
        } else if (process.platform === 'win32') {
          // Windows: Try to use hunspell or fall back to basic check
          return new Promise((resolve) => {
            const { spawn } = require('child_process');
            
            // Try hunspell first (if available)
            const hunspellProcess = spawn('hunspell', ['-d', 'en_US'], { 
              stdio: ['pipe', 'pipe', 'pipe'],
              timeout: 3000 
            });
            
            let output = '';
            
            hunspellProcess.stdout.on('data', (data) => {
              output += data.toString();
            });

            hunspellProcess.on('close', (code) => {
              // hunspell returns "*" for correct words, "&" for incorrect
              const isCorrect = output.includes('*') || output.trim() === '';
              console.log('[Main] Windows spell check result for', word, ':', isCorrect);
              resolve(isCorrect);
            });

            hunspellProcess.on('error', (error) => {
              console.error('[Main] hunspell not available, defaulting to correct:', error);
              resolve(true); // Default to correct if hunspell not available
            });

            hunspellProcess.stdin.write(cleanWord + '\n');
            hunspellProcess.stdin.end();

            setTimeout(() => {
              hunspellProcess.kill();
              resolve(true);
            }, 3000);
          });
          
        } else {
          // Linux: Use aspell or hunspell
          return new Promise((resolve) => {
            const { spawn } = require('child_process');
            
            const aspellProcess = spawn('aspell', ['-a'], { 
              stdio: ['pipe', 'pipe', 'pipe'],
              timeout: 3000 
            });
            
            let output = '';
            
            aspellProcess.stdout.on('data', (data) => {
              output += data.toString();
            });

            aspellProcess.on('close', (code) => {
              // Parse aspell output
              const lines = output.split('\n').filter(line => line.trim());
              let isCorrect = true;
              
              for (const line of lines) {
                if (line.startsWith('*')) {
                  isCorrect = true;
                  break;
                } else if (line.startsWith('&') || line.startsWith('#')) {
                  isCorrect = false;
                  break;
                }
              }
              
              console.log('[Main] Linux spell check result for', word, ':', isCorrect);
              resolve(isCorrect);
            });

            aspellProcess.on('error', (error) => {
              console.error('[Main] aspell error:', error);
              resolve(true); // Default to correct if aspell not available
            });

            aspellProcess.stdin.write(cleanWord + '\n');
            aspellProcess.stdin.end();

            setTimeout(() => {
              aspellProcess.kill();
              resolve(true);
            }, 3000);
          });
        }
        
      } catch (error) {
        console.error('[Main] Error using system spell checker:', error);
        return true; // Default to correct on error
      }
      
    } catch (error) {
      console.error('Error in system spell-check handler:', error);
      return true; // Assume correct on error
    }
  });

  ipcMain.handle('spell-suggestions', async (event, word: string) => {
    try {
      console.log('[Main] System spell suggestions request for word:', word);
      
      if (!word || typeof word !== 'string') {
        return [];
      }

      // Skip very short words
      if (word.length < 3) {
        return [];
      }

      const cleanWord = word.trim();

      try {
        // Get suggestions using system spell checker based on platform
        if (process.platform === 'darwin' || process.platform === 'linux') {
          // macOS and Linux: Use aspell for suggestions
          const { spawn } = require('child_process');
          
          return new Promise((resolve) => {
            const aspellProcess = spawn('aspell', ['-a'], { 
              stdio: ['pipe', 'pipe', 'pipe'],
              timeout: 3000 
            });
            
            let output = '';
            
            aspellProcess.stdout.on('data', (data) => {
              output += data.toString();
            });

            aspellProcess.on('close', (code) => {
              // Parse aspell output for suggestions
              const lines = output.split('\n').filter(line => line.trim());
              let suggestions: string[] = [];
              
              for (const line of lines) {
                if (line.startsWith('&')) {
                  // Line format: & word count offset: suggestion1, suggestion2, ...
                  const parts = line.split(':');
                  if (parts.length > 1) {
                    const suggestionsPart = parts[1].trim();
                    suggestions = suggestionsPart.split(',').map(s => s.trim()).slice(0, 5); // Limit to 5 suggestions
                  }
                  break;
                } else if (line.startsWith('#')) {
                  // No suggestions available
                  suggestions = [];
                  break;
                }
              }
              
              console.log('[Main] aspell spell suggestions for', word, ':', suggestions);
              resolve(suggestions);
            });

            aspellProcess.on('error', (error) => {
              console.error('[Main] aspell error getting suggestions:', error);
              resolve([]); // Return empty array on error
            });

            aspellProcess.stdin.write(cleanWord + '\n');
            aspellProcess.stdin.end();

            setTimeout(() => {
              aspellProcess.kill();
              resolve([]);
            }, 3000);
          });
          
        } else if (process.platform === 'win32') {
          // Windows: Try to use hunspell for suggestions
          return new Promise((resolve) => {
            const { spawn } = require('child_process');
            
            const hunspellProcess = spawn('hunspell', ['-d', 'en_US', '-s'], { 
              stdio: ['pipe', 'pipe', 'pipe'],
              timeout: 3000 
            });
            
            let output = '';
            
            hunspellProcess.stdout.on('data', (data) => {
              output += data.toString();
            });

            hunspellProcess.on('close', (code) => {
              // Parse hunspell suggestions
              const lines = output.split('\n').filter(line => line.trim());
              const suggestions = lines.slice(0, 5); // Limit to 5 suggestions
              
              console.log('[Main] hunspell spell suggestions for', word, ':', suggestions);
              resolve(suggestions);
            });

            hunspellProcess.on('error', (error) => {
              console.error('[Main] hunspell not available for suggestions:', error);
              resolve([]); // Return empty array if hunspell not available
            });

            hunspellProcess.stdin.write(cleanWord + '\n');
            hunspellProcess.stdin.end();

            setTimeout(() => {
              hunspellProcess.kill();
              resolve([]);
            }, 3000);
          });
        }
        
        return [];
        
      } catch (error) {
        console.error('[Main] Error getting spell suggestions:', error);
        return [];
      }
      
    } catch (error) {
      console.error('Error in system spell-suggestions handler:', error);
      return [];
    }
  });

  // ========================================
  // App Installer IPC Handlers
  // ========================================

  // Handle repository cloning
  ipcMain.handle('clone-repository', async (_event, gitUrl: string, appId: string) => {
    try {
      console.log('[Main] Cloning repository:', gitUrl, 'with ID:', appId);
      
      // Create apps directory in user data
      const appsDir = path.join(app.getPath('userData'), 'installed-apps');
      await fs.mkdir(appsDir, { recursive: true });
      
      const localPath = path.join(appsDir, appId);
      
      // Check if directory already exists
      try {
        await fs.access(localPath);
        return { success: false, error: 'App directory already exists' };
      } catch {
        // Directory doesn't exist, which is good
      }
      
      // Clone the repository using git
      return new Promise((resolve) => {
        const gitProcess = spawn('git', ['clone', gitUrl, localPath], {
          stdio: ['pipe', 'pipe', 'pipe']
        });
        
        let errorOutput = '';
        
        gitProcess.stderr.on('data', (data) => {
          errorOutput += data.toString();
        });
        
        gitProcess.on('close', (code) => {
          if (code === 0) {
            console.log('[Main] Repository cloned successfully to:', localPath);
            resolve({ success: true, localPath });
          } else {
            console.error('[Main] Git clone failed:', errorOutput);
            resolve({ success: false, error: errorOutput || 'Git clone failed' });
          }
        });
        
        gitProcess.on('error', (error) => {
          console.error('[Main] Git process error:', error);
          resolve({ success: false, error: error.message });
        });
      });
      
    } catch (error) {
      console.error('[Main] Error cloning repository:', error);
      return { success: false, error: error.message };
    }
  });

  // Helper function to find package manager executable
  const findPackageManagerPath = async (packageManager: string): Promise<string> => {
    const commonPaths = [
      `/opt/homebrew/bin/${packageManager}`,  // Homebrew on Apple Silicon
      `/usr/local/bin/${packageManager}`,     // Homebrew on Intel Mac
      `/usr/bin/${packageManager}`,           // System install
      packageManager                          // Fallback to PATH
    ];
    
    for (const pmPath of commonPaths) {
      try {
        const result = spawn('which', [pmPath], { stdio: 'pipe' });
        const output = await new Promise<string>((resolve, reject) => {
          let stdout = '';
          result.stdout?.on('data', (data) => stdout += data.toString());
          result.on('close', (code) => {
            if (code === 0 && stdout.trim()) {
              resolve(stdout.trim());
            } else {
              reject(new Error(`${pmPath} not found`));
            }
          });
          result.on('error', reject);
        });
        
        if (output) {
          console.log(`[Main] Found ${packageManager} at:`, output);
          return output;
        }
      } catch (error) {
        // Try next path
        continue;
      }
    }
    
    // Fallback to package manager name (will use PATH)
    console.warn(`[Main] Could not find full path for ${packageManager}, using PATH lookup`);
    return packageManager;
  };

  // Handle project analysis
  ipcMain.handle('analyze-project', async (_event, projectPath: string) => {
    try {
      console.log('[Main] Analyzing project at:', projectPath);
      
      const analysis = {
        name: path.basename(projectPath),
        description: '',
        projectType: 'unknown' as 'web' | 'electron' | 'cli' | 'library' | 'unknown',
        buildCommand: undefined as string | undefined,
        startCommand: undefined as string | undefined,
        port: undefined as number | undefined,
        requiresInstall: false,
        packageManager: 'npm' as string,
        packageManagerPath: undefined as string | undefined
      };
      
      // Check for package.json (Node.js project)
      const packageJsonPath = path.join(projectPath, 'package.json');
      try {
        const packageJsonContent = await fs.readFile(packageJsonPath, 'utf-8');
        const packageJson = JSON.parse(packageJsonContent);
        
        analysis.name = packageJson.name || analysis.name;
        analysis.description = packageJson.description || '';
        analysis.requiresInstall = true;
        
        // Detect package manager - prioritize package-lock.json over yarn.lock
        // since package-lock.json is often the primary lock file
        try {
          await fs.access(path.join(projectPath, 'package-lock.json'));
          analysis.packageManager = 'npm';
        } catch {
          try {
            await fs.access(path.join(projectPath, 'pnpm-lock.yaml'));
            analysis.packageManager = 'pnpm';
          } catch {
            try {
              await fs.access(path.join(projectPath, 'yarn.lock'));
              analysis.packageManager = 'yarn';
            } catch {
              analysis.packageManager = 'npm'; // Default fallback
            }
          }
        }
        
        // Find the full path to the package manager
        analysis.packageManagerPath = await findPackageManagerPath(analysis.packageManager);
        
        // Analyze scripts and dependencies
        const scripts = packageJson.scripts || {};
        const dependencies = { ...packageJson.dependencies, ...packageJson.devDependencies };
        
        // Detect project type and set appropriate commands
        if (dependencies['electron']) {
          analysis.projectType = 'electron';
          analysis.startCommand = scripts.start || scripts.electron || `${analysis.packageManagerPath} start`;
        } else if (dependencies['react'] || dependencies['vue'] || dependencies['angular'] || dependencies['svelte'] || dependencies['next'] || dependencies['nuxt'] || dependencies['vite'] || dependencies['webpack']) {
          analysis.projectType = 'web';
          analysis.startCommand = scripts.start || scripts.dev || scripts.serve || `${analysis.packageManagerPath} run dev`;
          analysis.buildCommand = scripts.build;
          
          // Try to detect port from common patterns
          if (scripts.start || scripts.dev) {
            const command = scripts.start || scripts.dev;
            const portMatch = command.match(/--port[= ](\d+)|:(\d+)/);
            if (portMatch) {
              analysis.port = parseInt(portMatch[1] || portMatch[2]);
            }
          }
          
          // Default ports for common frameworks
          if (!analysis.port) {
            if (dependencies['react'] && !dependencies['next']) analysis.port = 3000;
            else if (dependencies['vue'] && !dependencies['nuxt']) analysis.port = 8080;
            else if (dependencies['angular']) analysis.port = 4200;
            else if (dependencies['svelte']) analysis.port = 5000;
            else if (dependencies['next']) analysis.port = 3000;
            else if (dependencies['nuxt']) analysis.port = 3000;
            else if (dependencies['vite']) analysis.port = 5173;
            else analysis.port = 3000; // Default fallback
          }
        } else if (packageJson.bin || scripts.cli) {
          analysis.projectType = 'cli';
          analysis.startCommand = scripts.start || Object.keys(packageJson.bin || {})[0] || 'npm start';
        } else if (scripts.start || scripts.dev || scripts.serve) {
          // Has start scripts but not a recognized web framework
          analysis.projectType = 'web';
          analysis.startCommand = scripts.start || scripts.dev || scripts.serve;
          analysis.port = 3000; // Default port
        } else if (scripts.test || scripts.build) {
          // Looks like a library or tool
          analysis.projectType = 'library';
          analysis.startCommand = scripts.start || 'npm start'; // Fallback
        } else {
          // Generic Node.js project
          analysis.projectType = 'cli';
          analysis.startCommand = 'npm start';
        }
        
      } catch (error) {
        // No package.json or invalid JSON
        console.log('[Main] No valid package.json found, checking other project types');
      }
      
      // Check for Python projects
      if (analysis.projectType === 'unknown') {
        try {
          await fs.access(path.join(projectPath, 'requirements.txt'));
          analysis.projectType = 'web';
          analysis.requiresInstall = true;
          analysis.packageManager = 'pip';
          analysis.startCommand = 'python app.py';
          analysis.port = 5000;
        } catch {
          // Check for setup.py or pyproject.toml
          try {
            await fs.access(path.join(projectPath, 'setup.py'));
            analysis.projectType = 'library';
            analysis.requiresInstall = true;
            analysis.packageManager = 'pip';
          } catch {
            try {
              await fs.access(path.join(projectPath, 'pyproject.toml'));
              analysis.projectType = 'library';
              analysis.requiresInstall = true;
              analysis.packageManager = 'pip';
            } catch {
              // Not a Python project
            }
          }
        }
      }
      
      // Check for Rust projects
      if (analysis.projectType === 'unknown') {
        try {
          await fs.access(path.join(projectPath, 'Cargo.toml'));
          analysis.projectType = 'cli';
          analysis.requiresInstall = true;
          analysis.packageManager = 'cargo';
          analysis.startCommand = 'cargo run';
        } catch {
          // Not a Rust project
        }
      }
      
      // Check for Go projects
      if (analysis.projectType === 'unknown') {
        try {
          await fs.access(path.join(projectPath, 'go.mod'));
          analysis.projectType = 'cli';
          analysis.requiresInstall = false; // Go doesn't require separate install step
          analysis.startCommand = 'go run .';
        } catch {
          // Not a Go project
        }
      }
      
      console.log('[Main] Project analysis complete:', analysis);
      return { success: true, ...analysis };
      
    } catch (error) {
      console.error('[Main] Error analyzing project:', error);
      return { success: false, error: error.message };
    }
  });

  // Handle dependency installation
  ipcMain.handle('install-project-dependencies', async (_event, projectPath: string, packageManager: string, packageManagerPath?: string) => {
    try {
      console.log('[Main] Installing dependencies for project at:', projectPath, 'using:', packageManager);
      
      // Use provided path or find the package manager path
      const pmPath = packageManagerPath || await findPackageManagerPath(packageManager);
      
      let command: string;
      let args: string[];
      
      switch (packageManager) {
        case 'npm':
          command = pmPath;
          args = ['install'];
          break;
        case 'yarn':
          command = pmPath;
          args = ['install'];
          break;
        case 'pnpm':
          command = pmPath;
          args = ['install'];
          break;
        case 'pip':
          command = pmPath;
          args = ['install', '-r', 'requirements.txt'];
          break;
        case 'cargo':
          command = pmPath;
          args = ['build'];
          break;
        default:
          return { success: false, error: `Unsupported package manager: ${packageManager}` };
      }
      
      console.log('[Main] Using package manager command:', command, args.join(' '));
      
      return new Promise((resolve) => {
        const installProcess = spawn(command, args, {
          cwd: projectPath,
          stdio: ['pipe', 'pipe', 'pipe']
        });
        
        let output = '';
        let errorOutput = '';
        
        installProcess.stdout.on('data', (data) => {
          output += data.toString();
        });
        
        installProcess.stderr.on('data', (data) => {
          errorOutput += data.toString();
        });
        
        installProcess.on('close', (code) => {
          if (code === 0) {
            console.log('[Main] Dependencies installed successfully');
            resolve({ success: true });
          } else {
            console.error('[Main] Dependency installation failed:', errorOutput);
            resolve({ success: false, error: errorOutput || 'Installation failed' });
          }
        });
        
        installProcess.on('error', (error) => {
          console.error('[Main] Install process error:', error);
          resolve({ success: false, error: error.message });
        });
      });
      
    } catch (error) {
      console.error('[Main] Error installing dependencies:', error);
      return { success: false, error: error.message };
    }
  });

  // Handle app configuration saving
  ipcMain.handle('save-app-configuration', async (_event, appConfig: any) => {
    try {
      console.log('[Main] Saving app configuration:', appConfig.id);
      
      const configDir = path.join(app.getPath('userData'), 'app-configs');
      await fs.mkdir(configDir, { recursive: true });
      
      const configPath = path.join(configDir, `${appConfig.id}.json`);
      await fs.writeFile(configPath, JSON.stringify(appConfig, null, 2));
      
      console.log('[Main] App configuration saved to:', configPath);
      return { success: true };
      
    } catch (error) {
      console.error('[Main] Error saving app configuration:', error);
      return { success: false, error: error.message };
    }
  });

  // Handle checking for port conflicts
  ipcMain.handle('check-port-conflict', async (_event, port: number) => {
    try {
      const { spawn } = require('child_process');
      const lsof = spawn('lsof', ['-ti', `:${port}`]);
      
      let pids = '';
      lsof.stdout.on('data', (data) => {
        pids += data.toString();
      });
      
      return new Promise((resolve) => {
        lsof.on('close', (code) => {
          if (code === 0 && pids.trim()) {
            const pidList = pids.trim().split('\n').filter(pid => pid.trim());
            resolve({ hasConflict: true, pids: pidList });
          } else {
            resolve({ hasConflict: false, pids: [] });
          }
        });
      });
    } catch (error) {
      console.error('[Main] Error checking port conflict:', error);
      return { hasConflict: false, pids: [] };
    }
  });

  // Handle killing processes on a port
  ipcMain.handle('kill-port-processes', async (_event, port: number) => {
    try {
      const { spawn } = require('child_process');
      const lsof = spawn('lsof', ['-ti', `:${port}`]);
      
      let pids = '';
      lsof.stdout.on('data', (data) => {
        pids += data.toString();
      });
      
      return new Promise((resolve) => {
        lsof.on('close', (code) => {
          if (code === 0 && pids.trim()) {
            const pidList = pids.trim().split('\n').filter(pid => pid.trim());
            let killedCount = 0;
            let errors = [];
            
            pidList.forEach(pid => {
              try {
                process.kill(parseInt(pid), 'SIGTERM');
                console.log(`[Main] Killed process ${pid} on port ${port}`);
                killedCount++;
              } catch (error) {
                console.log(`[Main] Could not kill process ${pid}:`, error.message);
                errors.push(`PID ${pid}: ${error.message}`);
              }
            });
            
            resolve({ 
              success: true, 
              killedCount, 
              totalProcesses: pidList.length,
              errors: errors.length > 0 ? errors : undefined
            });
          } else {
            resolve({ success: true, killedCount: 0, totalProcesses: 0 });
          }
        });
      });
    } catch (error) {
      console.error('[Main] Error killing port processes:', error);
      return { success: false, error: error.message };
    }
  });

  // Handle app launching
  ipcMain.handle('launch-app', async (_event, appConfig: any) => {
    try {
      console.log('[Main] Launching app:', appConfig.name);
      
      if (!appConfig.startCommand) {
        return { success: false, error: 'No start command defined for this app' };
      }
      
      // Check if app is already running
      if (runningApps.has(appConfig.id)) {
        return { success: false, error: 'App is already running. Stop it first before launching again.' };
      }
      
      // Parse the start command
      const commandParts = appConfig.startCommand.split(' ');
      const command = commandParts[0];
      const args = commandParts.slice(1);
      
      // Launch the app process
      const appProcess = spawn(command, args, {
        cwd: appConfig.localPath,
        stdio: ['pipe', 'pipe', 'pipe'],
        detached: true // Allow the process to run independently
      });
      
      // Store the process reference
      runningApps.set(appConfig.id, {
        process: appProcess,
        port: appConfig.port
      });
      
      // Handle process exit
      appProcess.on('exit', (code, signal) => {
        console.log(`[Main] App ${appConfig.name} exited with code ${code}, signal ${signal}`);
        runningApps.delete(appConfig.id);
      });
      
      appProcess.on('error', (error) => {
        console.error('[Main] App launch error:', error);
        runningApps.delete(appConfig.id);
      });
      
      // Don't wait for the process to complete for web apps
      if (appConfig.projectType === 'web') {
        // Give it a moment to start up
        setTimeout(() => {
          console.log('[Main] Web app should be starting on port:', appConfig.port);
        }, 2000);
      }
      
      console.log('[Main] App launched successfully');
      return { success: true };
      
    } catch (error) {
      console.error('[Main] Error launching app:', error);
      return { success: false, error: error.message };
    }
  });

  // Handle app stopping
  ipcMain.handle('stop-app', async (_event, appId: string) => {
    try {
      console.log('[Main] Stopping app:', appId);
      
      const runningApp = runningApps.get(appId);
      if (!runningApp) {
        return { success: false, error: 'App is not currently running' };
      }
      
      const { process: appProcess, port } = runningApp;
      
      // Kill the process
      if (appProcess && !appProcess.killed) {
        appProcess.kill('SIGTERM');
        
        // If it doesn't exit gracefully, force kill after 5 seconds
        setTimeout(() => {
          if (!appProcess.killed) {
            console.log('[Main] Force killing app process');
            appProcess.kill('SIGKILL');
          }
        }, 5000);
      }
      
      // Also kill any processes on the port (in case of orphaned processes)
      if (port) {
        try {
          const { spawn } = require('child_process');
          const killPort = spawn('lsof', ['-ti', `:${port}`]);
          
          let pids = '';
          killPort.stdout.on('data', (data) => {
            pids += data.toString();
          });
          
          killPort.on('close', (code) => {
            if (code === 0 && pids.trim()) {
              const pidList = pids.trim().split('\n');
              pidList.forEach(pid => {
                try {
                  process.kill(parseInt(pid), 'SIGTERM');
                  console.log(`[Main] Killed process ${pid} on port ${port}`);
                } catch (error) {
                  console.log(`[Main] Could not kill process ${pid}:`, error.message);
                }
              });
            }
          });
        } catch (error) {
          console.warn('[Main] Could not kill processes on port:', error);
        }
      }
      
      // Remove from tracking
      runningApps.delete(appId);
      
      console.log('[Main] App stopped successfully');
      return { success: true };
      
    } catch (error) {
      console.error('[Main] Error stopping app:', error);
      return { success: false, error: error.message };
    }
  });

  // Handle checking if app is running
  ipcMain.handle('is-app-running', async (_event, appId: string) => {
    return runningApps.has(appId);
  });

  // Handle app removal
  ipcMain.handle('remove-app', async (_event, appId: string) => {
    try {
      console.log('[Main] Removing app:', appId);
      
      // Remove app directory
      const appsDir = path.join(app.getPath('userData'), 'installed-apps');
      const appPath = path.join(appsDir, appId);
      
      try {
        await fs.rm(appPath, { recursive: true, force: true });
        console.log('[Main] App directory removed:', appPath);
      } catch (error) {
        console.warn('[Main] Could not remove app directory:', error);
      }
      
      // Remove app configuration
      const configDir = path.join(app.getPath('userData'), 'app-configs');
      const configPath = path.join(configDir, `${appId}.json`);
      
      try {
        await fs.unlink(configPath);
        console.log('[Main] App configuration removed:', configPath);
      } catch (error) {
        console.warn('[Main] Could not remove app configuration:', error);
      }
      
      return { success: true };
      
    } catch (error) {
      console.error('[Main] Error removing app:', error);
      return { success: false, error: error.message };
    }
  });

  // Handle showing item in folder
  ipcMain.handle('show-item-in-folder', async (_event, itemPath: string) => {
    try {
      console.log('[Main] Showing item in folder:', itemPath);
      await shell.showItemInFolder(itemPath);
      return { success: true };
    } catch (error) {
      console.error('[Main] Error showing item in folder:', error);
      return { success: false, error: error.message };
    }
  });

  // Handle LLM-powered project analysis for better start commands
  ipcMain.handle('analyze-project-with-llm', async (event, projectPath: string, basicAnalysis: any) => {
    try {
      console.log('[Main] Running LLM-powered project analysis for:', projectPath);
      
      // Get the goosed client for this window
      const windowId = BrowserWindow.fromWebContents(event.sender)?.id;
      if (!windowId) {
        return { success: false, error: 'No window context found' };
      }
      
      const client = goosedClients.get(windowId);
      if (!client) {
        return { success: false, error: 'No goosed client found' };
      }
      
      // Read key project files for context
      const projectFiles = [];
      const filesToCheck = [
        'package.json',
        'README.md',
        'README.rst',
        'requirements.txt',
        'Cargo.toml',
        'go.mod',
        'Makefile',
        'docker-compose.yml',
        'Dockerfile',
        '.env.example',
        'pyproject.toml',
        'setup.py'
      ];
      
      for (const fileName of filesToCheck) {
        try {
          const filePath = path.join(projectPath, fileName);
          const content = await fs.readFile(filePath, 'utf-8');
          projectFiles.push({
            name: fileName,
            content: content.length > 2000 ? content.substring(0, 2000) + '...' : content
          });
        } catch {
          // File doesn't exist, skip
        }
      }
      
      // Get directory structure (first 2 levels)
      let directoryStructure = '';
      try {
        const getDirectoryTree = async (dir: string, level: number = 0, maxLevel: number = 2): Promise<string> => {
          if (level > maxLevel) return '';
          
          const items = await fs.readdir(dir);
          let tree = '';
          
          for (const item of items.slice(0, 20)) { // Limit to first 20 items
            if (item.startsWith('.') && !['package.json', 'README.md'].includes(item)) continue;
            
            const itemPath = path.join(dir, item);
            const stats = await fs.lstat(itemPath);
            const indent = '  '.repeat(level);
            
            if (stats.isDirectory()) {
              tree += `${indent}${item}/\n`;
              if (level < maxLevel) {
                tree += await getDirectoryTree(itemPath, level + 1, maxLevel);
              }
            } else {
              tree += `${indent}${item}\n`;
            }
          }
          return tree;
        };
        
        directoryStructure = await getDirectoryTree(projectPath);
      } catch (error) {
        directoryStructure = 'Could not read directory structure';
      }
      
      // Create a comprehensive prompt for the LLM
      const prompt = `You are a project analysis expert. I need help analyzing a Git repository to determine the best way to run/start this project.

**Project Path:** ${path.basename(projectPath)}

**Basic Analysis Results:**
- Project Type: ${basicAnalysis.projectType || 'unknown'}
- Package Manager: ${basicAnalysis.packageManager || 'unknown'}
- Current Start Command: ${basicAnalysis.startCommand || 'none'}
- Build Command: ${basicAnalysis.buildCommand || 'none'}
- Port: ${basicAnalysis.port || 'unknown'}

**Directory Structure:**
\`\`\`
${directoryStructure}
\`\`\`

**Project Files:**
${projectFiles.map(file => `
**${file.name}:**
\`\`\`
${file.content}
\`\`\`
`).join('\n')}

**Task:** Please analyze this project and provide:

1. **Project Type** (web, cli, library, desktop, mobile, etc.)
2. **Best Start Command** - The exact command to run this project
3. **Setup Steps** - Any required setup before running (if needed)
4. **Port** - Default port if it's a web service
5. **Description** - Brief description of what this project does

**Requirements:**
- Provide practical, working commands
- Consider the most common development workflow
- If it's a web project, prioritize development server commands
- If multiple options exist, choose the most standard one
- Be specific about package managers (npm, yarn, pnpm, pip, cargo, etc.)

**Response Format (JSON):**
\`\`\`json
{
  "projectType": "web|cli|library|desktop|mobile|unknown",
  "startCommand": "exact command to run",
  "setupSteps": ["step 1", "step 2"],
  "port": 3000,
  "description": "Brief project description",
  "confidence": "high|medium|low",
  "reasoning": "Why these commands were chosen"
}
\`\`\``;

      // Send the prompt to the LLM
      try {
        const response = await fetch(`${client.getConfig().baseUrl}/v1/chat/completions`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': process.env.GOOSE_EXTERNAL_BACKEND ? 'test' : SERVER_SECRET,
          },
          body: JSON.stringify({
            messages: [
              {
                role: 'user',
                content: prompt
              }
            ],
            model: 'gpt-4o', // Use a capable model
            temperature: 0.1, // Low temperature for consistent analysis
            max_tokens: 1000
          })
        });

        if (!response.ok) {
          throw new Error(`LLM API error: ${response.status}`);
        }

        const result = await response.json();
        const llmResponse = result.choices?.[0]?.message?.content;

        if (!llmResponse) {
          throw new Error('No response from LLM');
        }

        // Extract JSON from the response
        const jsonMatch = llmResponse.match(/```json\s*([\s\S]*?)\s*```/);
        if (!jsonMatch) {
          throw new Error('Could not parse LLM response as JSON');
        }

        const analysis = JSON.parse(jsonMatch[1]);
        
        console.log('[Main] LLM analysis complete:', analysis);
        return { success: true, analysis };

      } catch (llmError) {
        console.error('[Main] LLM analysis failed:', llmError);
        return { success: false, error: `LLM analysis failed: ${llmError.message}` };
      }

    } catch (error) {
      console.error('[Main] Error in LLM project analysis:', error);
      return { success: false, error: error.message };
    }
  });

  // Handle loading all saved app configurations
  ipcMain.handle('load-saved-apps', async (_event) => {
    try {
      console.log('[Main] Loading saved app configurations');
      
      const configDir = path.join(app.getPath('userData'), 'app-configs');
      
      // Check if config directory exists
      try {
        await fs.access(configDir);
      } catch {
        // Directory doesn't exist, return empty array
        console.log('[Main] No app configs directory found, returning empty array');
        return { success: true, apps: [] };
      }
      
      // Read all config files
      const configFiles = await fs.readdir(configDir);
      const jsonFiles = configFiles.filter(file => file.endsWith('.json'));
      
      const apps = [];
      for (const file of jsonFiles) {
        try {
          const configPath = path.join(configDir, file);
          const configContent = await fs.readFile(configPath, 'utf-8');
          const appConfig = JSON.parse(configContent);
          
          // Verify the app directory still exists
          try {
            await fs.access(appConfig.localPath);
            // Convert lastUpdated string back to Date object
            if (appConfig.lastUpdated) {
              appConfig.lastUpdated = new Date(appConfig.lastUpdated);
            }
            apps.push(appConfig);
          } catch {
            console.warn('[Main] App directory no longer exists, skipping:', appConfig.localPath);
            // Optionally remove the orphaned config file
            try {
              await fs.unlink(configPath);
              console.log('[Main] Removed orphaned config file:', file);
            } catch (unlinkError) {
              console.warn('[Main] Could not remove orphaned config file:', unlinkError);
            }
          }
        } catch (error) {
          console.error('[Main] Error reading config file:', file, error);
        }
      }
      
      console.log('[Main] Loaded', apps.length, 'saved apps');
      return { success: true, apps };
      
    } catch (error) {
      console.error('[Main] Error loading saved apps:', error);
      return { success: false, error: error.message, apps: [] };
    }
  });


}

app.whenReady().then(async () => {
  try {
    await appMain();
  } catch (error) {
    dialog.showErrorBox('Goose Error', `Failed to create main window: ${error}`);
    app.quit();
  }
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
  // Clean up all running apps before quitting
  console.log('[Main] Cleaning up running apps before quit...');
  for (const [appId, appInfo] of runningApps.entries()) {
    try {
      console.log(`[Main] Stopping app ${appId} before quit`);
      
      // Kill the process
      if (appInfo.process && !appInfo.process.killed) {
        appInfo.process.kill('SIGTERM');
        
        // Force kill after 2 seconds if still running
        setTimeout(() => {
          if (!appInfo.process.killed) {
            console.log(`[Main] Force killing app ${appId} during quit`);
            appInfo.process.kill('SIGKILL');
          }
        }, 2000);
      }
      
      // Also kill any processes on the port
      if (appInfo.port) {
        try {
          const killPort = spawn('lsof', ['-ti', `:${appInfo.port}`]);
          
          let pids = '';
          killPort.stdout.on('data', (data) => {
            pids += data.toString();
          });
          
          killPort.on('close', (code) => {
            if (code === 0 && pids.trim()) {
              const pidList = pids.trim().split('\n');
              pidList.forEach(pid => {
                try {
                  process.kill(parseInt(pid), 'SIGTERM');
                  console.log(`[Main] Killed process ${pid} on port ${appInfo.port} during quit`);
                } catch (error) {
                  console.log(`[Main] Could not kill process ${pid} during quit:`, error.message);
                }
              });
            }
          });
        } catch (error) {
          console.warn(`[Main] Could not kill processes on port ${appInfo.port} during quit:`, error);
        }
      }
    } catch (error) {
      console.error(`[Main] Error stopping app ${appId} during quit:`, error);
    }
  }
  runningApps.clear();

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

app.on('window-all-closed', () => {
  // Only quit if we're not on macOS or don't have a tray icon
  if (process.platform !== 'darwin' || !tray) {
    app.quit();
  }
});
