import { Provider, ProviderResponse } from './types';
import { getApiUrl, getSecretKey } from '../../../config';
import { special_provider_cases } from '../providers/utils';

export function isSecretKey(keyName: string): boolean {
  // Endpoints and hosts should not be stored as secrets
  const nonSecretKeys = [
    'DATABRICKS_HOST',
    'OLLAMA_HOST',
    'AZURE_OPENAI_ENDPOINT',
    'AZURE_OPENAI_DEPLOYMENT_NAME',
  ];
  return !nonSecretKeys.includes(keyName);
}

export async function getActiveProviders(): Promise<string[]> {
  try {
    // Fetch the secrets settings
    const configSettings = await getConfigSettings();
    console.log('[getActiveProviders]:', configSettings);

    // Check for special provider cases (e.g. ollama running locally)
    const specialCasesResults = await Promise.all(
      Object.entries(special_provider_cases).map(async ([providerName, checkFunction]) => {
        const isActive = await checkFunction(); // Dynamically re-check status
        console.log(`Special case result for ${providerName}:`, isActive);
        return isActive ? providerName : null;
      })
    );
    // Extract active providers based on `is_set` in `secret_status` or providers with no keys
    const activeProviders = Object.values(configSettings) // Convert object to array
      .filter((provider) => {
        const apiKeyStatus = Object.values(provider.config_status || {}); // Get all key statuses

        // Include providers if all required keys are set
        return apiKeyStatus.length > 0 && apiKeyStatus.every((key) => key.is_set);
      })
      .map((provider) => provider.name || 'Unknown Provider'); // Extract provider name

    // Combine active providers from secrets settings and special cases (avoiding repeats)
    const allActiveProviders = activeProviders.concat(
      specialCasesResults.filter(
        (provider) => provider !== null && !activeProviders.includes(provider)
      )
    );

    return allActiveProviders;
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
