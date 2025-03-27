import React from 'react';
import { getApiUrl, getSecretKey } from './config';
import { type View } from './App';
import { type SettingsViewOptions } from './components/settings/SettingsView';
import { toast } from 'react-toastify';

import builtInExtensionsData from './built-in-extensions.json';
import { toastError, toastLoading, toastSuccess } from './toasts';
import { Toast } from 'react-toastify/dist/components';

// Hardcoded default extension timeout in seconds
export const DEFAULT_EXTENSION_TIMEOUT = 300;

// ExtensionConfig type matching the Rust version
// TODO: refactor this
export type ExtensionConfig =
  | {
      type: 'sse';
      name: string;
      uri: string;
      env_keys?: string[];
      timeout?: number;
    }
  | {
      type: 'stdio';
      name: string;
      cmd: string;
      args: string[];
      env_keys?: string[];
      timeout?: number;
    }
  | {
      type: 'builtin';
      name: string;
      env_keys?: string[];
      timeout?: number;
    };

// FullExtensionConfig type matching all the fields that come in deep links and are stored in local storage
export type FullExtensionConfig = ExtensionConfig & {
  id: string;
  description: string;
  enabled: boolean;
};

export interface ExtensionPayload {
  name?: string;
  type?: string;
  cmd?: string;
  args?: string[];
  uri?: string;
  env_keys?: string[];
  timeout?: number;
}

export const BUILT_IN_EXTENSIONS = builtInExtensionsData as FullExtensionConfig[];

function sanitizeName(name: string) {
  return name.toLowerCase().replace(/-/g, '').replace(/_/g, '').replace(/\s/g, '');
}

export async function addExtension(
  extension: FullExtensionConfig,
  silent: boolean = false
): Promise<Response> {
  try {
    // Create the config based on the extension type
    const config = {
      type: extension.type,
      ...(extension.type === 'stdio' && {
        name: sanitizeName(extension.name),
        cmd: await replaceWithShims(extension.cmd),
        args: extension.args || [],
      }),
      ...(extension.type === 'sse' && {
        name: sanitizeName(extension.name),
        uri: extension.uri,
      }),
      ...(extension.type === 'builtin' && {
        name: sanitizeName(extension.name),
      }),
      env_keys: extension.env_keys,
      timeout: extension.timeout,
    };

    let toastId;
    if (!silent) toastId = toastLoading({ title: extension.name, msg: 'Adding extension...' });

    const response = await fetch(getApiUrl('/extensions/add'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify(config),
    });

    const data = await response.json();

    if (!data.error) {
      if (!silent) {
        if (toastId) toast.dismiss(toastId);
        toastSuccess({ title: extension.name, msg: `Successfully enabled extension` });
      }
      return response;
    }

    var errorMessage = `Error adding extension`;
    // Attempt to extract the message from inside StdioProcessError()
    // NOTE: this may change if the error response from /extensions/add changes
    const regex = /StdioProcessError\("(.*?)"\)/;
    const match = data.message.match(regex);

    if (match) {
      const extracted = match[1];
      // only display the message if it is less than 100 chars
      errorMessage = extracted.length > 100 ? errorMessage : extracted;
    }

    if (toastId) toast.dismiss(toastId);
    toastError({
      title: extension.name,
      msg: errorMessage,
      traceback: data.message,
      toastOptions: { autoClose: false },
    });

    return response;
  } catch (error) {
    const errorMessage = `Failed to add ${extension.name} extension: ${error instanceof Error ? error.message : 'Unknown error'}`;
    console.error(errorMessage);
    toastError({
      title: extension.name,
      msg: 'Failed to add extension',
      traceback: error.message,
      toastOptions: { autoClose: false },
    });
    throw error;
  }
}

export async function removeExtension(name: string, silent: boolean = false): Promise<Response> {
  try {
    const response = await fetch(getApiUrl('/extensions/remove'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify(sanitizeName(name)),
    });

    const data = await response.json();

    if (!data.error) {
      if (!silent) {
        toastSuccess({ title: name, msg: 'Successfully disabled extension' });
      }
      return response;
    }

    const errorMessage = `Error removing ${name} extension${data.message ? `. ${data.message}` : ''}`;
    console.error(errorMessage);
    toastError({
      title: name,
      msg: 'Error removing extension',
      traceback: data.message,
      toastOptions: { autoClose: false },
    });
    return response;
  } catch (error) {
    const errorMessage = `Failed to remove ${name} extension: ${error instanceof Error ? error.message : 'Unknown error'}`;
    console.error(errorMessage);
    toastError({
      title: name,
      msg: 'Error removing extension',
      traceback: error.message,
      toastOptions: { autoClose: false },
    });
    throw error;
  }
}

// Store extension config in user_settings
function storeExtensionConfig(config: FullExtensionConfig) {
  try {
    const userSettingsStr = localStorage.getItem('user_settings');
    const userSettings = userSettingsStr
      ? JSON.parse(userSettingsStr)
      : { models: [], extensions: [] };

    // Check if config already exists (based on cmd for stdio, uri for sse, name for builtin)
    const extensionExists = userSettings.extensions.some(
      (extension: { id: string }) => extension.id === config.id
    );

    if (!extensionExists) {
      userSettings.extensions.push(config);
      localStorage.setItem('user_settings', JSON.stringify(userSettings));
      console.log('Extension config stored successfully in user_settings');
      // Notify settings update through electron IPC
      window.electron.emit('settings-updated');
    } else {
      console.log('Extension config already exists in user_settings');
    }
  } catch (error) {
    console.error('Error storing extension config:', error);
  }
}

export async function loadAndAddStoredExtensions() {
  try {
    const userSettingsStr = localStorage.getItem('user_settings');

    if (userSettingsStr) {
      const userSettings = JSON.parse(userSettingsStr);
      const enabledExtensions = userSettings.extensions.filter((ext: any) => ext.enabled);
      console.log('Adding extensions from localStorage: ', enabledExtensions);
      for (const ext of enabledExtensions) {
        await addExtension(ext, true);
      }
    } else {
      console.log('Saving default builtin extensions to localStorage');
      // TODO - Revisit
      // @ts-expect-error "we actually do always have all the properties required for builtins, but tsc cannot tell for some reason"
      BUILT_IN_EXTENSIONS.forEach(async (extension: FullExtensionConfig) => {
        storeExtensionConfig(extension);
        if (extension.enabled) {
          await addExtension(extension, true);
        }
      });
    }
  } catch (error) {
    console.error('Error loading and activating extensions from localStorage: ', error);
  }
}

// Update the path to the binary based on the command
export async function replaceWithShims(cmd: string) {
  const binaryPathMap: Record<string, string> = {
    goosed: await window.electron.getBinaryPath('goosed'),
    jbang: await window.electron.getBinaryPath('jbang'),
    npx: await window.electron.getBinaryPath('npx'),
    uvx: await window.electron.getBinaryPath('uvx'),
  };

  if (binaryPathMap[cmd]) {
    console.log('--------> Replacing command with shim ------>', cmd, binaryPathMap[cmd]);
    cmd = binaryPathMap[cmd];
  }

  return cmd;
}

function envVarsRequired(config: ExtensionConfig) {
  return config.env_keys?.length > 0;
}

function handleError(message: string, shouldThrow = false): void {
  toastError({
    title: 'Failed to install extension',
    msg: message,
    traceback: message,
    toastOptions: { autoClose: false },
  });
  console.error(message);
  if (shouldThrow) {
    throw new Error(message);
  }
}

export async function addExtensionFromDeepLink(
  url: string,
  setView: (view: View, options: SettingsViewOptions) => void
) {
  if (!url.startsWith('goose://extension')) {
    handleError('Invalid URL: URL must use the goose://extension scheme');
    return;
  }

  const parsedUrl = new URL(url);

  if (parsedUrl.protocol !== 'goose:') {
    handleError('Invalid protocol: URL must use the goose:// scheme', true);
  }

  // Check that all required fields are present and not empty
  const requiredFields = ['name', 'description'];

  for (const field of requiredFields) {
    const value = parsedUrl.searchParams.get(field);
    if (!value || value.trim() === '') {
      handleError(`The link is missing required field '${field}'`, true);
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
    handleError('npx with -c argument can lead to code injection', true);
  }

  const envList = parsedUrl.searchParams.getAll('env');
  const id = parsedUrl.searchParams.get('id');
  const name = parsedUrl.searchParams.get('name');
  const description = parsedUrl.searchParams.get('description');
  const timeout = parsedUrl.searchParams.get('timeout');

  // split env based on delimiter to a map
  const envs = envList.reduce(
    (acc, env) => {
      const [key, value] = env.split('=');
      acc[key] = value;
      return acc;
    },
    {} as Record<string, string>
  );

  // Create a ExtensionConfig from the URL parameters
  // Parse timeout if provided, otherwise use default
  const parsedTimeout = timeout ? parseInt(timeout, 10) : null;

  const config: FullExtensionConfig = {
    id,
    name,
    type: 'stdio',
    cmd,
    args,
    description,
    enabled: true,
    env_keys: Object.keys(envs).length > 0 ? Object.keys(envs) : [],
    timeout:
      parsedTimeout !== null && !isNaN(parsedTimeout) && Number.isInteger(parsedTimeout)
        ? parsedTimeout
        : DEFAULT_EXTENSION_TIMEOUT,
  };

  // Store the extension config regardless of env vars status
  storeExtensionConfig(config);

  // Check if extension requires env vars and go to settings if so
  if (envVarsRequired(config)) {
    console.log('Environment variables required, redirecting to settings');
    setView('settings', { extensionId: config.id, showEnvVars: true });
    return;
  }

  // If no env vars are required, proceed with extending Goosed
  await addExtension(config);
}
