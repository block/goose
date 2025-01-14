
// Helper to construct API endpoints
export const getApiUrl = (endpoint: string): string => {  
  const baseUrl = window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_SERVER__PORT');
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};

export const getSecretKey = (): string => {
  return window.appConfig.get('secretKey');
}


// add MCP system from a goose://extension url 
export const addMCPSystem = async (url: string) => {
  console.log("adding MCP from URL", url);
  if (!url.startsWith("goose://extension")) {
    console.log("Invalid URL: URL must use the goose://extension scheme");
  }

  const parsedUrl = new URL(url);

  if (parsedUrl.protocol !== "goose:") {
    throw new Error("Invalid protocol: URL must use the goose:// scheme");
  }

  const system = parsedUrl.searchParams.get("cmd");
  if (!system) {
    throw new Error("Missing required 'cmd' parameter in the URL");
  }

  const argsParam = parsedUrl.searchParams.getAll("arg");
  const args = argsParam;

  const environmentParam = parsedUrl.searchParams.get("environment");
  console.log("environmentParam", environmentParam);
  const env = environmentParam ? JSON.parse(environmentParam) : {};

  addMCP(system, args, env);
}

// add a MCP system
export const addMCP = async (system: string, args: string[], env?: [{ string: string }]) => {

  // allowlist the CMD
  const allowedCMDs = ['npx', 'uvx', 'goosed'];

  if (!allowedCMDs.includes(system)) {
    console.error(`System ${system} is not supported right now`);
    return;
  }

  if (system === 'goosed') {
    // if its something built in - we will append the path to the binary
    system = await window.electron.getBinaryPath('goosed');
  }

  const systemConfig = {
    type: "Stdio",
    cmd: system,
    args: args,
    env: env
  };

  try {
    const response = await fetch(getApiUrl('/systems/add'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(systemConfig)
    });

    if (!response.ok) {
      throw new Error(`Failed to add system config for ${system} args: ${args} env: ${env}: ${response.statusText}`);
    }
    console.log(`Successfully added MCP config for ${system} args: ${args}`);
  } catch (error) {
    console.log(`Error adding MCP config for ${system} args: ${args} env: ${env}:`, error);
  }

};