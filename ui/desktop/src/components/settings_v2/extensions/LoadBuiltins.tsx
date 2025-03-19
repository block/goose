import type { ExtensionConfig } from '../../../api/types.gen';
import builtInExtensionsData from './built-in-extensions.json';
import { FixedExtensionEntry } from '../../ConfigContext';

// Type definition for built-in extensions from JSON
type BuiltinExtension = {
  id: string;
  name: string;
  display_name: string;
  description: string;
  enabled: boolean;
  type: 'builtin';
  env_keys: string[];
  timeout?: number;
};

// TODO: need to keep this in sync better with `name_to_key` on the rust side
function nameToKey(name: string): string {
  return name
    .split('')
    .filter((char) => !char.match(/\s/))
    .join('')
    .toLowerCase();
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
export async function syncBuiltInExtensions(
  existingExtensions: FixedExtensionEntry[],
  addExtensionFn: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>
): Promise<void> {
  try {
    console.log('Setting up built-in extensions... in syncBuiltinExtensions');

    // Create a set of existing extension IDs for quick lookup
    const existingExtensionKeys = new Set(existingExtensions.map((ext) => nameToKey(ext.name)));
    console.log('existing extension ids', existingExtensionKeys);

    // Cast the imported JSON data to the expected type
    const builtinExtensions = builtInExtensionsData as BuiltinExtension[];

    // Track how many extensions were added
    let addedCount = 0;

    // Check each built-in extension
    for (const builtinExt of builtinExtensions) {
      // Only add if the extension doesn't already exist -- use the id
      if (!existingExtensionKeys.has(builtinExt.id)) {
        console.log(`Adding built-in extension: ${builtinExt.id}`);

        // Convert to the ExtensionConfig format
        const extConfig: ExtensionConfig = {
          name: builtinExt.name,
          display_name: builtinExt.display_name,
          type: 'builtin',
          timeout: builtinExt.timeout ?? 300,
        };

        // Add the extension with its default enabled state
        await addExtensionFn(nameToKey(builtinExt.name), extConfig, builtinExt.enabled);
        addedCount++;
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
