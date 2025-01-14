
// Helper to construct API endpoints
export const getApiUrl = (endpoint: string): string => {  
  const baseUrl = window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_SERVER__PORT');
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};

export const getSecretKey = (): string => {
  return window.appConfig.get('secretKey');
}

// add a MCP system
export const addMCPSystem = async (system: string, args: [string], env: [{ string: string }]) => {

  console.log("calling add system for MCP")

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