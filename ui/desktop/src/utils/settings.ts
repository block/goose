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
  toggleNavigation: string | null;
}

export type DefaultKeyboardShortcuts = {
  [K in keyof KeyboardShortcuts]: string;
};

export interface Settings {
  showMenuBarIcon: boolean;
  showDockIcon: boolean;
  enableWakelock: boolean;
  spellcheckEnabled: boolean;
  externalGoosed?: ExternalGoosedConfig;
  globalShortcut?: string | null;
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
  toggleNavigation: 'CommandOrControl+/',
};

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
  return { ...defaultKeyboardShortcuts, ...settings.keyboardShortcuts };
}
