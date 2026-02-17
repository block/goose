/**
 * Tauri bridge — provides the same interface as `window.electron` but backed by Tauri v2 APIs.
 *
 * During migration this is assigned to `window.electron` in renderer.tsx so existing
 * components work without modification.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, emit, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open, save, message, confirm } from '@tauri-apps/plugin-dialog';
import { sendNotification } from '@tauri-apps/plugin-notification';
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
import { relaunch } from '@tauri-apps/plugin-process';
import { check } from '@tauri-apps/plugin-updater';

import type { Settings } from './utils/settings';

// ── Types matching the Electron preload ────────────────────────────────

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

interface SaveDialogOptions {
  title?: string;
  defaultPath?: string;
  buttonLabel?: string;
  filters?: Array<{ name: string; extensions: string[] }>;
  message?: string;
  nameFieldLabel?: string;
  showsTagField?: boolean;
}

interface SaveDialogResponse {
  canceled: boolean;
  filePath?: string;
}

interface FileResponse {
  file: string;
  filePath: string;
  error: string | null;
  found: boolean;
}

interface UpdaterEvent {
  event: string;
  data?: unknown;
}

// Store unlisten promises for event cleanup (stored immediately to avoid race conditions)
const unlistenMap = new Map<string, Map<Function, Promise<UnlistenFn>>>();

// Detect platform from navigator
function detectPlatform(): string {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('mac')) return 'darwin';
  if (ua.includes('win')) return 'win32';
  return 'linux';
}

// ── Config cache ──────────────────────────────────────────────────────
let cachedConfig: Record<string, unknown> | null = null;

async function loadConfig(): Promise<Record<string, unknown>> {
  if (cachedConfig) return cachedConfig;
  cachedConfig = await invoke<Record<string, unknown>>('get_config');
  return cachedConfig;
}

// ── The bridge object ─────────────────────────────────────────────────

export const tauriBridge = {
  platform: detectPlatform(),

  reactReady: () => {
    // In Tauri, the frontend is ready when the module loads.
    // Emit an event so the backend knows.
    emit('react-ready', {});
  },

  getConfig: () => {
    // Return cached config synchronously if available, otherwise empty object
    // The async version is used during init
    return cachedConfig ?? {};
  },

  // ── Window management ───────────────────────────────────────────────

  hideWindow: () => getCurrentWindow().hide(),

  closeWindow: () => getCurrentWindow().close(),

  reloadApp: () => window.location.reload(),

  createChatWindow: (
    query?: string,
    dir?: string,
    version?: string,
    resumeSessionId?: string,
    viewType?: string,
    recipeDeeplink?: string
  ) =>
    invoke('create_chat_window', {
      query,
      dir,
      version,
      resumeSessionId,
      viewType,
      recipeDeeplink,
    }),

  // ── Dialogs ─────────────────────────────────────────────────────────

  directoryChooser: async () => {
    const selected = await open({ directory: true, multiple: false });
    return {
      canceled: selected === null,
      filePaths: selected ? [selected] : [],
    };
  },

  showSaveDialog: async (options: SaveDialogOptions): Promise<SaveDialogResponse> => {
    const filePath = await save({
      title: options.title,
      defaultPath: options.defaultPath,
      filters: options.filters,
    });
    return {
      canceled: filePath === null,
      filePath: filePath ?? undefined,
    };
  },

  showMessageBox: async (options: MessageBoxOptions): Promise<MessageBoxResponse> => {
    if (options.buttons && options.buttons.length > 0) {
      const result = await confirm(options.message, {
        title: options.title,
        kind: options.type === 'error' ? 'error' : options.type === 'warning' ? 'warning' : 'info',
      });
      return { response: result ? 0 : 1 };
    }
    await message(options.message, {
      title: options.title,
      kind: options.type === 'error' ? 'error' : options.type === 'warning' ? 'warning' : 'info',
    });
    return { response: 0 };
  },

  selectFileOrDirectory: (defaultPath?: string) =>
    invoke<string | null>('select_file_or_directory', { defaultPath }),

  // ── File operations ─────────────────────────────────────────────────

  readFile: (filePath: string) => invoke<FileResponse>('read_file', { filePath }),

  writeFile: (filePath: string, content: string) =>
    invoke<boolean>('write_file', { filePath, content }),

  ensureDirectory: (dirPath: string) => invoke<boolean>('ensure_directory', { dirPath }),

  listFiles: (dirPath: string, extension?: string) =>
    invoke<string[]>('list_files', { dirPath, extension }),

  openDirectoryInExplorer: (directoryPath: string) =>
    invoke<boolean>('open_directory_in_explorer', { directoryPath }),

  getPathForFile: (file: File) => {
    // In Tauri, use the file's webkitRelativePath or name as a fallback
    // The webview provides native file paths via drag events
    return (file as any).path || file.name;
  },

  // ── Settings ────────────────────────────────────────────────────────

  getSettings: () => invoke<Settings>('get_settings'),

  saveSettings: (settings: Settings) => invoke<boolean>('save_settings', { settings }),

  getSecretKey: () => invoke<string>('get_secret_key'),

  getGoosedHostPort: () => invoke<string | null>('get_goosed_host_port'),

  // ── Notifications & external ────────────────────────────────────────

  showNotification: (data: NotificationData) => {
    sendNotification({ title: data.title, body: data.body });
  },

  openExternal: (url: string) => openUrl(url),

  openInChrome: (url: string) => openUrl(url),

  fetchMetadata: (url: string) => invoke<string>('fetch_metadata', { url }),

  // ── System state ────────────────────────────────────────────────────

  setMenuBarIcon: (_show: boolean) => Promise.resolve(true),
  getMenuBarIconState: () => Promise.resolve(true),

  setDockIcon: (_show: boolean) => Promise.resolve(true),
  getDockIconState: () => Promise.resolve(true),

  setWakelock: (enable: boolean) => invoke<boolean>('set_wakelock', { enable }),
  getWakelockState: () => invoke<boolean>('get_wakelock_state'),

  setSpellcheck: (enable: boolean) => invoke<boolean>('set_spellcheck', { enable }),
  getSpellcheckState: () => invoke<boolean>('get_spellcheck_state'),

  openNotificationsSettings: async () => {
    // Open system notification settings
    const platform = detectPlatform();
    if (platform === 'darwin') {
      await openUrl(
        'x-apple.systempreferences:com.apple.Notifications-Settings'
      );
    } else if (platform === 'win32') {
      await openUrl('ms-settings:notifications');
    }
    return true;
  },

  // ── Config ──────────────────────────────────────────────────────────

  getVersion: () => {
    return cachedConfig?.GOOSE_VERSION as string ?? '';
  },

  checkForOllama: () => invoke<boolean>('check_for_ollama'),

  getAllowedExtensions: () => invoke<string[]>('get_allowed_extensions'),

  getBinaryPath: (_binaryName: string) => Promise.resolve(''),

  // ── Updates ─────────────────────────────────────────────────────────

  checkForUpdates: async () => {
    try {
      const update = await check();
      if (update) {
        return {
          updateInfo: { version: update.version, body: update.body },
          error: null,
        };
      }
      return { updateInfo: null, error: null };
    } catch (e) {
      return { updateInfo: null, error: String(e) };
    }
  },

  downloadUpdate: async () => {
    try {
      const update = await check();
      if (update) {
        await update.downloadAndInstall();
        return { success: true, error: null };
      }
      return { success: false, error: 'No update available' };
    } catch (e) {
      return { success: false, error: String(e) };
    }
  },

  installUpdate: () => {
    relaunch();
  },

  restartApp: () => {
    relaunch();
  },

  onUpdaterEvent: (callback: (event: UpdaterEvent) => void) => {
    listen('updater-event', (event) => {
      callback(event.payload as UpdaterEvent);
    });
  },

  getUpdateState: async (): Promise<{
    updateAvailable: boolean;
    latestVersion?: string;
  } | null> => {
    try {
      const update = await check();
      if (update) {
        return { updateAvailable: true, latestVersion: update.version };
      }
      return { updateAvailable: false };
    } catch {
      return null;
    }
  },

  isUsingGitHubFallback: () => Promise.resolve(false),

  // ── Recipes ─────────────────────────────────────────────────────────

  hasAcceptedRecipeBefore: (recipe: any) =>
    invoke<boolean>('has_accepted_recipe_before', { recipe }),

  recordRecipeHash: (recipe: any) =>
    invoke<boolean>('record_recipe_hash', { recipe }),

  // ── Apps ─────────────────────────────────────────────────────────────

  launchApp: (app: any) => invoke<void>('launch_app', { gooseApp: app }),

  refreshApp: (app: any) => invoke<void>('refresh_app', { gooseApp: app }),

  closeApp: (appName: string) => invoke<void>('close_app', { appName }),

  // ── Recent dirs ─────────────────────────────────────────────────────

  addRecentDir: (dir: string) => invoke<boolean>('add_recent_dir', { dir }),

  // ── Theme ───────────────────────────────────────────────────────────

  broadcastThemeChange: (themeData: {
    mode: string;
    useSystemTheme: boolean;
    theme: string;
  }) => {
    emit('broadcast-theme-change', themeData);
  },

  // ── Events ──────────────────────────────────────────────────────────

  on: (channel: string, callback: (...args: any[]) => void) => {
    if (!unlistenMap.has(channel)) {
      unlistenMap.set(channel, new Map());
    }

    // Store the promise immediately to avoid race conditions with off()
    const unlistenPromise = listen(channel, (event) => {
      const payload = event?.payload;
      callback({ sender: null, preventDefault: () => {} }, ...(Array.isArray(payload) ? payload : [payload]));
    });

    unlistenMap.get(channel)!.set(callback, unlistenPromise);
  },

  off: (channel: string, callback: (...args: any[]) => void) => {
    const channelMap = unlistenMap.get(channel);
    if (channelMap) {
      const unlistenPromise = channelMap.get(callback);
      if (unlistenPromise) {
        channelMap.delete(callback);
        // Await the promise to ensure we unlisten even if listen() hasn't resolved yet
        unlistenPromise.then((unlisten) => unlisten());
      }
    }
  },

  emit: (channel: string, ...args: any[]) => {
    emit(channel, args);
  },

  // ── Mouse ───────────────────────────────────────────────────────────

  onMouseBackButtonClicked: (callback: () => void) => {
    listen('mouse-back-button-clicked', () => callback());
  },

  offMouseBackButtonClicked: (_callback: () => void) => {
    // Cleanup handled via unlisten
  },

  // ── Logging ─────────────────────────────────────────────────────────

  logInfo: (txt: string) => {
    console.log('[goose]', txt);
  },
};

// ── AppConfig compatibility ───────────────────────────────────────────

export const appConfigBridge = {
  get: (key: string): unknown => {
    return cachedConfig?.[key];
  },
  getAll: (): Record<string, unknown> => {
    return cachedConfig ?? {};
  },
};

// ── Initialization ────────────────────────────────────────────────────

export async function initTauriBridge(): Promise<void> {
  await loadConfig();
}
