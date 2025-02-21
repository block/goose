import { getApiUrl, getSecretKey } from './config';
import { type View } from './App';
import { toast } from 'react-toastify';

// ExtensionConfig type matching the Rust version
export type ExtensionConfig =
  | {
      type: 'sse';
      name: string;
      uri: string;
      env_keys?: string[];
    }
  | {
      type: 'stdio';
      name: string;
      cmd: string;
      args: string[];
      env_keys?: string[];
    }
  | {
      type: 'builtin';
      name: string;
      env_keys?: string[];
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
}

export const BUILT_IN_EXTENSIONS = [
  {
    id: 'developer',
    name: 'Developer',
    description: 'General development tools useful for software engineering.',
    enabled: true,
    type: 'builtin',
    env_keys: [],
  },
  {
    id: 'computercontroller',
    name: 'Computer Controller',
    description:
      "General computer control tools that don't require you to be a developer or engineer.",
    enabled: false,
    type: 'builtin',
    env_keys: [],
  },
  {
    id: 'memory',
    name: 'Memory',
    description: 'Teach goose your preferences as you go.',
    enabled: false,
    type: 'builtin',
    env_keys: [],
  },
  {
    id: 'jetbrains',
    name: 'Jetbrains',
    description: 'Integration with any Jetbrains IDE',
    enabled: false,
    type: 'builtin',
    env_keys: [],
  },
  /* TODO re-enable when we have a smoother auth flow {
    id: 'google_drive',
    name: 'Google Drive',
    description: 'Built-in Google Drive integration for file management and access',
    enabled: false,
    type: 'builtin',
    env_keys: [
      'GOOGLE_DRIVE_OAUTH_PATH',
      'GOOGLE_DRIVE_CREDENTIALS_PATH',
      'GOOGLE_DRIVE_OAUTH_CONFIG',
    ],
  },*/
];

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
    };

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
        toast.success(`Successfully enabled ${extension.name} extension`);
      }
      return response;
    }

    const errorMessage = `Error adding ${extension.name} extension ${data.message ? `. ${data.message}` : ''}`;
    console.error(errorMessage);
    toast.error(errorMessage);
    return response;
  } catch (error) {
    const errorMessage = `Failed to add ${extension.name} extension: ${error instanceof Error ? error.message : 'Unknown error'}`;
    console.error(errorMessage);
    toast.error(errorMessage);
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
        toast.success(`Successfully disabled ${name} extension`);
      }
      return response;
    }

    const errorMessage = `Error removing ${name} extension${data.message ? `. ${data.message}` : ''}`;
    console.error(errorMessage);
    toast.error(errorMessage);
    return response;
  } catch (error) {
    const errorMessage = `Failed to remove ${name} extension: ${error instanceof Error ? error.message : 'Unknown error'}`;
    console.error(errorMessage);
    toast.error(errorMessage);
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
      window.electron.send('settings-updated');
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
  toast.error(message);
  console.error(message);
  if (shouldThrow) {
    throw new Error(message);
  }
}

export async function addExtensionFromDeepLink(url: string, setView: (view: View) => void) {
  if (!url.startsWith('goose://extension')) {
    handleError(
      'Failed to install extension: Invalid URL: URL must use the goose://extension scheme'
    );
    return;
  }

  const parsedUrl = new URL(url);

  if (parsedUrl.protocol !== 'goose:') {
    handleError(
      'Failed to install extension: Invalid protocol: URL must use the goose:// scheme',
      true
    );
  }

  // Check that all required fields are present and not empty
  const requiredFields = ['name', 'description'];

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
  const id = parsedUrl.searchParams.get('id');
  const name = parsedUrl.searchParams.get('name');
  const description = parsedUrl.searchParams.get('description');

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
  const config: FullExtensionConfig = {
    id,
    name,
    type: 'stdio',
    cmd,
    args,
    description,
    enabled: true,
    env_keys: Object.keys(envs).length > 0 ? Object.keys(envs) : [],
  };

  // Store the extension config regardless of env vars status
  storeExtensionConfig(config);

  // Check if extension requires env vars and go to settings if so
  if (envVarsRequired(config)) {
    console.log('Environment variables required, redirecting to settings');
    setView('settings');
    // TODO - add code which can auto-open the modal on the settings view
    // navigate(`/settings?extensionId=${config.id}&showEnvVars=true`);
    return;
  }

  // If no env vars are required, proceed with extending Goosed
  await addExtension(config);
}
