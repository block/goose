import type { Settings, SettingKey } from '../utils/settings';
import type { Recipe } from '../recipe';
import type { GooseApp } from '../api';

export interface NotificationData {
  title: string;
  body: string;
}

export interface MessageBoxOptions {
  type?: 'none' | 'info' | 'error' | 'question' | 'warning';
  buttons?: string[];
  defaultId?: number;
  title?: string;
  message: string;
  detail?: string;
}

export interface MessageBoxResponse {
  response: number;
  checkboxChecked?: boolean;
}

export interface SaveDialogOptions {
  title?: string;
  defaultPath?: string;
  buttonLabel?: string;
  filters?: Array<{ name: string; extensions: string[] }>;
  message?: string;
  nameFieldLabel?: string;
  showsTagField?: boolean;
}

export interface SaveDialogResponse {
  canceled: boolean;
  filePath?: string;
}

export interface FileResponse {
  file: string;
  filePath: string;
  error: string | null;
  found: boolean;
}

export interface OpenDialogReturnValue {
  canceled: boolean;
  filePaths: string[];
}

export interface CreateChatWindowOptions {
  query?: string;
  dir?: string;
  version?: string;
  resumeSessionId?: string;
  viewType?: string;
  recipeId?: string;
}

export interface UpdaterEvent {
  event: string;
  data?: unknown;
}

export type PlatformEventCallback = (...args: unknown[]) => void;

export interface PlatformAPI {
  isWeb: boolean;
  platform: string;
  arch: string;

  // Lifecycle
  reactReady: () => void;
  getConfig: () => Record<string, unknown>;
  reloadApp: () => void;

  // Backend connection
  getSecretKey: () => Promise<string>;
  getGoosedHostPort: () => Promise<string | null>;

  // Window management
  hideWindow: () => void;
  closeWindow: () => void;
  createChatWindow: (options?: CreateChatWindowOptions) => void;

  // Dialogs
  directoryChooser: () => Promise<OpenDialogReturnValue>;
  showMessageBox: (options: MessageBoxOptions) => Promise<MessageBoxResponse>;
  showSaveDialog: (options: SaveDialogOptions) => Promise<SaveDialogResponse>;
  selectFileOrDirectory: (defaultPath?: string) => Promise<string | null>;

  // File system
  readFile: (filePath: string) => Promise<FileResponse>;
  writeFile: (filePath: string, content: string) => Promise<boolean>;
  ensureDirectory: (dirPath: string) => Promise<boolean>;
  listFiles: (dirPath: string, extension?: string) => Promise<string[]>;
  getBinaryPath: (binaryName: string) => Promise<string>;
  getPathForFile: (file: File) => string;
  getAllowedExtensions: () => Promise<string[]>;
  openDirectoryInExplorer: (directoryPath: string) => Promise<boolean>;

  // Notifications
  showNotification: (data: NotificationData) => void;
  logInfo: (txt: string) => void;

  // External links
  openExternal: (url: string) => Promise<void>;
  openInChrome: (url: string) => void;
  fetchMetadata: (url: string) => Promise<string>;

  // Settings
  getSetting: <K extends SettingKey>(key: K) => Promise<Settings[K]>;
  setSetting: <K extends SettingKey>(key: K, value: Settings[K]) => Promise<void>;

  // System toggles
  setMenuBarIcon: (show: boolean) => Promise<boolean>;
  getMenuBarIconState: () => Promise<boolean>;
  setDockIcon: (show: boolean) => Promise<boolean>;
  getDockIconState: () => Promise<boolean>;
  setWakelock: (enable: boolean) => Promise<boolean>;
  getWakelockState: () => Promise<boolean>;
  setSpellcheck: (enable: boolean) => Promise<boolean>;
  getSpellcheckState: () => Promise<boolean>;
  openNotificationsSettings: () => Promise<boolean>;

  // Events (IPC bridge)
  on: (channel: string, callback: PlatformEventCallback) => void;
  off: (channel: string, callback: PlatformEventCallback) => void;
  emit: (channel: string, ...args: unknown[]) => void;
  broadcastThemeChange: (themeData: {
    mode: string;
    useSystemTheme: boolean;
    theme: string;
    tokensUpdated?: boolean;
  }) => void;
  onMouseBackButtonClicked: (callback: () => void) => void;
  offMouseBackButtonClicked: (callback: () => void) => void;

  // Auto-update
  getVersion: () => string;
  checkForUpdates: () => Promise<{ updateInfo: unknown; error: string | null }>;
  downloadUpdate: () => Promise<{ success: boolean; error: string | null }>;
  installUpdate: () => void;
  restartApp: () => void;
  onUpdaterEvent: (callback: (event: UpdaterEvent) => void) => void;
  getUpdateState: () => Promise<{ updateAvailable: boolean; latestVersion?: string } | null>;
  isUsingGitHubFallback: () => Promise<boolean>;

  // Mesh / local inference
  checkForOllama: () => Promise<boolean>;
  checkMesh: () => Promise<{
    running: boolean;
    installed: boolean;
    models: string[];
    token?: string;
    peerCount?: number;
    nodeStatus?: string;
    binaryPath?: string;
  }>;
  startMesh: (args: string[]) => Promise<{ started: boolean; error?: string; pid?: number }>;
  stopMesh: () => Promise<{ stopped: boolean }>;

  // Recipes
  hasAcceptedRecipeBefore: (recipe: Recipe) => Promise<boolean>;
  recordRecipeHash: (recipe: Recipe) => Promise<boolean>;

  // MCP apps
  launchApp: (app: GooseApp) => Promise<void>;
  refreshApp: (app: GooseApp) => Promise<void>;
  closeApp: (appName: string) => Promise<void>;

  // Recent dirs
  addRecentDir: (dir: string) => Promise<boolean>;
}
