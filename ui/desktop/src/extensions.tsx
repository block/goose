import { getApiUrl } from './config';
import { toast } from 'react-toastify';
import { safeJsonParse } from './utils/jsonUtils';

import builtInExtensionsData from './built-in-extensions.json';
import { toastError, toastLoading, toastSuccess } from './toasts';

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
      type: 'streamable_http';
      name: string;
      uri: string;
      env_keys?: string[];
      headers?: Record<string, string>;
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
    console.log('Adding extension:', extension);
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
      ...(extension.type === 'streamable_http' && {
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
        'X-Secret-Key': await window.electron.getSecretKey(),
      },
      body: JSON.stringify(config),
    });

    const responseText = await response.text();

    if (!response.ok) {
      const errorMsg = `Server returned ${response.status}: ${response.statusText}. Response: ${responseText}`;
      console.error(errorMsg);
      if (toastId) toast.dismiss(toastId);
      toastError({
        title: extension.name,
        msg: 'Failed to add extension',
        traceback: errorMsg,
        toastOptions: { autoClose: false },
      });
      return response;
    }

    // Only try to parse JSON if we got a successful response and have JSON content
    let data;
    try {
      data = JSON.parse(responseText);
    } catch (e) {
      console.error('Failed to parse response as JSON:', e);
      data = { error: true, message: responseText };
    }

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
      traceback: error instanceof Error ? error.message : String(error),
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
        'X-Secret-Key': await window.electron.getSecretKey(),
      },
      body: JSON.stringify(sanitizeName(name)),
    });

    const data = await safeJsonParse<{ error: boolean; message: string }>(response);

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
      traceback: error instanceof Error ? error.message : String(error),
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
      const enabledExtensions = userSettings.extensions.filter(
        (ext: FullExtensionConfig) => ext.enabled
      );
      console.log('Adding extensions from localStorage: ', enabledExtensions);
      for (const ext of enabledExtensions) {
        await addExtension(ext, true);
      }
    } else {
      console.log('Saving default builtin extensions to localStorage');
      // TODO - Revisit
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
    'npx.cmd': await window.electron.getBinaryPath('npx.cmd'),
  };

  if (binaryPathMap[cmd]) {
    console.log('--------> Replacing command with shim ------>', cmd, binaryPathMap[cmd]);
    cmd = binaryPathMap[cmd];
  }

  return cmd;
}
