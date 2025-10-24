// Registry mapping: channel -> (originalListener -> wrappedListener)
import Electron, { contextBridge, ipcRenderer, webUtils } from 'electron';
import { Recipe } from './recipe';

// Types and internal mapping from original listener -> wrapped ipcRenderer listener
type RawListener =
  | ((event: Electron.IpcRendererEvent, ...args: unknown[]) => void)
  | ((...args: unknown[]) => void);
type WrappedListener = (event: Electron.IpcRendererEvent, ...args: unknown[]) => void;
const listenerMap: Map<string, Map<RawListener, WrappedListener>> = new Map();

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

interface LogSizeInfo {
  total_bytes: number;
  total_mb: number;
  total_gb: number;
  file_count: number;
  log_path: string;
}

interface ClearLogsResult {
  success: boolean;
  files_cleared: number;
  bytes_cleared: number;
  mb_cleared: number;
  message?: string;
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
    viewType?: string,
    recipeId?: string
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
  on: <T = unknown>(
    channel: string,
    callback: ((event: Electron.IpcRendererEvent, ...args: T[]) => void) | ((...args: T[]) => void)
  ) => void;
  off: <T = unknown>(
    channel: string,
    callback: ((event: Electron.IpcRendererEvent, ...args: T[]) => void) | ((...args: T[]) => void)
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
  hasAcceptedRecipeBefore: (recipe: Recipe) => Promise<boolean>;
  recordRecipeHash: (recipe: Recipe) => Promise<boolean>;
  openDirectoryInExplorer: (directoryPath: string) => Promise<boolean>;
  // Log management functions
  getLogSize: () => Promise<LogSizeInfo>;
  clearLogs: () => Promise<ClearLogsResult>;
  getLogPath: () => Promise<string>;
  openLogsFolder: () => Promise<void>;
};

type AppConfigAPI = {
  get: (key: string) => unknown;
  getAll: () => Record<string, unknown>;
};

const electronAPI = {
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
    viewType?: string,
    recipeId?: string
  ) =>
    ipcRenderer.send(
      'create-chat-window',
      query,
      dir,
      version,
      resumeSessionId,
      recipe,
      viewType,
      recipeId
    ),
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
    ipcRenderer.removeListener(
      'mouse-back-button-clicked',
      callback as unknown as (...args: unknown[]) => void
    );
  },
  on: <T = unknown>(
    channel: string,
    callback: (event: Electron.IpcRendererEvent, ...args: T[]) => void
  ) => {
    // Wrap the renderer callback so the event parameter is stripped and only the args are passed.
    const map = listenerMap.get(channel) ?? new Map<RawListener, WrappedListener>();
    const wrapped = (_event: Electron.IpcRendererEvent, ...args: T[]) => {
      // Try calling the callback as (event, ...args) first (many handlers expect the event),
      // otherwise fall back to calling with only the args (payload-only handlers).
      try {
        (callback as unknown as (event: Electron.IpcRendererEvent, ...args: T[]) => void)(
          _event,
          ...args
        );
        return;
      } catch {
        // ignore and try payload-only invocation
      }
      try {
        (callback as unknown as (...args: T[]) => void)(...args);
      } catch (err) {
        // Don't let listener exceptions break the ipc pipeline

        console.error('Renderer listener error for channel', channel, err);
      }
    };
    map.set(callback as RawListener, wrapped as WrappedListener);
    listenerMap.set(channel, map);
    ipcRenderer.on(channel, wrapped);
  },
  off: <T = unknown>(
    channel: string,
    callback: (event: Electron.IpcRendererEvent, ...args: T[]) => void
  ) => {
    const map = listenerMap.get(channel);
    const wrapped = map?.get(callback as unknown as RawListener) as WrappedListener | undefined;
    if (wrapped) {
      ipcRenderer.off(channel, wrapped);
      map?.delete(callback as unknown as RawListener);
      if (map && map.size === 0) listenerMap.delete(channel);
    }
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
  hasAcceptedRecipeBefore: (recipe: Recipe) =>
    ipcRenderer.invoke('has-accepted-recipe-before', recipe),
  recordRecipeHash: (recipe: Recipe) => ipcRenderer.invoke('record-recipe-hash', recipe),
  openDirectoryInExplorer: (directoryPath: string) =>
    ipcRenderer.invoke('open-directory-in-explorer', directoryPath),
  getLogSize: () => ipcRenderer.invoke('get-log-size') as Promise<LogSizeInfo>,
  clearLogs: () => ipcRenderer.invoke('clear-logs') as unknown as Promise<ClearLogsResult>,
  getLogPath: () => ipcRenderer.invoke('get-log-path') as Promise<string>,
  openLogsFolder: () => ipcRenderer.invoke('open-logs-folder') as Promise<void>,
} as unknown as ElectronAPI;

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
