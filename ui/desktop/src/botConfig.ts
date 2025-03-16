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
      console.error('No config parameter found in bot URL');
      return null;
    }

    // Decode the base64 config parameter
    const decodedConfig = window.atob(configParam);
    const config: BotConfig = JSON.parse(decodedConfig);

    // Validate required fields
    if (!config.id || !config.name || !config.instructions) {
      console.error('Invalid bot configuration: missing required fields');
      return null;
    }

    return config;
  } catch (error) {
    console.error('Failed to parse bot configuration:', error);
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
    const response = await fetch(getApiUrl('/agent/prompt'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({ prompt: instructions }),
    });

    return response.ok;
  } catch (error) {
    console.error('Error setting bot system prompt:', error);
    return false;
  }
}
