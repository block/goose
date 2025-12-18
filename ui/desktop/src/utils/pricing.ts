import { getPricing, PricingData } from '../api';

/**
 * Fetch pricing for a specific provider/model from the backend
 */
export async function fetchModelPricing(
  provider: string,
  model: string
): Promise<PricingData | null> {
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
