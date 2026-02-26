import type { PricingData } from '@/api';
import { getPricing } from '@/api';

// OpenRouter model pricing — cached in memory
let openRouterCache: Map<string, PricingData> | null = null;
let openRouterFetchPromise: Promise<Map<string, PricingData>> | null = null;

// Skip server endpoint after first failure to avoid repeated 404 console noise
let serverPricingAvailable = true;
let serverPricingPromise: Promise<PricingData | null> | null = null;

/**
 * Fetch and cache all model pricing from OpenRouter's public API.
 * Based on Kilocode's approach: https://openrouter.ai/api/v1/models
 */
async function fetchOpenRouterPricing(): Promise<Map<string, PricingData>> {
  if (openRouterCache) return openRouterCache;

  // Deduplicate concurrent requests
  if (openRouterFetchPromise) return openRouterFetchPromise;

  openRouterFetchPromise = (async () => {
    try {
      const response = await fetch('https://openrouter.ai/api/v1/models', {
        headers: { Accept: 'application/json' },
      });

      if (!response.ok) {
        console.warn(`[pricing] OpenRouter API returned ${response.status}`);
        return new Map();
      }

      const json = await response.json();
      const models = json?.data;

      if (!Array.isArray(models)) {
        console.warn('[pricing] OpenRouter API returned unexpected format');
        return new Map();
      }

      const cache = new Map<string, PricingData>();

      for (const model of models) {
        if (!model?.id || !model?.pricing) continue;

        // OpenRouter prices are per-token as strings, convert to per-token numbers
        const inputCost = model.pricing.prompt ? parseFloat(model.pricing.prompt) : 0;
        const outputCost = model.pricing.completion ? parseFloat(model.pricing.completion) : 0;

        // Extract provider from model id (e.g., "anthropic/claude-3.5-sonnet" → "anthropic")
        const [provider] = model.id.split('/');

        const pricingData: PricingData = {
          model: model.id,
          provider: provider || 'openrouter',
          input_token_cost: inputCost,
          output_token_cost: outputCost,
          context_length: model.context_length ?? null,
          currency: '$',
        };

        // Store with full id as key
        cache.set(model.id, pricingData);

        // Also store with just the model name (after provider/) for fuzzy matching
        const modelName = model.id.split('/').slice(1).join('/');
        if (modelName && !cache.has(modelName)) {
          cache.set(modelName, pricingData);
        }
      }

      openRouterCache = cache;
      return cache;
    } catch (err) {
      console.warn('[pricing] Failed to fetch OpenRouter models:', err);
      return new Map();
    } finally {
      openRouterFetchPromise = null;
    }
  })();

  return openRouterFetchPromise;
}

/**
 * Find a model in the OpenRouter cache by fuzzy matching.
 * Tries exact match, then provider/model, then just model name.
 */
function findInOpenRouter(
  cache: Map<string, PricingData>,
  provider: string,
  model: string
): PricingData | null {
  // Try exact match: "anthropic/claude-3.5-sonnet"
  const exactKey = `${provider}/${model}`;
  const exactMatch = cache.get(exactKey);
  if (exactMatch) return exactMatch;

  // Try just model name
  const modelMatch = cache.get(model);
  if (modelMatch) return modelMatch;

  // Try partial match: model name contains the search term
  for (const [key, data] of cache) {
    if (key.includes(model) || model.includes(key.split('/').pop() || '')) {
      return data;
    }
  }

  return null;
}

/**
 * Fetch pricing for a specific provider/model.
 * Strategy: try backend first, fall back to OpenRouter API.
 */
export async function fetchModelPricing(
  provider: string,
  model: string
): Promise<PricingData | null> {
  // 1. Try the backend endpoint (skip if previously failed, deduplicate concurrent calls)
  if (serverPricingAvailable) {
    if (!serverPricingPromise) {
      serverPricingPromise = (async () => {
        try {
          const response = await getPricing({
            body: { provider, model },
            throwOnError: false,
          });

          if (response.data?.pricing?.[0]) {
            return response.data.pricing[0];
          }

          serverPricingAvailable = false;
          return null;
        } catch {
          serverPricingAvailable = false;
          return null;
        } finally {
          serverPricingPromise = null;
        }
      })();
    }

    const result = await serverPricingPromise;
    if (result) return result;
  }

  // 2. Fall back to OpenRouter public API
  try {
    const cache = await fetchOpenRouterPricing();
    return findInOpenRouter(cache, provider, model);
  } catch {
    return null;
  }
}
