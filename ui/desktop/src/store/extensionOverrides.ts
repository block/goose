// Store for extension overrides when starting a new session from the hub
// These overrides allow temporarily enabling/disabling extensions before creating a session
// Resets after session creation

import type { ExtensionConfig } from '@/api';

// Map of extension name -> enabled state (overrides from hub view)
type ExtensionOverrides = Map<string, boolean>;

const state: {
  extensionOverrides: ExtensionOverrides;
} = {
  extensionOverrides: new Map(),
};

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

export function getExtensionConfigsWithOverrides(
  allExtensions: Array<{ name: string; enabled: boolean } & Omit<ExtensionConfig, 'name'>>
): ExtensionConfig[] {
  if (state.extensionOverrides.size === 0) {
    return allExtensions
      .filter((ext) => ext.enabled)
      .map((ext) => {
        const { enabled: _enabled, ...config } = ext;
        return config as ExtensionConfig;
      });
  }

  return allExtensions
    .filter((ext) => {
      if (state.extensionOverrides.has(ext.name)) {
        return state.extensionOverrides.get(ext.name);
      }
      return ext.enabled;
    })
    .map((ext) => {
      const { enabled: _enabled, ...config } = ext;
      return config as ExtensionConfig;
    });
}
