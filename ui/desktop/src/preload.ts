import Electron, { contextBridge, ipcRenderer } from 'electron';

const config = JSON.parse(process.argv.find((arg) => arg.startsWith('{')) || '{}');

// Define the API types in a single place
type ElectronAPI = {
  getConfig: () => Record<string, any>;
  hideWindow: () => void;
  directoryChooser: (replace: string) => void;
  createChatWindow: (query?: string, dir?: string, version?: string) => void;
  logInfo: (txt: string) => void;
  showNotification: (data: any) => void;
  openInChrome: (url: string) => void;
  fetchMetadata: (url: string) => Promise<any>;
  reloadApp: () => void;
  checkForOllama: () => Promise<boolean>;
  selectFileOrDirectory: () => Promise<string>;
  startPowerSaveBlocker: () => Promise<number>;
  stopPowerSaveBlocker: () => Promise<void>;
  getBinaryPath: (binaryName: string) => Promise<string>;
  on: (
    channel: string,
    callback: (event: Electron.IpcRendererEvent, ...args: any[]) => void
  ) => void;
  off: (
    channel: string,
    callback: (event: Electron.IpcRendererEvent, ...args: any[]) => void
  ) => void;
  emit: (channel: string, ...args: any[]) => void;
};

type AppConfigAPI = {
  get: (key: string) => any;
  getAll: () => Record<string, any>;
};

const electronAPI: ElectronAPI = {
  getConfig: () => config,
  hideWindow: () => ipcRenderer.send('hide-window'),
  directoryChooser: (replace: string) => ipcRenderer.send('directory-chooser', replace),
  createChatWindow: (query?: string, dir?: string, version?: string) =>
    ipcRenderer.send('create-chat-window', query, dir, version),
  logInfo: (txt: string) => ipcRenderer.send('logInfo', txt),
  showNotification: (data: any) => ipcRenderer.send('notify', data),
  openInChrome: (url: string) => ipcRenderer.send('open-in-chrome', url),
  fetchMetadata: (url: string) => ipcRenderer.invoke('fetch-metadata', url),
  reloadApp: () => ipcRenderer.send('reload-app'),
  checkForOllama: () => ipcRenderer.invoke('check-ollama'),
  selectFileOrDirectory: () => ipcRenderer.invoke('select-file-or-directory'),
  startPowerSaveBlocker: () => ipcRenderer.invoke('start-power-save-blocker'),
  stopPowerSaveBlocker: () => ipcRenderer.invoke('stop-power-save-blocker'),
  getBinaryPath: (binaryName: string) => ipcRenderer.invoke('get-binary-path', binaryName),
  on: (channel: string, callback: (event: Electron.IpcRendererEvent, ...args: any[]) => void) => {
    ipcRenderer.on(channel, callback);
  },
  off: (channel: string, callback: (event: Electron.IpcRendererEvent, ...args: any[]) => void) => {
    ipcRenderer.off(channel, callback);
  },
  emit: (channel: string, ...args: any[]) => {
    ipcRenderer.emit(channel, ...args);
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
