import type { ExtensionConfig } from '../../../api/types.gen';
import builtInExtensionsData from './built-in-extensions.json';
import { FixedExtensionEntry } from '../../ConfigContext';
import { getApiUrl, getSecretKey } from '../../../config';
import { toast } from 'react-toastify';
import { ToastError, ToastLoading, ToastSuccess } from '../../settings/models/toasts';

// Default extension timeout in seconds
// TODO: keep in sync with rust better
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
  ToastError({
    title: 'Error',
    msg: message,
    traceback: message,
  });
  console.error(message);
  if (shouldThrow) {
    throw new Error(message);
  }
}

// Update the path to the binary based on the command
async function replaceWithShims(cmd: string) {
  const binaryPathMap: Record<string, string> = {
    goosed: await window.electron.getBinaryPath('goosed'),
    npx: await window.electron.getBinaryPath('npx'),
    uvx: await window.electron.getBinaryPath('uvx'),
  };

  if (binaryPathMap[cmd]) {
    console.log('--------> Replacing command with shim ------>', cmd, binaryPathMap[cmd]);
    cmd = binaryPathMap[cmd];
  }

  return cmd;
}

interface activateExtensionProps {
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
}

/**
 * Activates an extension by adding it to both the config system and the API.
 * @param name The extension name
 * @param config The extension configuration
 * @param addExtensionFn Function to add extension to config
 * @returns Promise that resolves when activation is complete
 */
export async function activateExtension({
  addToConfig,
  extensionConfig,
}: activateExtensionProps): Promise<void> {
  try {
    // AddToAgent
    await AddToAgent(extensionConfig);
  } catch (error) {
    console.error('Failed to add extension to agent:', error);
    // add to config with enabled = false
    await addToConfig(extensionConfig.name, extensionConfig, false);
    // Rethrow the error to inform the caller
    throw error;
  }

  // Then add to config
  try {
    await addToConfig(extensionConfig.name, extensionConfig, true);
  } catch (error) {
    console.error('Failed to add extension to config:', error);
    // remove from Agent
    try {
      await RemoveFromAgent(extensionConfig.name);
    } catch (removeError) {
      console.error('Failed to remove extension from agent after config failure:', removeError);
    }
    // Rethrow the error to inform the caller
    throw error;
  }
}

interface addToAgentOnStartupProps {
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
}

export async function addToAgentOnStartup({
  addToConfig,
  extensionConfig,
}: addToAgentOnStartupProps): Promise<void> {
  try {
    // AddToAgent
    await AddToAgent(extensionConfig);
  } catch (error) {
    console.log('got error trying to add to agent in addAgentOnStartUp', error);
    // update config with enabled = false
    try {
      await toggleExtension({ toggle: 'toggleOff', extensionConfig, addToConfig });
    } catch (toggleError) {
      console.error('Failed to toggle extension off after agent error:', toggleError);
    }
    // Rethrow the error to inform the caller
    throw error;
  }
}

interface updateExtensionProps {
  enabled: boolean;
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
  extensionConfig: ExtensionConfig;
}

// updating -- no change to enabled state
export async function updateExtension({
  enabled,
  addToConfig,
  extensionConfig,
}: updateExtensionProps) {
  if (enabled) {
    try {
      // AddToAgent
      await AddToAgent(extensionConfig);
    } catch (error) {
      console.error('Failed to add extension to agent during update:', error);
      // Failed to add to agent -- show that error to user and do not update the config file
      throw error;
    }

    // Then add to config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, enabled);
    } catch (error) {
      console.error('Failed to update extension in config:', error);
      throw error;
    }
  } else {
    try {
      await addToConfig(extensionConfig.name, extensionConfig, enabled);
    } catch (error) {
      console.error('Failed to update disabled extension in config:', error);
      throw error;
    }
  }
}

interface toggleExtensionProps {
  toggle: 'toggleOn' | 'toggleOff';
  extensionConfig: ExtensionConfig;
  addToConfig: (name: string, extensionConfig: ExtensionConfig, enabled: boolean) => Promise<void>;
}

export async function toggleExtension({
  toggle,
  extensionConfig,
  addToConfig,
}: toggleExtensionProps) {
  // disabled to enabled
  if (toggle == 'toggleOn') {
    try {
      // add to agent
      await AddToAgent(extensionConfig);
    } catch (error) {
      console.error('Error adding extension to agent. Will try to toggle back off. Error:', error);
      try {
        await toggleExtension({ toggle: 'toggleOff', extensionConfig, addToConfig });
      } catch (toggleError) {
        console.error('Failed to toggle extension off after agent error:', toggleError);
      }
      throw error;
    }

    // update the config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, true);
    } catch (error) {
      console.error('Failed to update config after enabling extension:', error);
      // remove from agent
      try {
        await RemoveFromAgent(extensionConfig.name);
      } catch (removeError) {
        console.error('Failed to remove extension from agent after config failure:', removeError);
      }
      throw error;
    }
  } else if (toggle == 'toggleOff') {
    // enabled to disabled
    let agentRemoveError = null;
    try {
      await RemoveFromAgent(extensionConfig.name);
    } catch (error) {
      // note there was an error, but attempt to remove from config anyway
      console.error('Error removing extension from agent', extensionConfig.name, error);
      agentRemoveError = error;
    }

    // update the config
    try {
      await addToConfig(extensionConfig.name, extensionConfig, false);
    } catch (error) {
      console.error('Error removing extension from config', extensionConfig.name, 'Error:', error);
      throw error;
    }

    // If we had an error removing from agent but succeeded updating config, still throw the original error
    if (agentRemoveError) {
      throw agentRemoveError;
    }
  }
}

interface deleteExtensionProps {
  name: string;
  removeFromConfig: (name: string) => Promise<void>;
}

export async function deleteExtension({ name, removeFromConfig }: deleteExtensionProps) {
  // remove from agent
  let agentRemoveError = null;
  try {
    await RemoveFromAgent(name);
  } catch (error) {
    console.error('Failed to remove extension from agent during deletion:', error);
    agentRemoveError = error;
  }

  try {
    await removeFromConfig(name);
  } catch (error) {
    console.error(
      'Failed to remove extension from config after removing from agent. Error:',
      error
    );
    // If we also had an agent remove error, log it but throw the config error as it's more critical
    throw error;
  }

  // If we had an error removing from agent but succeeded removing from config, still throw the original error
  if (agentRemoveError) {
    throw agentRemoveError;
  }
}

{
  /*Deeplinks*/
}

export async function addExtensionFromDeepLink(
  url: string,
  addExtensionFn: (
    name: string,
    extensionConfig: ExtensionConfig,
    enabled: boolean
  ) => Promise<void>,
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
  const allowedCommands = ['jbang', 'npx', 'uvx', 'goosed'];
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
    await activateExtension({ extensionConfig: config, addToConfig: addExtensionFn });
  } catch (error) {
    console.error('Failed to activate extension from deeplink:', error);
    throw error;
  }
}

{
  /*Built ins*/
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
        try {
          await addExtensionFn(nameToKey(builtinExt.name), extConfig, builtinExt.enabled);
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

{
  /* Agent-related helper functions */
}
async function extensionApiCall<T>(
  endpoint: string,
  payload: any,
  actionType: 'adding' | 'removing',
  extensionName: string
): Promise<Response> {
  let toastId;
  const actionVerb = actionType === 'adding' ? 'Adding' : 'Removing';
  const pastVerb = actionType === 'adding' ? 'added' : 'removed';

  try {
    if (actionType === 'adding') {
      // Show loading toast
      toastId = ToastLoading({
        title: extensionName,
        msg: `${actionVerb} ${extensionName} extension...`,
      });
      // FIXME: this also shows when toggling -- should only show when you have modal up (fix: diff message for toggling)
      toast.info(
        'Press the ESC key on your keyboard to continue using goose while extension loads'
      );
    }

    const response = await fetch(getApiUrl(endpoint), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify(payload),
    });

    // Handle non-OK responses
    if (!response.ok) {
      const errorMsg = `Server returned ${response.status}: ${response.statusText}`;
      console.error(errorMsg);

      // Special handling for 428 Precondition Required (agent not initialized)
      if (response.status === 428 && actionType === 'adding') {
        if (toastId) toast.dismiss(toastId);
        ToastError({
          title: extensionName,
          msg: 'Agent is not initialized. Please initialize the agent first.',
          traceback: errorMsg,
        });
        throw new Error('Agent is not initialized. Please initialize the agent first.');
      }

      const msg = `Failed to ${actionType === 'adding' ? 'add' : 'remove'} ${extensionName} extension: ${errorMsg}`;
      console.error(msg);

      if (toastId) toast.dismiss(toastId);
      ToastError({
        title: extensionName,
        msg: msg,
        traceback: errorMsg,
      });
      throw new Error(msg);
    }

    // Parse response JSON safely
    let data;
    try {
      const text = await response.text();
      data = text ? JSON.parse(text) : { error: false };
    } catch (parseError) {
      console.warn('Could not parse response as JSON, assuming success', parseError);
      data = { error: false };
    }

    if (!data.error) {
      if (toastId) toast.dismiss(toastId);
      ToastSuccess({ title: extensionName, msg: `Successfully ${pastVerb} extension` });
      return response;
    } else {
      const errorMessage = `Error ${actionType} extension -- parsing data: ${data.message || 'Unknown error'}`;
      console.error(errorMessage);
      if (toastId) toast.dismiss(toastId);
      ToastError({
        title: extensionName,
        msg: errorMessage,
        traceback: data.message || 'Unknown error', // why data.message not data.error?
      });
      throw new Error(errorMessage);
    }
  } catch (error) {
    if (toastId) toast.dismiss(toastId);
    console.error(`Error in extensionApiCall for ${extensionName}:`, error);
    throw error;
  }
}

// Public functions
export async function AddToAgent(extension: ExtensionConfig): Promise<Response> {
  try {
    if (extension.type === 'stdio') {
      console.log('extension command', extension.cmd);
      extension.cmd = await replaceWithShims(extension.cmd);
      console.log('next ext command', extension.cmd);
    }

    return await extensionApiCall('/extensions/add', extension, 'adding', extension.name);
  } catch (error) {
    console.error(`Failed to add extension ${extension.name} to agent:`, error);
    throw error;
  }
}

export async function RemoveFromAgent(name: string): Promise<Response> {
  try {
    return await extensionApiCall('/extensions/remove', name, 'removing', name);
  } catch (error) {
    console.error(`Failed to remove extension ${name} from agent:`, error);
    throw error;
  }
}
