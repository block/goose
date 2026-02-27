import fsSync from 'node:fs';
import type { Settings } from './settings';
import { defaultKeyboardShortcuts } from './settings';

// Default settings used when no settings file exists or when parsing fails
export const defaultSettings: Settings = {
  showMenuBarIcon: true,
  showDockIcon: true,
  enableWakelock: false,
  spellcheckEnabled: true,
  keyboardShortcuts: defaultKeyboardShortcuts,
};

// Reads and parses the settings file at the given path.
// Returns defaultSettings if the file does not exist or contains invalid JSON.
export function getSettings(settingsFilePath: string): Settings {
  if (fsSync.existsSync(settingsFilePath)) {
    try {
      const data = fsSync.readFileSync(settingsFilePath, 'utf8');
      return JSON.parse(data);
    } catch {
      console.warn(
        `[Settings] Failed to parse ${settingsFilePath}, falling back to defaults`
      );
      return defaultSettings;
    }
  }
  return defaultSettings;
}

// Applies a modifier function to the current settings, then writes atomically
// via a temp file + rename to prevent partial writes on crash/power loss.
export function updateSettings(
  settingsFilePath: string,
  modifier: (settings: Settings) => void
): void {
  const settings = getSettings(settingsFilePath);
  modifier(settings);

  const tmpPath = settingsFilePath + '.tmp';
  fsSync.writeFileSync(tmpPath, JSON.stringify(settings, null, 2));
  fsSync.renameSync(tmpPath, settingsFilePath);
}
