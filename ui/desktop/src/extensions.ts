import { getApiUrl, getSecretKey } from "./config";

// ExtensionConfig type matching the Rust version
export type ExtensionConfig =
  | {
      type: "sse";
      uri: string;
      env_keys?: string[];
    }
  | {
      type: "stdio";
      cmd: string;
      args: string[];
      env_keys?: string[];
    }
  | {
      type: "builtin";
      name: string;
      env_keys?: string[];
    };

// FullExtensionConfig type matching all the fields that come in deep links and are stored in local storage
export type FullExtensionConfig = ExtensionConfig & {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
}

// Store extension config in user_settings
const storeExtensionConfig = (config: FullExtensionConfig) => {
  try {
    const userSettingsStr = localStorage.getItem('user_settings');
    const userSettings = userSettingsStr ? JSON.parse(userSettingsStr) : { models: [], extensions: [] };

    // Check if config already exists (based on cmd for stdio, uri for sse, name for builtin)
    const extensionExists = userSettings.extensions.some(
      (extension: { id: string }) => extension.id === config.id
    );

    if (!extensionExists) {
      userSettings.extensions.push(config);
      localStorage.setItem('user_settings', JSON.stringify(userSettings));
      console.log('Extension config stored successfully in user_settings');
    } else {
      console.log('Extension config already exists in user_settings');
    }
  } catch (error) {
    console.error('Error storing extension config:', error);
  }
};

// Load stored extension configs from user_settings
export const loadStoredExtensionConfigs = async (): Promise<void> => {
  try {
    const userSettingsStr = localStorage.getItem('user_settings');

    if (userSettingsStr) {
      const userSettings = JSON.parse(userSettingsStr);
      const enabledExtensions = userSettings.extensions.filter((ext: any) => ext.enabled);

      for (const ext of enabledExtensions) {
        // Convert extension format back to ExtensionConfig
        const config: ExtensionConfig = {
          type: 'stdio', // Assuming all stored extensions are stdio type for now
          cmd: ext.command,
          args: [],
          env_keys: ext.environmentVariables?.map((env: any) => env.name) || []
        };

        await extendGoosed(config);
      }

      console.log('Loaded stored extension configs from user_settings and activated extensions with agent');
    }
  } catch (error) {
    console.error('Error loading stored extension configs:', error);
  }
};

// Extend Goosed with a new system configuration
export const extendGoosed = async (config: ExtensionConfig) => {
  // allowlist the CMD for stdio type
  if (config.type === "stdio") {
    const allowedCMDs = ['goosed', 'npx', 'uvx'];
    if (!allowedCMDs.includes(config.cmd)) {
      console.error(`System ${config.cmd} is not supported right now`);
      return;
    }
    
    // Update the path to the binary based on the command
    const binaryPathMap: Record<string, string> = {
      'goosed': await window.electron.getBinaryPath('goosed'),
      'npx': await window.electron.getBinaryPath('npx'),
      'uvx': await window.electron.getBinaryPath('uvx'),
    };

    if (binaryPathMap[config.cmd]) {
      config = {
        ...config,
        cmd: binaryPathMap[config.cmd]
      };
    } 
  }

  try {
    const response = await fetch(getApiUrl('/systems/add'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify(config)
    });
    const data = await response.json();

    if (response.ok && !data.error) {
      console.log(`Successfully added system config: ${JSON.stringify(config)}`);
    } else {
      throw new Error(data.message || `Failed to add system config: ${response.statusText}`);
    }
    console.log(`Successfully added system config: ${JSON.stringify(config)}`);
    return true;
  } catch (error) {
    console.log(`Error adding system config:`, error);
    return false;
  }
};

// Check if extension requires env vars
const envVarsRequired = (config: ExtensionConfig): boolean => {
  return config.env_keys?.length > 0;
};

// Navigate to settings with extension ID
const navigateToSettings = (extensionId: string) => {
  // Using window.location.href since we're in a non-React context
  window.location.href = `/settings?extensionId=${extensionId}&showEnvVars=true`;
};

// Extend Goosed from a goose://extension URL
export const extendGoosedFromUrl = async (url: string) => {
  if (!url.startsWith("goose://extension")) {
    console.log("Invalid URL: URL must use the goose://extension scheme");
    return;
  }

  console.log('extendGoosedFromUrl', url);

  const parsedUrl = new URL(url);

  if (parsedUrl.protocol !== "goose:") {
    throw new Error("Invalid protocol: URL must use the goose:// scheme");
  }

  const cmd = parsedUrl.searchParams.get("cmd");

  if (!cmd) {
    throw new Error("Missing required 'cmd' parameter in the URL");
  }

  const args = parsedUrl.searchParams.getAll("arg");
  const envList = parsedUrl.searchParams.getAll("env");
  const id = parsedUrl.searchParams.get("id");
  const name = parsedUrl.searchParams.get("name");
  const description = parsedUrl.searchParams.get("description");

  // split env based on delimiter to a map
  const envs = envList.reduce((acc, env) => {
    const [key, value] = env.split("=");
    acc[key] = value;
    return acc;
  }, {} as Record<string, string>);

  // Create a ExtensionConfig from the URL parameters
  const config: FullExtensionConfig = {
    id,
    name,
    type: "stdio",
    cmd,
    args,
    description,
    enabled: true,
    env_keys: Object.keys(envs).length > 0 ? Object.keys(envs) : undefined
  };

  // Store the extension config regardless of env vars status
  storeExtensionConfig(config);

  // Check if extension requires env vars and go to settings if so
  if (envVarsRequired(config)) {
    console.log('Environment variables required, redirecting to settings');
    navigateToSettings(config.id);
    return;
  }

  // If no env vars are required, proceed with extending Goosed
  const success = await extendGoosed(config);

  if (!success) {
    console.log('Error installing extension from url', url);
  }
};