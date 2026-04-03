import type { ExtensionConfig } from '../../../api/types.gen';
import { FixedExtensionEntry } from '../../ConfigContext';
import defaultBundledExtensionsData from './bundled-extensions.json';
import { nameToKey } from './utils';

// Type definition for built-in extensions from JSON
type BundledExtension = {
  id: string;
  name: string;
  display_name?: string;
  description: string;
  enabled: boolean;
  type: 'builtin' | 'stdio' | 'streamable_http';
  cmd?: string;
  args?: string[];
  uri?: string;
  envs?: { [key: string]: string };
  env_keys?: Array<string>;
  timeout?: number;
  allow_configure?: boolean;
};

let bundledExtensionsOverride: BundledExtension[] | null = null;

async function loadBundledExtensions(): Promise<BundledExtension[]> {
  if (bundledExtensionsOverride !== null) return bundledExtensionsOverride;

  try {
    const distroExtensions = await window.electron.getDistroBundledExtensions();
    if (distroExtensions) {
      bundledExtensionsOverride = distroExtensions;
      return distroExtensions;
    }
  } catch {
    // use default
  }
  return defaultBundledExtensionsData as BundledExtension[];
}

const DEPRECATED_GOOGLE_DRIVE_IDS = ['googledrive', 'google_drive'];
const DEPRECATED_GOOGLE_DRIVE_ENV_KEYS = [
  'GOOGLE_DRIVE_CREDENTIALS_PATH',
  'GOOGLE_DRIVE_OAUTH_PATH',
];

export function isDeprecatedGoogleDriveExtension(ext: FixedExtensionEntry): boolean {
  if (!DEPRECATED_GOOGLE_DRIVE_IDS.includes(nameToKey(ext.name))) {
    return false;
  }
  if (ext.type === 'builtin') {
    return true;
  }
  if (
    ext.type === 'stdio' &&
    'env_keys' in ext &&
    Array.isArray(ext.env_keys) &&
    ext.env_keys.some((key: string) => DEPRECATED_GOOGLE_DRIVE_ENV_KEYS.includes(key))
  ) {
    return true;
  }
  return false;
}

/**
 * Synchronizes built-in extensions with the config system.
 * This function ensures all built-in extensions are added, which is especially
 * important for first-time users with an empty config.yaml.
 *
 * @param existingExtensions Current list of extensions from the config (could be empty)
 * @param addExtensionFn Function to add a new extension to the config
 * @returns Promise that resolves when sync is complete
 */
export async function syncBundledExtensions(
  existingExtensions: FixedExtensionEntry[],
  addExtensionFn: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>
): Promise<void> {
  try {
    const bundledExtensions = await loadBundledExtensions();

    // Process each bundled extension
    for (const bundledExt of bundledExtensions) {
      // Find if this extension already exists
      const existingExt = existingExtensions.find((ext) => nameToKey(ext.name) === bundledExt.id);

      // Skip if extension exists and is already marked as bundled, except when
      // we must migrate deprecated extensions.
      if (
        existingExt &&
        'bundled' in existingExt &&
        existingExt.bundled &&
        !isDeprecatedGoogleDriveExtension(existingExt)
      ) {
        continue;
      }

      // Create the config for this extension
      let extConfig: ExtensionConfig;
      switch (bundledExt.type) {
        case 'builtin':
          extConfig = {
            type: bundledExt.type,
            name: bundledExt.name,
            description: bundledExt.description,
            display_name: bundledExt.display_name,
            timeout: bundledExt.timeout ?? 300,
            bundled: true,
          };
          break;
        case 'stdio':
          extConfig = {
            type: bundledExt.type,
            name: bundledExt.name,
            description: bundledExt.description,
            timeout: bundledExt.timeout,
            cmd: bundledExt.cmd || '',
            args: bundledExt.args || [],
            envs: bundledExt.envs,
            env_keys: bundledExt.env_keys || [],
            bundled: true,
          };
          break;
        case 'streamable_http':
          extConfig = {
            type: bundledExt.type,
            name: bundledExt.name,
            description: bundledExt.description,
            timeout: bundledExt.timeout,
            uri: bundledExt.uri || '',
            bundled: true,
          };
      }

      // Add or update the extension, preserving enabled state if it exists
      const enabled = existingExt ? existingExt.enabled : bundledExt.enabled;
      await addExtensionFn(bundledExt.name, extConfig, enabled);
    }
  } catch (error) {
    console.error('Failed to sync built-in extensions:', error);
    throw error;
  }
}

/**
 * Function to initialize all built-in extensions for a first-time user.
 * This can be called when the application is first installed.
 */
export async function initializeBundledExtensions(
  addExtensionFn: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>
): Promise<void> {
  // Call with an empty list to ensure all built-ins are added
  await syncBundledExtensions([], addExtensionFn);
}
