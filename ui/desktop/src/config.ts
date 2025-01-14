
// Helper to construct API endpoints
export const getApiUrl = (endpoint: string): string => {  
  const baseUrl = window.appConfig.get('GOOSE_API_HOST') + ':' + window.appConfig.get('GOOSE_SERVER__PORT');
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};

export const getSecretKey = (): string => {
  return window.appConfig.get('secretKey');
}


// Function to send the system configuration to the server
export const addSystemConfig = async (system: string) => {
  console.log("calling add system")
  
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