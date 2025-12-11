// Store for new chat configuration
// Acts as a cache that can be updated from UI or synced from session
// Resets on page refresh - defaults to window.appConfig.get('GOOSE_WORKING_DIR')

import type { ExtensionConfig } from '../api';

// Map of extension name -> enabled state (overrides from hub view)
type ExtensionOverrides = Map<string, boolean>;

interface NewChatState {
  workingDir: string | null;
  extensionOverrides: ExtensionOverrides;
}

const state: NewChatState = {
  workingDir: null,
  extensionOverrides: new Map(),
};

export function setWorkingDir(dir: string): void {
  state.workingDir = dir;
}

export function getWorkingDir(): string {
  return state.workingDir ?? (window.appConfig.get('GOOSE_WORKING_DIR') as string);
}

export function clearWorkingDir(): void {
  state.workingDir = null;
}

// Extension override functions
export function setExtensionOverride(name: string, enabled: boolean): void {
  state.extensionOverrides.set(name, enabled);
}

export function getExtensionOverride(name: string): boolean | undefined {
  return state.extensionOverrides.get(name);
}

export function hasExtensionOverrides(): boolean {
  return state.extensionOverrides.size > 0;
}

export function getExtensionOverrides(): ExtensionOverrides {
  return state.extensionOverrides;
}

export function clearExtensionOverrides(): void {
  state.extensionOverrides.clear();
}

// Get extension configs with overrides applied
export function getExtensionConfigsWithOverrides(
  allExtensions: Array<{ name: string; enabled: boolean } & Omit<ExtensionConfig, 'name'>>
): ExtensionConfig[] {
  if (state.extensionOverrides.size === 0) {
    // No overrides, return global enabled extensions
    return allExtensions
      .filter((ext) => ext.enabled)
      .map((ext) => {
        const { enabled: _enabled, ...config } = ext;
        return config as ExtensionConfig;
      });
  }

  // Apply overrides
  return allExtensions
    .filter((ext) => {
      // Check if we have an override for this extension
      if (state.extensionOverrides.has(ext.name)) {
        return state.extensionOverrides.get(ext.name);
      }
      // Otherwise use the global enabled state
      return ext.enabled;
    })
    .map((ext) => {
      const { enabled: _enabled, ...config } = ext;
      return config as ExtensionConfig;
    });
}

// Generic getters/setters for future extensibility
export function getNewChatState(): Readonly<NewChatState> {
  return { ...state };
}

export function resetNewChatState(): void {
  state.workingDir = null;
  state.extensionOverrides.clear();
}
