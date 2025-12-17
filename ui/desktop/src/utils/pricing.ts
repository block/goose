import { getPricing, PricingData } from '../api';

export interface ModelCostInfo {
  input_token_cost: number;
  output_token_cost: number;
  currency: string;
  context_length?: number;
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
      context_length: undefined,
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

    const pricing: PricingData | undefined = response.data.pricing?.[0];

    if (pricing) {
      return {
        input_token_cost: pricing.input_token_cost,
        output_token_cost: pricing.output_token_cost,
        currency: pricing.currency,
        context_length: pricing.context_length ?? undefined,
      };
    }

    return null;
  } catch {
    return null;
  }
}
