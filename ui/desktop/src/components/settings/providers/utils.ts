import { ProviderResponse, Provider, TransformedSecret, RawSecretStatus } from './types'
import { getProvidersList } from '../../../utils/providerUtils'
import { getApiUrl, getSecretKey } from "../../../config";

export async function getSecretsSettings(): Promise<Record<string, ProviderResponse>> {
    const providerList = await getProvidersList();
    // Extract the list of IDs
    const providerIds = providerList.map((provider) => provider.id);

    // Fetch secrets state (set/unset) using the provider IDs
    const response = await fetch(getApiUrl("/secrets/providers"), {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({
            providers: providerIds
        })
    });

    if (!response.ok) {
        throw new Error('Failed to fetch secrets');
    }

    const data = await response.json() as Record<string, ProviderResponse>;
    console.log("raw response", data)
    return data
}

export function transformProviderSecretsResponse(data: Record<string, ProviderResponse>) : Provider[]  {
    // Transform the response into a list of ProviderWithSecrets objects
    const providerOrder = ['openai', 'anthropic', 'databricks']; // maintains these three at top of resulting list
    const transformedProviders: Provider[] = Object.entries(data)
        .map(([id, status]: [string, any]) => ({
            id: id.toLowerCase(),
            name: status.name ? status.name : id,
            keys: status.secret_status ? Object.keys(status.secret_status) : [],
            description: status.description ? status.description : "Unsupported provider",
            supported: status.supported,
            canDelete: id.toLowerCase() !== 'openai' && id.toLowerCase() !== 'anthropic',
            order: providerOrder.indexOf(id.toLowerCase())
        }))
        .sort((a, b) => {
            if (a.order !== -1 && b.order !== -1) {
                return a.order - b.order;
            }
            if (a.order === -1 && b.order === -1) {
                return a.name.localeCompare(b.name);
            }
            return a.order === -1 ? 1 : -1;
        });

    console.log("transformed providers", transformedProviders)
    return transformedProviders
}

export function transformSecrets(data: Record<string, ProviderResponse>): TransformedSecret[] {
    return Object.entries(data)
        .filter(([_, provider]) => provider.supported && provider.secret_status)
        .flatMap(([_, provider]) =>
            Object.entries(provider.secret_status!).map(([key, rawStatus]) => ({
                key,                                 // Secret key (e.g., "OPENAI_API_KEY")
                location: rawStatus.location || "none", // Default location if missing
                is_set: rawStatus.is_set,            // Renamed from `is_set` to `isSet`
            }))
        );
}
