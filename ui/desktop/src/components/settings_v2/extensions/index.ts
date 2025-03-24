import type { ExtensionConfig } from '../../../api/types.gen';
import builtInExtensionsData from './built-in-extensions.json';
import { FixedExtensionEntry } from '../../ConfigContext';
import { toast } from 'react-toastify';
import { getApiUrl, getSecretKey } from '../../../config';

// Default extension timeout in seconds
export const DEFAULT_EXTENSION_TIMEOUT = 300;

// Type definition for built-in extensions from JSON
type BuiltinExtension = {
  id: string;
  name: string;
  display_name: string;
  description: string;
  enabled: boolean;
  type: 'builtin';
  envs?: { [key: string]: string };
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

function handleError(message: string, shouldThrow = false): void {
  toast.error(message, { autoClose: false });
  console.error(message);
  if (shouldThrow) {
    throw new Error(message);
  }
}

export async function addExtensionFromDeepLink(
  url: string,
  addExtensionFn: (name: string, config: ExtensionConfig, enabled: boolean) => Promise<void>,
  setView: (view: string, options: { extensionId: string; showEnvVars: boolean }) => void
) {
  const parsedUrl = new URL(url);

  if (parsedUrl.protocol !== 'goose:') {
    handleError(
      'Failed to install extension: Invalid protocol: URL must use the goose:// scheme',
      true
    );
  }

  // Check that all required fields are present and not empty
  const requiredFields = ['name'];

  for (const field of requiredFields) {
    const value = parsedUrl.searchParams.get(field);
    if (!value || value.trim() === '') {
      handleError(
        `Failed to install extension: The link is missing required field '${field}'`,
        true
      );
    }
  }

  const cmd = parsedUrl.searchParams.get('cmd');
  if (!cmd) {
    handleError("Failed to install extension: Missing required 'cmd' parameter in the URL", true);
  }

  // Validate that the command is one of the allowed commands
  const allowedCommands = ['npx', 'uvx', 'goosed'];
  if (!allowedCommands.includes(cmd)) {
    handleError(
      `Failed to install extension: Invalid command: ${cmd}. Only ${allowedCommands.join(', ')} are allowed.`,
      true
    );
  }

  // Check for security risk with npx -c command
  const args = parsedUrl.searchParams.getAll('arg');
  if (cmd === 'npx' && args.includes('-c')) {
    handleError(
      'Failed to install extension: npx with -c argument can lead to code injection',
      true
    );
  }

  const envList = parsedUrl.searchParams.getAll('env');
  const name = parsedUrl.searchParams.get('name')!;
  const timeout = parsedUrl.searchParams.get('timeout');

  // Create the extension config
  const config: ExtensionConfig = {
    name: name,
    type: 'stdio',
    cmd: cmd,
    args: args,
    envs:
      envList.length > 0
        ? Object.fromEntries(
            envList.map((env) => {
              const [key] = env.split('=');
              return [key, '']; // Initialize with empty string as value
            })
          )
        : undefined,
    timeout: timeout ? parseInt(timeout, 10) : DEFAULT_EXTENSION_TIMEOUT,
  };

  // Check if extension requires env vars and go to settings if so
  if (config.envs && Object.keys(config.envs).length > 0) {
    console.log('Environment variables required, redirecting to settings');
    setView('settings', { extensionId: nameToKey(name), showEnvVars: true });
    return;
  }

  // If no env vars are required, proceed with adding the extension
  try {
    // First add to the config system
    await addExtensionFn(nameToKey(name), config, true);

    // Then call the API endpoint
    const response = await fetch(getApiUrl('/extensions/add'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({
        type: config.type,
        name: nameToKey(name),
        cmd: config.cmd,
        args: config.args || [],
        env_keys: config.envs ? Object.keys(config.envs) : undefined,
        timeout: config.timeout,
      }),
    });

    const data = await response.json();

    if (!data.error) {
      toast.success(`Extension "${name}" has been successfully enabled`);
    } else {
      const errorMessage = `Error adding ${name} extension${data.message ? `. ${data.message}` : ''}`;
      console.error(errorMessage);
      toast.error(errorMessage, { autoClose: false });
    }
  } catch (error) {
    const errorMessage = `Failed to add ${name} extension: ${error instanceof Error ? error.message : 'Unknown error'}`;
    console.error(errorMessage);
    toast.error(errorMessage, { autoClose: false });
    throw error;
  }
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
