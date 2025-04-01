import type { ExtensionConfig } from '../../../api/types.gen';
import { FixedExtensionEntry } from '../../ConfigContext';
import builtInExtensionsData from './built-in-extensions.json';
import { nameToKey } from './utils';

// Type definition for built-in extensions from JSON
type BuiltinExtension = {
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
export async function syncBuiltInExtensions(
  existingExtensions: FixedExtensionEntry[],
  addExtensionFn: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>
): Promise<void> {
  try {
    console.log('Setting up built-in extensions... in syncBuiltinExtensions');

    // Create a set of existing extension IDs for quick lookup
    const existingExtensionKeys = new Set(existingExtensions.map((ext) => nameToKey(ext.name)));

    // Cast the imported JSON data to the expected type
    const builtinExtensions = builtInExtensionsData as BuiltinExtension[];

    // Track how many extensions were added
    let addedCount = 0;

    // Check each built-in extension
    for (const builtinExt of builtinExtensions) {
      // Only add if the extension doesn't already exist -- use the id
      if (!existingExtensionKeys.has(builtinExt.id)) {
        console.log(`Adding built-in extension: ${builtinExt.id}`);
        let extConfig: ExtensionConfig;
        switch (builtinExt.type) {
          case 'builtin':
            extConfig = {
              name: builtinExt.name,
              display_name: builtinExt.display_name,
              type: builtinExt.type,
              timeout: builtinExt.timeout ?? 300,
            };
            break;
          case 'stdio':
            extConfig = {
              name: builtinExt.name,
              description: builtinExt.description,
              type: builtinExt.type,
              timeout: builtinExt.timeout,
              cmd: builtinExt.cmd,
              args: builtinExt.args,
              envs: builtinExt.envs,
            };
            break;
          case 'sse':
            extConfig = {
              name: builtinExt.name,
              description: builtinExt.description,
              type: builtinExt.type,
              timeout: builtinExt.timeout,
              uri: builtinExt.uri,
            };
        }
        // Add the extension with its default enabled state
        try {
          await addExtensionFn(builtinExt.name, extConfig, builtinExt.enabled);
          addedCount++;
        } catch (error) {
          console.error(`Failed to add built-in extension ${builtinExt.name}:`, error);
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
export async function initializeBuiltInExtensions(
  addExtensionFn: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>
): Promise<void> {
  // Call with an empty list to ensure all built-ins are added
  await syncBuiltInExtensions([], addExtensionFn);
}
