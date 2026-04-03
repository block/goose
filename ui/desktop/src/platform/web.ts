/* global location, Notification */
import type { PlatformAPI, PlatformEventCallback } from './types';
import { defaultSettings } from '../utils/settings';
import type { Settings, SettingKey } from '../utils/settings';

// Polyfill window.appConfig for web mode (Electron preload normally sets this).
// Config values are injected by goose-web as <meta name="goose:KEY"> tags.
if (typeof window !== 'undefined' && !window.appConfig) {
  const readMeta = (name: string) =>
    document.querySelector(`meta[name="goose:${name}"]`)?.getAttribute('content') ?? undefined;
  const webConfig: Record<string, unknown> = {
    GOOSE_WORKING_DIR: readMeta('working-dir') ?? '',
  };
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => webConfig[key],
    getAll: () => webConfig,
  };
}

const SETTINGS_STORAGE_KEY = 'goose_settings';

function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (raw) return { ...defaultSettings, ...JSON.parse(raw) };
  } catch {
    // fall through
  }
  return { ...defaultSettings };
}

function saveSettings(settings: Settings): void {
  localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
}

/** Simple in-page event bus to replace Electron IPC */
class EventBus {
  private listeners = new Map<string, Set<PlatformEventCallback>>();

  on(channel: string, callback: PlatformEventCallback) {
    if (!this.listeners.has(channel)) {
      this.listeners.set(channel, new Set());
    }
    this.listeners.get(channel)!.add(callback);
  }

  off(channel: string, callback: PlatformEventCallback) {
    this.listeners.get(channel)?.delete(callback);
  }

  emit(channel: string, ...args: unknown[]) {
    this.listeners.get(channel)?.forEach((cb) => {
      try {
        cb(...args);
      } catch (e) {
        console.error(`[EventBus] error in "${channel}" listener:`, e);
      }
    });
  }
}

const bus = new EventBus();

function detectPlatform(): string {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('win')) return 'win32';
  if (ua.includes('mac')) return 'darwin';
  return 'linux';
}

function detectArch(): string {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('arm') || ua.includes('aarch64')) return 'arm64';
  return 'x64';
}

/**
 * Read a `<meta name="goose:KEY" content="VALUE">` tag injected by the server.
 */
function getMeta(name: string): string | null {
  const el = document.querySelector(`meta[name="goose:${name}"]`);
  return el?.getAttribute('content') ?? null;
}

export const webPlatform: PlatformAPI = {
  isWeb: true,
  platform: detectPlatform(),
  arch: detectArch(),

  reactReady: () => {},
  getConfig: () => ({}),
  reloadApp: () => location.reload(),

  getSecretKey: async () => {
    // In web mode the reverse proxy injects the secret server-side;
    // the browser never needs to know it. Return empty string so the
    // API client header is present but valueless (proxy adds the real one).
    return getMeta('secret-key') ?? '';
  },

  getGoosedHostPort: async () => {
    // When served through goose-web reverse proxy, API is on same origin.
    return getMeta('api-base') ?? '';
  },

  // Window management — limited in browser
  hideWindow: () => {},
  closeWindow: () => window.close(),
  createChatWindow: () => {
    window.open(window.location.href, '_blank');
  },

  // Dialogs
  directoryChooser: async () => {
    const path = window.prompt('Enter working directory path:');
    if (path) {
      return { canceled: false, filePaths: [path] };
    }
    return { canceled: true, filePaths: [] };
  },

  showMessageBox: async (options) => {
    const ok = window.confirm(options.message + (options.detail ? `\n\n${options.detail}` : ''));
    return { response: ok ? 0 : 1 };
  },

  showSaveDialog: async (options) => {
    const name = options.defaultPath ?? 'download';
    return { canceled: false, filePath: name };
  },

  selectFileOrDirectory: async () => {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.onchange = () => {
        const file = input.files?.[0];
        resolve(file ? file.name : null);
      };
      input.oncancel = () => resolve(null);
      input.click();
    });
  },

  // File system — not available in browser; return safe defaults
  readFile: async (filePath) => ({
    file: '',
    filePath,
    error: 'File system access not available in web mode',
    found: false,
  }),
  writeFile: async () => false,
  ensureDirectory: async () => false,
  listFiles: async () => [],
  getBinaryPath: async () => '',
  getPathForFile: (file) => URL.createObjectURL(file),
  getAllowedExtensions: async () => [],
  openDirectoryInExplorer: async () => false,

  // Notifications
  showNotification: (data) => {
    if ('Notification' in window && Notification.permission === 'granted') {
      new Notification(data.title, { body: data.body });
    } else if ('Notification' in window && Notification.permission !== 'denied') {
      Notification.requestPermission().then((perm) => {
        if (perm === 'granted') new Notification(data.title, { body: data.body });
      });
    }
  },
  logInfo: (txt) => console.log('[goose]', txt),

  // External links
  openExternal: async (url) => {
    window.open(url, '_blank', 'noopener');
  },
  openInChrome: (url) => {
    window.open(url, '_blank', 'noopener');
  },
  fetchMetadata: async (url) => {
    const resp = await fetch(url);
    return resp.text();
  },

  // Settings via localStorage
  getSetting: async <K extends SettingKey>(key: K): Promise<Settings[K]> => {
    return loadSettings()[key];
  },
  setSetting: async <K extends SettingKey>(key: K, value: Settings[K]): Promise<void> => {
    const s = loadSettings();
    s[key] = value;
    saveSettings(s);
  },

  // System toggles — no-op in browser
  setMenuBarIcon: async () => false,
  getMenuBarIconState: async () => false,
  setDockIcon: async () => false,
  getDockIconState: async () => false,
  setWakelock: async (enable) => {
    try {
      if (enable && 'wakeLock' in navigator) {
        await (navigator as never as { wakeLock: { request: (t: string) => Promise<unknown> } }).wakeLock.request('screen');
        return true;
      }
    } catch {
      // not supported
    }
    return false;
  },
  getWakelockState: async () => false,
  setSpellcheck: async () => false,
  getSpellcheckState: async () => true,
  openNotificationsSettings: async () => false,

  // Event bus
  on: (channel, callback) => bus.on(channel, callback),
  off: (channel, callback) => bus.off(channel, callback),
  emit: (channel, ...args) => bus.emit(channel, ...args),
  broadcastThemeChange: () => {},
  onMouseBackButtonClicked: () => {},
  offMouseBackButtonClicked: () => {},

  // Auto-update — not applicable in web
  getVersion: () => import.meta.env.VITE_GOOSE_VERSION ?? 'web',
  checkForUpdates: async () => ({ updateInfo: null, error: null }),
  downloadUpdate: async () => ({ success: false, error: 'Not available in web mode' }),
  installUpdate: () => {},
  restartApp: () => location.reload(),
  onUpdaterEvent: () => {},
  getUpdateState: async () => null,
  isUsingGitHubFallback: async () => false,

  // Mesh / Ollama — no-op in web
  checkForOllama: async () => false,
  checkMesh: async () => ({ running: false, installed: false, models: [] }),
  startMesh: async () => ({ started: false, error: 'Not available in web mode' }),
  stopMesh: async () => ({ stopped: false }),

  // Recipes — use localStorage
  hasAcceptedRecipeBefore: async () => false,
  recordRecipeHash: async () => true,

  // MCP apps — no-op
  launchApp: async () => {},
  refreshApp: async () => {},
  closeApp: async () => {},

  addRecentDir: async () => false,
};
