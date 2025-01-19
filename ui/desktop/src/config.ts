// Helper to construct API endpoints
export const getApiUrl = (endpoint: string): string => {  
  const baseUrl = window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_PORT');
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};

export const getSecretKey = (): string => {
  return window.appConfig.get('secretKey');
}

// Environment variables type matching Rust's Envs
export type Envs = Record<string, string>;

// SystemConfig type matching the Rust version
export type SystemConfig =
  | {
      type: "sse";
      uri: string;
      envs?: Envs;
    }
  | {
      type: "stdio";
      cmd: string;
      args: string[];
      envs?: Envs;
    }
  | {
      type: "builtin";
      name: string;
    };

// Extend Goosed with a new system configuration
export const extendGoosed = async (config: SystemConfig) => {
  // allowlist the CMD for stdio type
  if (config.type === "stdio") {
    const allowedCMDs = ['npx', 'uvx', 'goosed'];
    if (!allowedCMDs.includes(config.cmd)) {
      console.error(`System ${config.cmd} is not supported right now`);
      return;
    }

    // if its goosed - we will update the path to the binary
    if (config.cmd === 'goosed') {
      config = {
        ...config,
        cmd: await window.electron.getBinaryPath('goosed')
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

    if (!response.ok) {
      throw new Error(`Failed to add system config: ${response.statusText}`);
    }
    console.log(`Successfully added system config: ${JSON.stringify(config)}`);
  } catch (error) {
    console.log(`Error adding system config:`, error);
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
    envs: Object.keys(envs).length > 0 ? envs : undefined
  };

  await extendGoosed(config);
};
