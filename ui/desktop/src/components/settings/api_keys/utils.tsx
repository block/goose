import { Provider, ProviderResponse } from './types';
import { getApiUrl, getSecretKey } from '../../../config';
import { default_key_value, required_keys } from '../models/hardcoded_stuff'; // e.g. { OPENAI_HOST: '', OLLAMA_HOST: '' }

export function isSecretKey(keyName: string): boolean {
  // Endpoints and hosts should not be stored as secrets
  const nonSecretKeys = [
    'DATABRICKS_HOST',
    'OLLAMA_HOST',
    'OPENAI_HOST',
    'OPENAI_BASE_PATH',
    'AZURE_OPENAI_ENDPOINT',
    'AZURE_OPENAI_DEPLOYMENT_NAME',
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
        if (!required_keys[providerName]) return false;

        // Get all required keys for this provider
        const providerRequiredKeys = required_keys[providerName];

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
          (key) => !(key in default_key_value)
        );

        // If there are no non-default keys, this provider needs at least one key explicitly set
        if (requiredNonDefaultKeys.length === 0) {
          return providerRequiredKeys.some((key) => configStatus[key]?.is_set === true);
        }

        // Otherwise, all non-default keys must be set
        return requiredNonDefaultKeys.every((key) => configStatus[key]?.is_set === true);
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
  const providerList = await getProvidersList();
  // Extract the list of IDs
  const providerIds = providerList.map((provider) => provider.id);

  // Fetch configs state (set/unset) using the provider IDs
  const response = await fetch(getApiUrl('/configs/providers'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({
      providers: providerIds,
    }),
  });

  if (!response.ok) {
    throw new Error('Failed to fetch secrets');
  }

  const data = (await response.json()) as Record<string, ProviderResponse>;
  return data;
}

export async function getProvidersList(): Promise<Provider[]> {
  const response = await fetch(getApiUrl('/agent/providers'), {
    method: 'GET',
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch providers: ${response.statusText}`);
  }

  const data = await response.json();

  // Format the response into an array of providers
  return data.map((item: any) => ({
    id: item.id, // Root-level ID
    name: item.details?.name || 'Unknown Provider', // Nested name in details
    description: item.details?.description || 'No description available.', // Nested description
    models: item.details?.models || [], // Nested models array
    requiredKeys: item.details?.required_keys || [], // Nested required keys array
  }));
}
