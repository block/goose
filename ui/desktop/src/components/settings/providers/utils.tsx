import { getConfigSettings } from '../api_keys/utils';

export const special_provider_cases = {
  Ollama: async () => await checkForOllamaApp(), // Dynamically re-check
};

export async function checkForOllamaApp() {
  console.log('Invoking check-ollama IPC handler...');
  try {
    const ollamaInstalled = await window.electron.checkForOllama();
    return ollamaInstalled;
  } catch (error) {
    console.error('Error invoking check-ollama:', error);
    return false;
  }
}

interface ConfigDetails {
  is_set: boolean;
  location?: string;
}

export async function checkOllamaHostIsSet(): Promise<boolean> {
  try {
    const configSettings = await getConfigSettings();
    // Accessing the 'ollama' configuration, then 'config_status', and finally 'OLLAMA_HOST' 'is_set'
    const isSet = configSettings['ollama'].config_status.OLLAMA_HOST.is_set;
    return isSet;
  } catch (error) {
    console.error('Error fetching ollama configuration:', error);
    return false; // or handle the error as appropriate
  }
}

interface OllamaConfigDetails {
  is_set: boolean;
  location?: string;
}

export async function checkOllama(): Promise<OllamaConfigDetails> {
  const setByApp = await checkForOllamaApp();
  const setByHost = await checkOllamaHostIsSet();

  if (setByApp && setByHost) {
    // Priority is given to 'Host' if both are set
    return {
      is_set: true,
      location: 'host',
    };
  } else if (setByApp) {
    // Set by the app only
    return {
      is_set: true,
      location: 'app',
    };
  } else if (setByHost) {
    // Set by the host only
    return {
      is_set: true,
      location: 'host',
    };
  } else {
    // Not set by either
    return {
      is_set: false,
      location: null,
    };
  }
}
