import { getApiUrl, getSecretKey } from './config';
import { toast } from 'react-toastify';

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
      traceback: error instanceof Error ? error.message : String(error),
      toastOptions: { autoClose: false },
    });
    throw error;
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
