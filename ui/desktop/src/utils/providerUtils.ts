import { extendGoosed, extendGoosedFromUrl, getApiUrl, getSecretKey } from "../config";

export const SELECTED_PROVIDER_KEY = "GOOSE_PROVIDER__API_KEY"

export interface ProviderOption {
  id: string;
  name: string;
  description: string;
  models: string;
}

export const OPENAI_ENDPOINT_PLACEHOLDER = "https://api.openai.com";
export const ANTHROPIC_ENDPOINT_PLACEHOLDER = "https://api.anthropic.com";
export const OPENAI_DEFAULT_MODEL = "gpt-4"
export const ANTHROPIC_DEFAULT_MODEL = "claude-3-sonnet"

export function getStoredProvider(config: any): string | null {
  console.log("config goose provider", config.GOOSE_PROVIDER)
  console.log("local storage goose provider", localStorage.getItem("GOOSE_PROVIDER"))
  return config.GOOSE_PROVIDER || localStorage.getItem("GOOSE_PROVIDER");
}

export interface Provider {
  id: string; // Lowercase key (e.g., "openai")
  name: string; // Provider name (e.g., "OpenAI")
  description: string; // Description of the provider
  models: string[]; // List of supported models
  requiredKeys: string[]; // List of required keys
}

export async function getProvidersList(): Promise<Provider[]> {
  const response = await fetch(getApiUrl("/agent/providers"), {
    method: "GET",
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch providers: ${response.statusText}`);
  }

  const data = await response.json();
  console.log("Raw API Response:", data); // Log the raw response

  // Format the response into an array of providers
  return data.map((item: any) => ({
    id: item.id, // Root-level ID
    name: item.details?.name || "Unknown Provider", // Nested name in details
    description: item.details?.description || "No description available.", // Nested description
    models: item.details?.models || [], // Nested models array
    requiredKeys: item.details?.required_keys || [], // Nested required keys array
  }));
}

const addAgent = async (provider: string) => {
  const response = await fetch(getApiUrl("/agent"), {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Secret-Key": getSecretKey(),
    },
    body: JSON.stringify({ provider: provider }),
  });

  if (!response.ok) {
    throw new Error(`Failed to add agent: ${response.statusText}`);
  }

  return response;
};

export const initializeSystem = async (provider: string) => {
  try {
    console.log("initializing with provider", provider)
    await addAgent(provider);
    await extendGoosed({
      type: "builtin",
      name: "developer"
    });

    // Handle deep link if present
    const deepLink = window.appConfig.get('DEEP_LINK');
    if (deepLink) {
      await extendGoosedFromUrl(deepLink);
    }
  } catch (error) {
    console.error("Failed to initialize system:", error);
    throw error;
  }
};
