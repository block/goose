import { getApiUrl } from '../config';
import { safeJsonParse } from './conversionUtils';

export interface ModelCostInfo {
  input_token_cost: number; // Cost per token for input (in USD)
  output_token_cost: number; // Cost per token for output (in USD)
  currency: string; // Currency symbol
}

// In-memory cache for current session only
const sessionPricingCache = new Map<string, ModelCostInfo | null>();

/**
 * Fetch pricing data from backend for specific provider/model
 */
async function fetchPricingForModel(
  provider: string,
  model: string
): Promise<ModelCostInfo | null> {
  // For OpenRouter models, we need to use the parsed provider and model for the API lookup
  let lookupProvider = provider;
  let lookupModel = model;

  if (provider.toLowerCase() === 'openrouter') {
    const parsed = parseOpenRouterModel(model);
    if (parsed) {
      lookupProvider = parsed[0];
      lookupModel = parsed[1];
    }
  }

  const apiUrl = getApiUrl('/config/pricing');
  const secretKey = await window.electron.getSecretKey();

  const headers: HeadersInit = { 'Content-Type': 'application/json' };
  if (secretKey) {
    headers['X-Secret-Key'] = secretKey;
  }

  const response = await fetch(apiUrl, {
    method: 'POST',
    headers,
    body: JSON.stringify({ configured_only: false }),
  });

  if (!response.ok) {
    throw new Error(`API request failed with status ${response.status}`);
  }

  const data = await safeJsonParse<{
    pricing: Array<{
      provider: string;
      model: string;
      input_token_cost: number;
      output_token_cost: number;
      currency: string;
    }>;
  }>(response, 'Failed to parse pricing data');

  // Find the specific model pricing using the lookup provider/model
  const pricing = data.pricing?.find(
    (p: {
      provider: string;
      model: string;
      input_token_cost: number;
      output_token_cost: number;
      currency: string;
    }) => {
      const providerMatch = p.provider.toLowerCase() === lookupProvider.toLowerCase();

      // More flexible model matching - handle versioned models
      let modelMatch = p.model === lookupModel;

      // If exact match fails, try matching without version suffix
      if (!modelMatch && lookupModel.includes('-20')) {
        // Remove date suffix like -20241022
        const modelWithoutDate = lookupModel.replace(/-20\d{6}$/, '');
        modelMatch = p.model === modelWithoutDate;

        // Also try with dots instead of dashes (claude-3-5-sonnet vs claude-3.5-sonnet)
        if (!modelMatch) {
          const modelWithDots = modelWithoutDate.replace(/-(\d)-/g, '.$1.');
          modelMatch = p.model === modelWithDots;
        }
      }

      return providerMatch && modelMatch;
    }
  );

  if (pricing) {
    return {
      input_token_cost: pricing.input_token_cost,
      output_token_cost: pricing.output_token_cost,
      currency: pricing.currency || '$',
    };
  }

  // API call succeeded but model not found in pricing data
  return null;
}

/**
 * Initialize the cost database - no-op since we fetch on demand now
 */
export async function initializeCostDatabase(): Promise<void> {
  // Clear session cache on init
  sessionPricingCache.clear();
}

/**
 * Update model costs from providers - no-op since we fetch on demand
 */
export async function updateAllModelCosts(): Promise<void> {
  // No-op - we fetch on demand now
}

/**
 * Parse OpenRouter model ID to extract provider and model
 * e.g., "anthropic/claude-sonnet-4" -> ["anthropic", "claude-sonnet-4"]
 */
function parseOpenRouterModel(modelId: string): [string, string] | null {
  const parts = modelId.split('/');
  if (parts.length === 2) {
    return [parts[0], parts[1]];
  }
  return null;
}

/**
 * Get cost information for a specific model with session caching
 */
export function getCostForModel(provider: string, model: string): ModelCostInfo | null {
  const cacheKey = `${provider}/${model}`;

  // Check session cache first
  if (sessionPricingCache.has(cacheKey)) {
    return sessionPricingCache.get(cacheKey) || null;
  }

  // For OpenRouter models, also check if we have cached data under the parsed provider/model
  if (provider.toLowerCase() === 'openrouter') {
    const parsed = parseOpenRouterModel(model);
    if (parsed) {
      const [parsedProvider, parsedModel] = parsed;
      const parsedCacheKey = `${parsedProvider}/${parsedModel}`;
      if (sessionPricingCache.has(parsedCacheKey)) {
        const cachedData = sessionPricingCache.get(parsedCacheKey) || null;
        // Also cache it under the original OpenRouter key for future lookups
        sessionPricingCache.set(cacheKey, cachedData);
        return cachedData;
      }
    }
  }

  // For local/free providers, return zero cost immediately
  const freeProviders = ['ollama', 'local', 'localhost'];
  if (freeProviders.includes(provider.toLowerCase())) {
    const zeroCost = {
      input_token_cost: 0,
      output_token_cost: 0,
      currency: '$',
    };
    sessionPricingCache.set(cacheKey, zeroCost);
    return zeroCost;
  }

  // Need to fetch - return null and let component handle async fetch
  return null;
}

/**
 * Fetch and cache pricing for a model
 */
export async function fetchAndCachePricing(
  provider: string,
  model: string
): Promise<{ costInfo: ModelCostInfo | null; error?: string } | null> {
  try {
    const cacheKey = `${provider}/${model}`;
    const costInfo = await fetchPricingForModel(provider, model);

    // Cache the result in session cache under the original key
    sessionPricingCache.set(cacheKey, costInfo);

    // For OpenRouter models, also cache under the parsed provider/model key
    // This helps with cross-referencing between frontend requests and backend responses
    if (provider.toLowerCase() === 'openrouter') {
      const parsed = parseOpenRouterModel(model);
      if (parsed) {
        const [parsedProvider, parsedModel] = parsed;
        const parsedCacheKey = `${parsedProvider}/${parsedModel}`;
        sessionPricingCache.set(parsedCacheKey, costInfo);
      }
    }

    if (costInfo) {
      return { costInfo };
    } else {
      // Model not found in pricing data
      return { costInfo: null, error: 'model_not_found' };
    }
  } catch {
    // This is a real API/network error
    return null;
  }
}
