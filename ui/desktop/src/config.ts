// Helper to construct API endpoints
export const getApiUrl = (endpoint: string): string => {  
  const baseUrl = window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_PORT');
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};

export const getSecretKey = (): string => {
  return window.appConfig.get('secretKey');
}

// SystemConfig type matching the Rust version
export type SystemConfig =
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
    };

// Extend Goosed with a new system configuration
export const extendGoosed = async (config: SystemConfig) => {
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
  } catch (error) {
    console.error(`Error adding system config:`, error.message || error);
  }
};

// Extend Goosed from a goose://extension URL
export const extendGoosedFromUrl = async (url: string) => {
  console.log("extending Goosed from URL", url);
  if (!url.startsWith("goose://extension")) {
    console.log("Invalid URL: URL must use the goose://extension scheme");
  }

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

  // split env based on delimiter to a map
  const envs = envList.reduce((acc, env) => {
    const [key, value] = env.split("=");
    acc[key] = value;
    return acc;
  }, {} as Record<string, string>);

  // Create a SystemConfig from the URL parameters
  const config: SystemConfig = {
    type: "stdio",
    cmd,
    args,
    env_keys: Object.keys(envs).length > 0 ? Object.keys(envs) : undefined
  };

  await extendGoosed(config);
};
