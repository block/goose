import { app } from 'electron';
import fs from 'fs';
import path from 'path';

// Types
export interface EnvToggles {
  GOOSE_SERVER__MEMORY: boolean;
  GOOSE_SERVER__COMPUTER_CONTROLLER: boolean;
}

export interface Settings {
  envToggles: EnvToggles;
}

// Constants
const SETTINGS_FILE = path.join(app.getPath('userData'), 'settings.json');

const defaultSettings: Settings = {
  envToggles: {
    GOOSE_SERVER__MEMORY: false,
    GOOSE_SERVER__COMPUTER_CONTROLLER: false,
  },
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

// Menu management
export function createEnvironmentMenu(
  envToggles: EnvToggles,
  onToggle: (newToggles: EnvToggles) => void
) {
  return [
    {
      label: 'Enable Memory Mode',
      type: 'checkbox',
      checked: envToggles.GOOSE_SERVER__MEMORY,
      click: (menuItem: { checked: boolean }) => {
        const newToggles = {
          ...envToggles,
          GOOSE_SERVER__MEMORY: menuItem.checked,
        };
        onToggle(newToggles);
      },
    },
    {
      label: 'Enable Computer Controller Mode',
      type: 'checkbox',
      checked: envToggles.GOOSE_SERVER__COMPUTER_CONTROLLER,
      click: (menuItem: { checked: boolean }) => {
        const newToggles = {
          ...envToggles,
          GOOSE_SERVER__COMPUTER_CONTROLLER: menuItem.checked,
        };
        onToggle(newToggles);
      },
    },
  ];
}
