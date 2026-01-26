import { app } from 'electron';
import fs from 'fs';
import path from 'path';

// Re-export types and constants for backwards compatibility in main process
export type {
  EnvToggles,
  ExternalGoosedConfig,
  KeyboardShortcuts,
  Settings,
} from './settingsTypes';
export { defaultKeyboardShortcuts, defaultSettings } from './settingsTypes';

import {
  Settings,
  KeyboardShortcuts,
  EnvToggles,
  defaultKeyboardShortcuts,
  defaultSettings,
} from './settingsTypes';

const SETTINGS_FILE = path.join(app.getPath('userData'), 'settings.json');

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
  if (!settings.keyboardShortcuts && settings.globalShortcut !== undefined) {
    const focusShortcut = settings.globalShortcut;
    let launcherShortcut: string | null = null;

    if (focusShortcut) {
      if (focusShortcut.includes('Shift')) {
        launcherShortcut = focusShortcut;
      } else {
        launcherShortcut = focusShortcut.replace(/\+([Gg])$/, '+Shift+$1');
      }
    }

    return {
      ...defaultKeyboardShortcuts,
      focusWindow: focusShortcut,
      quickLauncher: launcherShortcut,
    };
  }
  return settings.keyboardShortcuts || defaultKeyboardShortcuts;
}
