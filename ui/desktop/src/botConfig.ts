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
