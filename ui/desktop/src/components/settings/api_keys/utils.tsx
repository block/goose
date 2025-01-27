import { Provider, ProviderResponse } from './types';
import { getApiUrl, getSecretKey } from '../../../config';
import { special_provider_cases } from '../providers/utils';

export async function getActiveProviders(): Promise<string[]> {
  try {
    // Fetch the secrets settings
    const secretsSettings = await getSecretsSettings();

    // Check for special provider cases (e.g. ollama needs to be installed in Applications folder)
    const specialCasesResults = await Promise.all(
      Object.entries(special_provider_cases).map(async ([providerName, checkFunction]) => {
        const isActive = await checkFunction(); // Dynamically re-check status
        console.log(`Special case result for ${providerName}:`, isActive);
        return isActive ? providerName : null;
      })
    );

    // Extract active providers based on `is_set` in `secret_status` or providers with no keys
    const activeProviders = Object.values(secretsSettings) // Convert object to array
      .filter((provider) => {
        const apiKeyStatus = Object.values(provider.secret_status || {}); // Get all key statuses

        // Include providers if:
        // - They have at least one key set (`is_set: true`)
        return apiKeyStatus.some((key) => key.is_set);
      })
      .map((provider) => provider.name || 'Unknown Provider'); // Extract provider name

    // Combine active providers from secrets settings and special cases
    const allActiveProviders = [
      ...activeProviders,
      ...specialCasesResults.filter((provider) => provider !== null), // Filter out null results
    ];
    return allActiveProviders;
  } catch (error) {
    console.error('Failed to get active providers:', error);
    return [];
  }
}

export async function getSecretsSettings(): Promise<Record<string, ProviderResponse>> {
  const providerList = await getProvidersList();
  // Extract the list of IDs
  const providerIds = providerList.map((provider) => provider.id);

  // Fetch secrets state (set/unset) using the provider IDs
  const response = await fetch(getApiUrl('/secrets/providers'), {
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
