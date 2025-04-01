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
    // Create a set of existing extension IDs for quick lookup
    const existingExtensionKeys = new Set(existingExtensions.map((ext) => nameToKey(ext.name)));

    // Cast the imported JSON data to the expected type
    const bundledExtensions = bundledExtensionsData as BundledExtension[];

    // Track how many extensions were added
    let addedCount = 0;

    // Check each built-in extension
    for (const bundledExt of bundledExtensions) {
      // Only add if the extension doesn't already exist -- use the id
      if (!existingExtensionKeys.has(bundledExt.id)) {
        console.log(`Adding built-in extension: ${bundledExt.id}`);
        let extConfig: ExtensionConfig;
        switch (bundledExt.type) {
          case 'builtin':
            extConfig = {
              name: bundledExt.name,
              display_name: bundledExt.display_name,
              type: bundledExt.type,
              timeout: bundledExt.timeout ?? 300,
            };
            break;
          case 'stdio':
            extConfig = {
              name: bundledExt.name,
              description: bundledExt.description,
              type: bundledExt.type,
              timeout: bundledExt.timeout,
              cmd: bundledExt.cmd,
              args: bundledExt.args,
              envs: bundledExt.envs,
            };
            break;
          case 'sse':
            extConfig = {
              name: bundledExt.name,
              description: bundledExt.description,
              type: bundledExt.type,
              timeout: bundledExt.timeout,
              uri: bundledExt.uri,
            };
        }
        // Add the extension with its default enabled state
        try {
          await addExtensionFn(bundledExt.name, extConfig, bundledExt.enabled);
          addedCount++;
        } catch (error) {
          console.error(`Failed to add built-in extension ${bundledExt.name}:`, error);
          // Continue with other extensions even if one fails
        }
      }
    }

    if (addedCount > 0) {
      console.log(`Added ${addedCount} built-in extensions.`);
    } else {
      console.log('All built-in extensions already present.');
    }
  } catch (error) {
    console.error('Failed to add built-in extensions:', error);
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
