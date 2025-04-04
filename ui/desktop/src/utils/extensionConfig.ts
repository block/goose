import { ExtensionConfig } from '../api/types.gen';

// Key for storing extension configs in localStorage
const EXTENSION_CONFIG_KEY = 'extension_configs';

interface StoredExtensionConfig {
  config: ExtensionConfig;
  enabled: boolean;
}

/**
 * Get all stored extension configs
 */
export function getStoredExtensionConfigs(): Record<string, StoredExtensionConfig> {
  const stored = localStorage.getItem(EXTENSION_CONFIG_KEY);
  if (!stored) {
    return {};
  }
  try {
    return JSON.parse(stored);
  } catch (e) {
    console.error('Failed to parse stored extension configs:', e);
    return {};
  }
}

/**
 * Store an extension config in localStorage
 */
export async function storeExtensionConfig(
  name: string,
  config: ExtensionConfig,
  enabled: boolean
): Promise<void> {
  const configs = getStoredExtensionConfigs();
  configs[name] = { config, enabled };
  localStorage.setItem(EXTENSION_CONFIG_KEY, JSON.stringify(configs));
}

/**
 * Remove an extension config from localStorage
 */
export async function removeExtensionConfig(name: string): Promise<void> {
  const configs = getStoredExtensionConfigs();
  delete configs[name];
  localStorage.setItem(EXTENSION_CONFIG_KEY, JSON.stringify(configs));
}

/**
 * Update the enabled state of an extension in localStorage
 */
export async function updateExtensionEnabled(name: string, enabled: boolean): Promise<void> {
  const configs = getStoredExtensionConfigs();
  if (configs[name]) {
    configs[name].enabled = enabled;
    localStorage.setItem(EXTENSION_CONFIG_KEY, JSON.stringify(configs));
  }
}
