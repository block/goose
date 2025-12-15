import { getApiUrl } from '../config';

export interface ModelCostInfo {
  input_token_cost: number;
  output_token_cost: number;
  currency: string;
}

/**
 * Fetch pricing for a specific provider/model from the backend
 */
export async function fetchModelPricing(
  provider: string,
  model: string
): Promise<ModelCostInfo | null> {
  // For local/free providers, return zero cost immediately
  const freeProviders = ['ollama', 'local', 'localhost'];
  if (freeProviders.includes(provider.toLowerCase())) {
    return {
      input_token_cost: 0,
      output_token_cost: 0,
      currency: '$',
    };
  }

  try {
    const apiUrl = getApiUrl('/config/pricing');
    const secretKey = await window.electron.getSecretKey();

    const headers: HeadersInit = { 'Content-Type': 'application/json' };
    if (secretKey) {
      headers['X-Secret-Key'] = secretKey;
    }

    const response = await fetch(apiUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify({ provider, model }),
    });

    if (!response.ok) {
      return null;
    }

    const data = await response.json();
    const pricing = data.pricing?.[0];

    if (pricing) {
      return {
        input_token_cost: pricing.input_token_cost,
        output_token_cost: pricing.output_token_cost,
        currency: pricing.currency || '$',
      };
    }

    return null;
  } catch {
    return null;
  }
}
