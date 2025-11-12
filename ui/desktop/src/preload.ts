import Electron, { contextBridge, ipcRenderer, webUtils } from 'electron';
import { Recipe } from './recipe';

interface NotificationData {
  title: string;
  body: string;
}

interface MessageBoxOptions {
  type?: 'none' | 'info' | 'error' | 'question' | 'warning';
  buttons?: string[];
  defaultId?: number;
  title?: string;
  message: string;
  detail?: string;
}

interface MessageBoxResponse {
  response: number;
  checkboxChecked?: boolean;
}

interface FileResponse {
  file: string;
  filePath: string;
  error: string | null;
  found: boolean;
}

interface SaveDataUrlResponse {
  id: string;
  filePath?: string;
  error?: string;
}

const config = JSON.parse(process.argv.find((arg) => arg.startsWith('{')) || '{}');

interface UpdaterEvent {
  event: string;
  data?: unknown;
}

// Define the API types in a single place
type ElectronAPI = {
  platform: string;
  reactReady: () => void;
  getConfig: () => Record<string, unknown>;
  hideWindow: () => void;
  directoryChooser: (replace?: boolean) => Promise<Electron.OpenDialogReturnValue>;
  createChatWindow: (
    query?: string,
    dir?: string,
    version?: string,
    resumeSessionId?: string,
    recipe?: Recipe,
    viewType?: string
  ) => void;
  logInfo: (txt: string) => void;
  showNotification: (data: NotificationData) => void;
  showMessageBox: (options: MessageBoxOptions) => Promise<MessageBoxResponse>;
  openInChrome: (url: string) => void;
  fetchMetadata: (url: string) => Promise<string>;
  reloadApp: () => void;
  checkForOllama: () => Promise<boolean>;
  selectFileOrDirectory: (defaultPath?: string) => Promise<string | null>;
  startPowerSaveBlocker: () => Promise<number>;
  stopPowerSaveBlocker: () => Promise<void>;
  getBinaryPath: (binaryName: string) => Promise<string>;
  readFile: (directory: string) => Promise<FileResponse>;
  writeFile: (directory: string, content: string) => Promise<boolean>;
  ensureDirectory: (dirPath: string) => Promise<boolean>;
  listFiles: (dirPath: string, extension?: string) => Promise<string[]>;
  getAllowedExtensions: () => Promise<string[]>;
  getPathForFile: (file: File) => string;
  setMenuBarIcon: (show: boolean) => Promise<boolean>;
  getMenuBarIconState: () => Promise<boolean>;
  setDockIcon: (show: boolean) => Promise<boolean>;
  getDockIconState: () => Promise<boolean>;
  getSettings: () => Promise<unknown | null>;
  getSecretKey: () => Promise<string>;
  getGoosedHostPort: () => Promise<string | null>;
  setSchedulingEngine: (engine: string) => Promise<boolean>;
  setWakelock: (enable: boolean) => Promise<boolean>;
  getWakelockState: () => Promise<boolean>;
  openNotificationsSettings: () => Promise<boolean>;
  onMouseBackButtonClicked: (callback: () => void) => void;
  offMouseBackButtonClicked: (callback: () => void) => void;
  on: (
    channel: string,
    callback: (event: Electron.IpcRendererEvent, ...args: unknown[]) => void
  ) => void;
  off: (
    channel: string,
    callback: (event: Electron.IpcRendererEvent, ...args: unknown[]) => void
  ) => void;
  emit: (channel: string, ...args: unknown[]) => void;
  // Functions for image pasting
  saveDataUrlToTemp: (dataUrl: string, uniqueId: string) => Promise<SaveDataUrlResponse>;
  deleteTempFile: (filePath: string) => void;
  // Function for opening external URLs securely
  openExternal: (url: string) => Promise<void>;
  // Function to serve temp images
  getTempImage: (filePath: string) => Promise<string | null>;
  // Update-related functions
  getVersion: () => string;
  checkForUpdates: () => Promise<{ updateInfo: unknown; error: string | null }>;
  downloadUpdate: () => Promise<{ success: boolean; error: string | null }>;
  installUpdate: () => void;
  restartApp: () => void;
  onUpdaterEvent: (callback: (event: UpdaterEvent) => void) => void;
  getUpdateState: () => Promise<{ updateAvailable: boolean; latestVersion?: string } | null>;
  // Recipe warning functions
  closeWindow: () => void;
  hasAcceptedRecipeBefore: (recipeConfig: Recipe) => Promise<boolean>;
  recordRecipeHash: (recipeConfig: Recipe) => Promise<boolean>;
  openDirectoryInExplorer: (directoryPath: string) => Promise<boolean>;
  // Spell checking functions
  spellCheck: (word: string) => Promise<boolean>;
  spellSuggestions: (word: string) => Promise<string[]>;
  // BrowserView functions
  createBrowserView: (url: string, bounds: { x: number; y: number; width: number; height: number }) => Promise<{ viewId: string | null; success: boolean; error?: string }>;
  destroyBrowserView: (viewId: string) => Promise<boolean>;
  updateBrowserViewBounds: (viewId: string, bounds: { x: number; y: number; width: number; height: number }) => Promise<boolean>;
  browserViewNavigate: (viewId: string, url: string) => Promise<boolean>;
  browserViewGoBack: (viewId: string) => Promise<boolean>;
  browserViewGoForward: (viewId: string) => Promise<boolean>;
  browserViewRefresh: (viewId: string) => Promise<boolean>;
  getBrowserViewNavigationState: (viewId: string) => Promise<{ canGoBack: boolean; canGoForward: boolean; url: string; title: string } | null>;

  // BrowserView hide/show functions
  hideBrowserViews: () => Promise<boolean>;
  showBrowserViews: () => Promise<boolean>;

  // Iframe backdrop functions for smooth dock interaction
  createIframeBackdrop: () => Promise<{ success: boolean; backdropData?: any[]; error?: string }>;
  removeIframeBackdrop: () => Promise<boolean>;

  // Dock Window functions
  createDockWindow: () => Promise<{ success: boolean; windowId?: number; error?: string }>;
  showDockWindow: () => Promise<boolean>;
  hideDockWindow: () => Promise<boolean>;
  dockAddContainer: (containerType: string, filePath?: string) => Promise<boolean>;

  // App Installer functions
  cloneRepository: (gitUrl: string, appId: string) => Promise<{ success: boolean; localPath?: string; error?: string }>;
  analyzeProject: (projectPath: string) => Promise<{ success: boolean; name?: string; description?: string; projectType?: string; buildCommand?: string; startCommand?: string; port?: number; requiresInstall?: boolean; packageManager?: string; error?: string }>;
  analyzeProjectWithLLM: (projectPath: string, basicAnalysis: any) => Promise<{ success: boolean; analysis?: any; error?: string }>;
  installProjectDependencies: (projectPath: string, packageManager: string) => Promise<{ success: boolean; error?: string }>;
  saveAppConfiguration: (appConfig: any) => Promise<{ success: boolean; error?: string }>;
  launchApp: (appConfig: any) => Promise<{ success: boolean; error?: string }>;
  stopApp: (appId: string) => Promise<{ success: boolean; error?: string }>;
  isAppRunning: (appId: string) => Promise<boolean>;
  checkPortConflict: (port: number) => Promise<{ hasConflict: boolean; pids: string[] }>;
  killPortProcesses: (port: number) => Promise<{ success: boolean; killedCount: number; totalProcesses: number; errors?: string[] }>;
  removeApp: (appId: string) => Promise<{ success: boolean; error?: string }>;
  showItemInFolder: (path: string) => Promise<void>;
  loadSavedApps: () => Promise<{ success: boolean; apps: any[]; error?: string }>;
};

type AppConfigAPI = {
  get: (key: string) => unknown;
  getAll: () => Record<string, unknown>;
};

const electronAPI: ElectronAPI = {
  platform: process.platform,
  reactReady: () => ipcRenderer.send('react-ready'),
  getConfig: () => {
    if (!config || Object.keys(config).length === 0) {
      console.warn(
        'No config provided by main process. This may indicate an initialization issue.'
      );
    }
    return config;
  },
  hideWindow: () => ipcRenderer.send('hide-window'),
  directoryChooser: () => ipcRenderer.invoke('directory-chooser'),
  createChatWindow: (
    query?: string,
    dir?: string,
    version?: string,
    resumeSessionId?: string,
    recipe?: Recipe,
    viewType?: string
  ) =>
    ipcRenderer.send('create-chat-window', query, dir, version, resumeSessionId, recipe, viewType),
  logInfo: (txt: string) => ipcRenderer.send('logInfo', txt),
  showNotification: (data: NotificationData) => ipcRenderer.send('notify', data),
  showMessageBox: (options: MessageBoxOptions) => ipcRenderer.invoke('show-message-box', options),
  openInChrome: (url: string) => ipcRenderer.send('open-in-chrome', url),
  fetchMetadata: (url: string) => ipcRenderer.invoke('fetch-metadata', url),
  reloadApp: () => ipcRenderer.send('reload-app'),
  checkForOllama: () => ipcRenderer.invoke('check-ollama'),
  selectFileOrDirectory: (defaultPath?: string) =>
    ipcRenderer.invoke('select-file-or-directory', defaultPath),
  startPowerSaveBlocker: () => ipcRenderer.invoke('start-power-save-blocker'),
  stopPowerSaveBlocker: () => ipcRenderer.invoke('stop-power-save-blocker'),
  getBinaryPath: (binaryName: string) => ipcRenderer.invoke('get-binary-path', binaryName),
  readFile: (filePath: string) => ipcRenderer.invoke('read-file', filePath),
  writeFile: (filePath: string, content: string) =>
    ipcRenderer.invoke('write-file', filePath, content),
  ensureDirectory: (dirPath: string) => ipcRenderer.invoke('ensure-directory', dirPath),
  listFiles: (dirPath: string, extension?: string) =>
    ipcRenderer.invoke('list-files', dirPath, extension),
  getPathForFile: (file: File) => webUtils.getPathForFile(file),
  getAllowedExtensions: () => ipcRenderer.invoke('get-allowed-extensions'),
  setMenuBarIcon: (show: boolean) => ipcRenderer.invoke('set-menu-bar-icon', show),
  getMenuBarIconState: () => ipcRenderer.invoke('get-menu-bar-icon-state'),
  setDockIcon: (show: boolean) => ipcRenderer.invoke('set-dock-icon', show),
  getDockIconState: () => ipcRenderer.invoke('get-dock-icon-state'),
  getSettings: () => ipcRenderer.invoke('get-settings'),
  getSecretKey: () => ipcRenderer.invoke('get-secret-key'),
  getGoosedHostPort: () => ipcRenderer.invoke('get-goosed-host-port'),
  setSchedulingEngine: (engine: string) => ipcRenderer.invoke('set-scheduling-engine', engine),
  setWakelock: (enable: boolean) => ipcRenderer.invoke('set-wakelock', enable),
  getWakelockState: () => ipcRenderer.invoke('get-wakelock-state'),
  openNotificationsSettings: () => ipcRenderer.invoke('open-notifications-settings'),
  onMouseBackButtonClicked: (callback: () => void) => {
    // Wrapper that ignores the event parameter.
    const wrappedCallback = (_event: Electron.IpcRendererEvent) => callback();
    ipcRenderer.on('mouse-back-button-clicked', wrappedCallback);
    return wrappedCallback;
  },
  offMouseBackButtonClicked: (callback: () => void) => {
    ipcRenderer.removeListener('mouse-back-button-clicked', callback);
  },
  on: (
    channel: string,
    callback: (event: Electron.IpcRendererEvent, ...args: unknown[]) => void
  ) => {
    ipcRenderer.on(channel, callback);
  },
  off: (
    channel: string,
    callback: (event: Electron.IpcRendererEvent, ...args: unknown[]) => void
  ) => {
    ipcRenderer.off(channel, callback);
  },
  emit: (channel: string, ...args: unknown[]) => {
    ipcRenderer.emit(channel, ...args);
  },
  saveDataUrlToTemp: (dataUrl: string, uniqueId: string): Promise<SaveDataUrlResponse> => {
    return ipcRenderer.invoke('save-data-url-to-temp', dataUrl, uniqueId);
  },
  deleteTempFile: (filePath: string): void => {
    ipcRenderer.send('delete-temp-file', filePath);
  },
  openExternal: (url: string): Promise<void> => {
    return ipcRenderer.invoke('open-external', url);
  },
  getTempImage: (filePath: string): Promise<string | null> => {
    return ipcRenderer.invoke('get-temp-image', filePath);
  },
  getVersion: (): string => {
    return config.GOOSE_VERSION || ipcRenderer.sendSync('get-app-version') || '';
  },
  checkForUpdates: (): Promise<{ updateInfo: unknown; error: string | null }> => {
    return ipcRenderer.invoke('check-for-updates');
  },
  downloadUpdate: (): Promise<{ success: boolean; error: string | null }> => {
    return ipcRenderer.invoke('download-update');
  },
  installUpdate: (): void => {
    ipcRenderer.invoke('install-update');
  },
  restartApp: (): void => {
    ipcRenderer.send('restart-app');
  },
  onUpdaterEvent: (callback: (event: UpdaterEvent) => void): void => {
    ipcRenderer.on('updater-event', (_event, data) => callback(data));
  },
  getUpdateState: (): Promise<{ updateAvailable: boolean; latestVersion?: string } | null> => {
    return ipcRenderer.invoke('get-update-state');
  },
  closeWindow: () => ipcRenderer.send('close-window'),
  hasAcceptedRecipeBefore: (recipeConfig: Recipe) =>
    ipcRenderer.invoke('has-accepted-recipe-before', recipeConfig),
  recordRecipeHash: (recipeConfig: Recipe) =>
    ipcRenderer.invoke('record-recipe-hash', recipeConfig),
  openDirectoryInExplorer: (directoryPath: string) =>
    ipcRenderer.invoke('open-directory-in-explorer', directoryPath),
  // Spell checking functions
  spellCheck: (word: string) => ipcRenderer.invoke('spell-check', word),
  spellSuggestions: (word: string) => ipcRenderer.invoke('spell-suggestions', word),
  // BrowserView functions
  createBrowserView: (url: string, bounds: { x: number; y: number; width: number; height: number }) => 
    ipcRenderer.invoke('create-browser-view', url, bounds),
  destroyBrowserView: (viewId: string) => ipcRenderer.invoke('destroy-browser-view', viewId),
  updateBrowserViewBounds: (viewId: string, bounds: { x: number; y: number; width: number; height: number }) => 
    ipcRenderer.invoke('update-browser-view-bounds', viewId, bounds),
  browserViewNavigate: (viewId: string, url: string) => ipcRenderer.invoke('browser-view-navigate', viewId, url),
  browserViewGoBack: (viewId: string) => ipcRenderer.invoke('browser-view-go-back', viewId),
  browserViewGoForward: (viewId: string) => ipcRenderer.invoke('browser-view-go-forward', viewId),
  browserViewRefresh: (viewId: string) => ipcRenderer.invoke('browser-view-refresh', viewId),
  getBrowserViewNavigationState: (viewId: string) => ipcRenderer.invoke('browser-view-navigation-state', viewId),

  // BrowserView hide/show functions
  hideBrowserViews: () => ipcRenderer.invoke('hide-browser-views'),
  showBrowserViews: () => ipcRenderer.invoke('show-browser-views'),

  // Iframe backdrop functions for smooth dock interaction
  createIframeBackdrop: () => ipcRenderer.invoke('create-iframe-backdrop'),
  removeIframeBackdrop: () => ipcRenderer.invoke('remove-iframe-backdrop'),

  // Dock Window functions
  createDockWindow: () => ipcRenderer.invoke('create-dock-window'),
  showDockWindow: () => ipcRenderer.invoke('show-dock-window'),
  hideDockWindow: () => ipcRenderer.invoke('hide-dock-window'),
  dockAddContainer: (containerType: string, filePath?: string) => ipcRenderer.invoke('dock-add-container', containerType, filePath),

  // App Installer functions
  cloneRepository: (gitUrl: string, appId: string) => ipcRenderer.invoke('clone-repository', gitUrl, appId),
  analyzeProject: (projectPath: string) => ipcRenderer.invoke('analyze-project', projectPath),
  analyzeProjectWithLLM: (projectPath: string, basicAnalysis: any) => ipcRenderer.invoke('analyze-project-with-llm', projectPath, basicAnalysis),
  installProjectDependencies: (projectPath: string, packageManager: string) => 
    ipcRenderer.invoke('install-project-dependencies', projectPath, packageManager),
  saveAppConfiguration: (appConfig: any) => ipcRenderer.invoke('save-app-configuration', appConfig),
  launchApp: (appConfig: any) => ipcRenderer.invoke('launch-app', appConfig),
  stopApp: (appId: string) => ipcRenderer.invoke('stop-app', appId),
  isAppRunning: (appId: string) => ipcRenderer.invoke('is-app-running', appId),
  checkPortConflict: (port: number) => ipcRenderer.invoke('check-port-conflict', port),
  killPortProcesses: (port: number) => ipcRenderer.invoke('kill-port-processes', port),
  removeApp: (appId: string) => ipcRenderer.invoke('remove-app', appId),
  showItemInFolder: (path: string) => ipcRenderer.invoke('show-item-in-folder', path),
  loadSavedApps: () => ipcRenderer.invoke('load-saved-apps'),
};

const appConfigAPI: AppConfigAPI = {
  get: (key: string) => config[key],
  getAll: () => config,
};

// Listen for recipe updates and update config directly
ipcRenderer.on('recipe-decoded', (_, decodedRecipe) => {
  config.recipe = decodedRecipe;
});

// Expose the APIs
contextBridge.exposeInMainWorld('electron', electronAPI);
contextBridge.exposeInMainWorld('appConfig', appConfigAPI);

// Type declaration for TypeScript
declare global {
  interface Window {
    electron: ElectronAPI;
    appConfig: AppConfigAPI;
  }
}
