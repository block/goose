import { app } from 'electron';
import fs from 'fs';
import path from 'path';

export interface EnvToggles {
  GOOSE_SERVER__MEMORY: boolean;
  GOOSE_SERVER__COMPUTER_CONTROLLER: boolean;
}

export interface ExternalGoosedConfig {
  enabled: boolean;
  url: string;
  secret: string;
}

export interface KeyboardShortcuts {
  focusWindow: string | null;
  quickLauncher: string | null;
  newChat: string | null;
  newChatWindow: string | null;
  openDirectory: string | null;
  settings: string | null;
  find: string | null;
  findNext: string | null;
  findPrevious: string | null;
  alwaysOnTop: string | null;
}

export interface Settings {
  envToggles: EnvToggles;
  showMenuBarIcon: boolean;
  showDockIcon: boolean;
  enableWakelock: boolean;
  spellcheckEnabled: boolean;
  externalGoosed?: ExternalGoosedConfig;
  globalShortcut?: string | null; // Deprecated: use keyboardShortcuts.focusWindow
  keyboardShortcuts?: KeyboardShortcuts;
}

const SETTINGS_FILE = path.join(app.getPath('userData'), 'settings.json');

export const defaultKeyboardShortcuts: KeyboardShortcuts = {
  focusWindow: 'CommandOrControl+Alt+G',
  quickLauncher: 'CommandOrControl+Alt+Shift+G',
  newChat: 'CommandOrControl+T',
  newChatWindow: 'CommandOrControl+N',
  openDirectory: 'CommandOrControl+O',
  settings: 'CommandOrControl+,',
  find: 'CommandOrControl+F',
  findNext: 'CommandOrControl+G',
  findPrevious: 'CommandOrControl+Shift+G',
  alwaysOnTop: 'CommandOrControl+Shift+T',
};

const defaultSettings: Settings = {
  envToggles: {
    GOOSE_SERVER__MEMORY: false,
    GOOSE_SERVER__COMPUTER_CONTROLLER: false,
  },
  showMenuBarIcon: true,
  showDockIcon: true,
  enableWakelock: false,
  spellcheckEnabled: true,
  // globalShortcut is deprecated - not set in defaults, only kept in interface for backwards compatibility
  keyboardShortcuts: defaultKeyboardShortcuts,
};

// Settings management
export function loadSettings(): Settings {
  try {
    if (fs.existsSync(SETTINGS_FILE)) {
      const data = fs.readFileSync(SETTINGS_FILE, 'utf8');
      return JSON.parse(data);
    }
  } catch (error) {
    console.error('Error loading settings:', error);
  }
  return defaultSettings;
}

export function saveSettings(settings: Settings): void {
  try {
    fs.writeFileSync(SETTINGS_FILE, JSON.stringify(settings, null, 2));
  } catch (error) {
    console.error('Error saving settings:', error);
  }
}

export function updateEnvironmentVariables(envToggles: EnvToggles): void {
  if (envToggles.GOOSE_SERVER__MEMORY) {
    process.env.GOOSE_SERVER__MEMORY = 'true';
  } else {
    delete process.env.GOOSE_SERVER__MEMORY;
  }

  if (envToggles.GOOSE_SERVER__COMPUTER_CONTROLLER) {
    process.env.GOOSE_SERVER__COMPUTER_CONTROLLER = 'true';
  } else {
    delete process.env.GOOSE_SERVER__COMPUTER_CONTROLLER;
  }
}

export function getKeyboardShortcuts(settings: Settings): KeyboardShortcuts {
  // Migrate from old globalShortcut field if needed
  if (!settings.keyboardShortcuts && settings.globalShortcut !== undefined) {
    const focusShortcut = settings.globalShortcut;
    const launcherShortcut = focusShortcut ? focusShortcut.replace(/\+G$/i, '+Shift+G') : null;

    return {
      ...defaultKeyboardShortcuts,
      focusWindow: focusShortcut,
      quickLauncher: launcherShortcut,
    };
  }
  return settings.keyboardShortcuts || defaultKeyboardShortcuts;
}
