import { getApiUrl, getSecretKey } from './config';

export interface BotConfig {
  id: string;
  name: string;
  description: string;
  instructions: string;
  activities: string[] | null;
  outputExample?: string;
}

/**
 * Parses a bot configuration from a deep link URL
 * @param url The deep link URL (goose://bot?config=<base64-encoded-json>)
 * @returns The parsed bot configuration or null if invalid
 */
export function parseBotConfigFromUrl(url: string): BotConfig | null {
  try {
    const parsedUrl = new URL(url);
    const configParam = parsedUrl.searchParams.get('config');

    if (!configParam) {
      window.electron.logInfo('No config parameter found in bot URL');
      return null;
    }

    // Decode the base64 config parameter
    const decodedConfig = window.atob(configParam);
    window.electron.logInfo('Decoded config: ' + decodedConfig);
    const config: BotConfig = JSON.parse(decodedConfig);
    window.electron.logInfo('Parsed config: ' + JSON.stringify(config));

    // Validate required fields
    if (!config.id || !config.name || !config.instructions) {
      window.electron.logInfo('Invalid bot configuration: missing required fields');
      return null;
    }

    return config;
  } catch (error) {
    window.electron.logInfo('Failed to parse bot configuration: ' + error);
    return null;
  }
}

/**
 * Sets the system prompt for a bot
 * @param instructions The instructions to set as the system prompt
 * @returns A promise that resolves when the prompt is set
 */
export async function setBotSystemPrompt(instructions: string): Promise<boolean> {
  try {
    const apiUrl = getApiUrl('/agent/prompt');
    window.electron.logInfo(`Setting system prompt, API URL: ${apiUrl}`);

    // Use extension parameter just like in providerUtils.ts
    const requestBody = JSON.stringify({ extension: instructions });
    window.electron.logInfo(`Request body: ${requestBody}`);

    const response = await fetch(apiUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: requestBody,
    });

    if (!response.ok) {
      window.electron.logInfo(`Failed to set bot system prompt: ${response.statusText}`);
      return false;
    }

    window.electron.logInfo('System prompt extended successfully');
    return true;
  } catch (error) {
    window.electron.logInfo('Error setting bot system prompt: ' + error);
    return false;
  }
}
