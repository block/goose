// Shared types and constants that can be used in both main and renderer processes
// This file should NOT import any Node.js or Electron modules

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

type DefaultKeyboardShortcuts = {
  [K in keyof KeyboardShortcuts]: string;
};

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

export const defaultKeyboardShortcuts: DefaultKeyboardShortcuts = {
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

export const defaultSettings: Settings = {
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
