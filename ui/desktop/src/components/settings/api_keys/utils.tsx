import { ProviderResponse, ConfigDetails } from './types';
import { getApiUrl, getSecretKey } from '../../../config';
import { default_key_value, required_keys } from '../models/hardcoded_stuff'; // e.g. { OPENAI_HOST: '', OLLAMA_HOST: '' }

// Backend API response types
interface ProviderMetadata {
  description: string;
  models: string[];
}

interface ProviderDetails {
  name: string;
  metadata: ProviderMetadata;
  is_configured: boolean;
}

export function isSecretKey(keyName: string): boolean {
  // Endpoints and hosts should not be stored as secrets
  const nonSecretKeys = [
    'ANTHROPIC_HOST',
    'DATABRICKS_HOST',
    'OLLAMA_HOST',
    'OPENAI_HOST',
    'OPENAI_BASE_PATH',
    'AZURE_OPENAI_ENDPOINT',
    'AZURE_OPENAI_DEPLOYMENT_NAME',
    'AZURE_OPENAI_API_VERSION',
    'GCP_PROJECT_ID',
    'GCP_LOCATION',
  ];
  return !nonSecretKeys.includes(keyName);
}

export async function getActiveProviders(): Promise<string[]> {
  try {
    const configSettings = await getConfigSettings();
    const activeProviders = Object.values(configSettings)
      .filter((provider) => {
        const providerName = provider.name;
        const configStatus = provider.config_status ?? {};

        // Skip if provider isn't in required_keys
        if (!required_keys[providerName as keyof typeof required_keys]) return false;

        // Get all required keys for this provider
        const providerRequiredKeys = required_keys[providerName as keyof typeof required_keys];

        // Special case: If a provider has exactly one required key and that key
        // has a default value, check if it's explicitly set
        if (providerRequiredKeys.length === 1 && providerRequiredKeys[0] in default_key_value) {
          const key = providerRequiredKeys[0];
          // Only consider active if the key is explicitly set
          return configStatus[key]?.is_set === true;
        }

        // For providers with multiple keys or keys without defaults:
        // Check if all required keys without defaults are set
        const requiredNonDefaultKeys = providerRequiredKeys.filter(
          (key: string) => !(key in default_key_value)
        );

        // If there are no non-default keys, this provider needs at least one key explicitly set
        if (requiredNonDefaultKeys.length === 0) {
          return providerRequiredKeys.some((key: string) => configStatus[key]?.is_set === true);
        }

        // Otherwise, all non-default keys must be set
        return requiredNonDefaultKeys.every((key: string) => configStatus[key]?.is_set === true);
      })
      .map((provider) => provider.name || 'Unknown Provider');

    console.log('[GET ACTIVE PROVIDERS]:', activeProviders);
    return activeProviders;
  } catch (error) {
    console.error('Failed to get active providers:', error);
    return [];
  }
}

export async function getConfigSettings(): Promise<Record<string, ProviderResponse>> {
  // Fetch provider config status
  const response = await fetch(getApiUrl('/config/providers'), {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
  });

  if (!response.ok) {
    throw new Error('Failed to fetch provider configuration status');
  }

  const providers: ProviderDetails[] = await response.json();

  // Convert the response to the expected format
  const data: Record<string, ProviderResponse> = {};
  providers.forEach((provider) => {
    const providerRequiredKeys = required_keys[provider.name as keyof typeof required_keys] || [];

    data[provider.name] = {
      name: provider.name,
      supported: true,
      description: provider.metadata.description,
      models: provider.metadata.models,
      config_status: providerRequiredKeys.reduce<Record<string, ConfigDetails>>(
        (acc: Record<string, ConfigDetails>, key: string) => {
          acc[key] = {
            key,
            is_set: provider.is_configured,
            location: provider.is_configured ? 'config' : undefined,
          };
          return acc;
        },
        {}
      ),
    };
  });

  return data;
}
