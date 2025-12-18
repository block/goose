import { getPricing, PricingData } from '../api';

/**
 * Fetch pricing for a specific provider/model from the backend
 */
export async function fetchModelPricing(
  provider: string,
  model: string
): Promise<PricingData | null> {
  // For local/free providers, return zero cost immediately
  const freeProviders = ['ollama', 'local', 'localhost'];
  if (freeProviders.includes(provider.toLowerCase())) {
    return {
      provider,
      model,
      input_token_cost: 0,
      output_token_cost: 0,
      currency: '$',
      context_length: null,
    };
  }

  try {
    const response = await getPricing({
      body: { provider, model },
      throwOnError: false,
    });

    if (!response.data) {
      return null;
    }

    return response.data.pricing?.[0] ?? null;
  } catch {
    return null;
  }
}
