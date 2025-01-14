
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
// eg: goose://extension?cmd=npx&args=-y,@modelcontextprotocol/server-memory&description=this is my mcp&website=blah.com&environment={“VAR”:”VALUE”}
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

  const argsParam = parsedUrl.searchParams.get("args");
  const args = argsParam ? argsParam.split(",") : [];

  const environmentParam = parsedUrl.searchParams.get("environment");
  console.log("environmentParam", environmentParam);
  const env = environmentParam ? JSON.parse(environmentParam) : {};

  addMCP(system, args as [string], env as [{ string: string }]);
}

// add a MCP system
export const addMCP = async (system: string, args: [string], env: [{ string: string }]) => {

  // allowlist the CMD
  const allowedCMDs = ['npx', 'uvx'];

  if (!allowedCMDs.includes(system)) {
    console.error(`System ${system} is not supported right now`);
    return;
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

// this adds a built in MCP from the goosed binary
export const addBuiltInSystem = async (system: string) => {
  console.log("calling add system for built in")
  
  const systemConfig = {
    type: "Stdio",
    cmd: await window.electron.getBinaryPath('goosed'),
    args: ["mcp", system]
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
      throw new Error(`Failed to add system config for ${system}: ${response.statusText}`);
    }

    console.log(`Successfully added system config for ${system}`);
  } catch (error) {
    console.log(`Error adding system config for ${system}:`, error);
  }
};