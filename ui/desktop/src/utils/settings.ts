export interface ExternalGoslingdConfig {
  enabled: boolean;
  url: string;
  secret: string;
  certFingerprint?: string;
}

export interface ManagedSecretEntry {
  id: string;
  key: string;
  value: string;
}

export type ManagedSecretProfileUse = 'authentication' | 'config' | 'both';

export interface ManagedSecretProfile {
  id: string;
  name: string;
  website: string;
  note: string;
  template: 'custom' | 'vps' | 'supabase';
  useFor: ManagedSecretProfileUse;
  entries: ManagedSecretEntry[];
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

// prettier-ignore
export type LanguageSetting =
  | 'system' | 'en' | 'es' | 'fr' | 'de' | 'it' | 'pt' | 'id' | 'ms' | 'vi'
  | 'hi' | 'ja' | 'ko' | 'ru' | 'tr' | 'zh-CN' | 'zh-TW';

export interface Settings {
  // Desktop app settings
  showMenuBarIcon: boolean;
  disableAutoDownload: boolean;
  showDockIcon: boolean;
  enableWakelock: boolean;
  enableNotifications: boolean;
  spellcheckEnabled: boolean;
  archiveFolder: string | null;
  archivedSessionFiles: Record<string, string>;
  externalGoslingd: ExternalGoslingdConfig;
  managedSecretProfiles: ManagedSecretProfile[];
  globalShortcut?: string | null;
  keyboardShortcuts: KeyboardShortcuts;

  // UI preferences (migrated from localStorage)
  theme: 'dark' | 'light';
  useSystemTheme: boolean;
  language: LanguageSetting;
  responseStyle: string;
  showPricing: boolean;
  seenAnnouncementIds: string[];
}

export type SettingKey = keyof Settings;

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

export const defaultSettings: Settings = {
  // Desktop app settings
  showMenuBarIcon: true,
  disableAutoDownload: false,
  showDockIcon: true,
  enableWakelock: false,
  enableNotifications: true,
  spellcheckEnabled: true,
  archiveFolder: null,
  archivedSessionFiles: {},
  keyboardShortcuts: defaultKeyboardShortcuts,
  externalGoslingd: {
    enabled: false,
    url: '',
    secret: '',
  },
  managedSecretProfiles: [],

  // UI preferences
  theme: 'light',
  useSystemTheme: true,
  language: 'system',
  responseStyle: 'concise',
  showPricing: true,
  seenAnnouncementIds: [],
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
