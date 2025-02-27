import { Provider, ProviderResponse } from './types';
import { getApiUrl, getSecretKey } from '../../../config';
import { default_key_value } from '../models/hardcoded_stuff'; // e.g. { OPENAI_HOST: '', OLLAMA_HOST: '' }

export function isSecretKey(keyName: string): boolean {
  // Endpoints and hosts should not be stored as secrets
  const nonSecretKeys = [
    'DATABRICKS_HOST',
    'OLLAMA_HOST',
    'OPENAI_HOST',
    'OPENAI_BASE_PATH',
    'AZURE_OPENAI_ENDPOINT',
    'AZURE_OPENAI_DEPLOYMENT_NAME',
  ];
  return !nonSecretKeys.includes(keyName);
}

// A small helper: returns true if key is *not* in default_key_value
function isRequiredKey(key: string): boolean {
  return !Object.prototype.hasOwnProperty.call(default_key_value, key);
}

export async function getActiveProviders(): Promise<string[]> {
  try {
    const configSettings = await getConfigSettings();

    const activeProviders = Object.values(configSettings)
      .filter((provider) => {
        // 1. Get provider's config_status
        const configStatus = provider.config_status ?? {};

        // 2. Collect only the keys *not* in default_key_value
        const requiredKeyEntries = Object.entries(configStatus).filter(([k]) => isRequiredKey(k));

        // 3. If there are *no* non-default keys, it is NOT active
        if (requiredKeyEntries.length === 0) {
          return false;
        }

        // 4. Otherwise, all non-default keys must be `is_set`
        return requiredKeyEntries.every(([_, value]) => value?.is_set);
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
