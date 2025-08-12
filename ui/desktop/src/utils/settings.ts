import { app } from 'electron';
import type Electron from 'electron';
import fs from 'fs';
import path from 'path';

// Types
export interface EnvToggles {
  GOOSE_SERVER__MEMORY: boolean;
  GOOSE_SERVER__COMPUTER_CONTROLLER: boolean;
}

export type SchedulingEngine = 'builtin-cron' | 'temporal';

export interface Settings {
  envToggles: EnvToggles;
  showMenuBarIcon: boolean;
  showDockIcon: boolean;
  schedulingEngine: SchedulingEngine;
  showQuitConfirmation: boolean;
  enableWakelock: boolean;
}

// Constants
const SETTINGS_FILE = path.join(app.getPath('userData'), 'settings.json');

const defaultSettings: Settings = {
  envToggles: {
    GOOSE_SERVER__MEMORY: false,
    GOOSE_SERVER__COMPUTER_CONTROLLER: false,
  },
  showMenuBarIcon: true,
  showDockIcon: true,
  schedulingEngine: 'builtin-cron',
  showQuitConfirmation: true,
  enableWakelock: false,
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

// Environment management
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

export function updateSchedulingEngineEnvironment(schedulingEngine: SchedulingEngine): void {
  // Set GOOSE_SCHEDULER_TYPE based on the scheduling engine setting
  if (schedulingEngine === 'temporal') {
    process.env.GOOSE_SCHEDULER_TYPE = 'temporal';
  } else {
    process.env.GOOSE_SCHEDULER_TYPE = 'legacy';
  }
}

// Build Environment submenu items for the View menu
export function createEnvironmentMenu(
  toggles: EnvToggles,
  onChange: (newToggles: EnvToggles) => void
): Electron.MenuItemConstructorOptions[] {
  return [
    {
      label: 'Server Memory',
      type: 'checkbox',
      checked: !!toggles.GOOSE_SERVER__MEMORY,
      click: () =>
        onChange({
          ...toggles,
          GOOSE_SERVER__MEMORY: !toggles.GOOSE_SERVER__MEMORY,
        }),
    },
    {
      label: 'Computer Controller',
      type: 'checkbox',
      checked: !!toggles.GOOSE_SERVER__COMPUTER_CONTROLLER,
      click: () =>
        onChange({
          ...toggles,
          GOOSE_SERVER__COMPUTER_CONTROLLER: !toggles.GOOSE_SERVER__COMPUTER_CONTROLLER,
        }),
    },
  ];
}
