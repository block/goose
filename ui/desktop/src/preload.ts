import Electron, { contextBridge, ipcRenderer, webUtils } from 'electron';
import { Recipe } from './recipe';

// RecipeConfig is used for window creation and should match Recipe interface
type RecipeConfig = Recipe;

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
    recipeConfig?: RecipeConfig,
    viewType?: string
  ) => void;
  logInfo: (txt: string) => void;
  showNotification: (data: NotificationData) => void;
  showMessageBox: (options: MessageBoxOptions) => Promise<MessageBoxResponse>;
  openInChrome: (url: string) => void;
  fetchMetadata: (url: string) => Promise<string>;
  reloadApp: () => void;
  checkForOllama: () => Promise<boolean>;
  selectFileOrDirectory: () => Promise<string | null>;
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
  setSchedulingEngine: (engine: string) => Promise<boolean>;
  setQuitConfirmation: (show: boolean) => Promise<boolean>;
  getQuitConfirmationState: () => Promise<boolean>;
  openNotificationsSettings: () => Promise<boolean>;
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
};

type AppConfigAPI = {
  get: (key: string) => unknown;
  getAll: () => Record<string, unknown>;
};

const electronAPI: ElectronAPI = {
  platform: process.platform,
  reactReady: () => ipcRenderer.send('react-ready'),
  getConfig: () => config,
  hideWindow: () => ipcRenderer.send('hide-window'),
  directoryChooser: (replace?: boolean) => ipcRenderer.invoke('directory-chooser', replace),
  createChatWindow: (
    query?: string,
    dir?: string,
    version?: string,
    resumeSessionId?: string,
    recipeConfig?: RecipeConfig,
    viewType?: string
  ) =>
    ipcRenderer.send(
      'create-chat-window',
      query,
      dir,
      version,
      resumeSessionId,
      recipeConfig,
      viewType
    ),
  logInfo: (txt: string) => ipcRenderer.send('logInfo', txt),
  showNotification: (data: NotificationData) => ipcRenderer.send('notify', data),
  showMessageBox: (options: MessageBoxOptions) => ipcRenderer.invoke('show-message-box', options),
  openInChrome: (url: string) => ipcRenderer.send('open-in-chrome', url),
  fetchMetadata: (url: string) => ipcRenderer.invoke('fetch-metadata', url),
  reloadApp: () => ipcRenderer.send('reload-app'),
  checkForOllama: () => ipcRenderer.invoke('check-ollama'),
  selectFileOrDirectory: () => ipcRenderer.invoke('select-file-or-directory'),
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
  setSchedulingEngine: (engine: string) => ipcRenderer.invoke('set-scheduling-engine', engine),
  setQuitConfirmation: (show: boolean) => ipcRenderer.invoke('set-quit-confirmation', show),
  getQuitConfirmationState: () => ipcRenderer.invoke('get-quit-confirmation-state'),
  openNotificationsSettings: () => ipcRenderer.invoke('open-notifications-settings'),
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
};

const appConfigAPI: AppConfigAPI = {
  get: (key: string) => config[key],
  getAll: () => config,
};

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
