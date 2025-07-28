import type { ExtensionConfig } from '../../../api/types.gen';
import { FixedExtensionEntry } from '../../ConfigContext';
import bundledExtensionsData from './bundled-extensions.json';
import { nameToKey } from './utils';

// Type definition for built-in extensions from JSON
type BundledExtension = {
  id: string;
  name: string;
  display_name?: string;
  description?: string;
  enabled: boolean;
  type: 'builtin' | 'stdio' | 'sse';
  cmd?: string;
  args?: string[];
  uri?: string;
  envs?: { [key: string]: string };
  env_keys?: Array<string>;
  timeout?: number;
  allow_configure?: boolean;
};

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
    // Cast the imported JSON data to the expected type
    const bundledExtensions = bundledExtensionsData as BundledExtension[];

    // Process each bundled extension
    for (const bundledExt of bundledExtensions) {
      // Find if this extension already exists
      const existingExt = existingExtensions.find((ext) => nameToKey(ext.name) === bundledExt.id);

      // Skip if extension exists and is already marked as bundled
      if (existingExt && 'bundled' in existingExt && existingExt.bundled) continue;

      // Create the config for this extension
      let extConfig: ExtensionConfig;
      switch (bundledExt.type) {
        case 'builtin':
          extConfig = {
            name: bundledExt.name,
            display_name: bundledExt.display_name,
            type: bundledExt.type,
            timeout: bundledExt.timeout ?? 300,
            bundled: true,
          };
          break;
        case 'stdio':
          extConfig = {
            name: bundledExt.name,
            description: bundledExt.description,
            type: bundledExt.type,
            timeout: bundledExt.timeout,
            cmd: bundledExt.cmd || '',
            args: bundledExt.args || [],
            envs: bundledExt.envs,
            env_keys: bundledExt.env_keys || [],
            bundled: true,
          };
          break;
        case 'sse':
          extConfig = {
            name: bundledExt.name,
            description: bundledExt.description,
            type: bundledExt.type,
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
