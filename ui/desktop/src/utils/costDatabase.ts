// Import the proper type from ConfigContext
import { getApiUrl, getSecretKey } from '../config';

export interface ModelCostInfo {
  input_token_cost: number; // Cost per token for input (in USD)
  output_token_cost: number; // Cost per token for output (in USD)
  currency: string; // Currency symbol
}

// In-memory cache for current model pricing only
let currentModelPricing: {
  provider: string;
  model: string;
  costInfo: ModelCostInfo | null;
} | null = null;

// LocalStorage keys
const PRICING_CACHE_KEY = 'goose_pricing_cache';
const PRICING_CACHE_TIMESTAMP_KEY = 'goose_pricing_cache_timestamp';
const RECENTLY_USED_MODELS_KEY = 'goose_recently_used_models';
const CACHE_TTL_MS = 7 * 24 * 60 * 60 * 1000; // 7 days in milliseconds
const MAX_RECENTLY_USED_MODELS = 20; // Keep only the last 20 used models in cache

interface PricingItem {
  provider: string;
  model: string;
  input_token_cost: number;
  output_token_cost: number;
  currency: string;
}

interface PricingCacheData {
  pricing: PricingItem[];
  timestamp: number;
}

interface RecentlyUsedModel {
  provider: string;
  model: string;
  lastUsed: number;
}

/**
 * Get recently used models from localStorage
 */
function getRecentlyUsedModels(): RecentlyUsedModel[] {
  try {
    const stored = localStorage.getItem(RECENTLY_USED_MODELS_KEY);
    return stored ? JSON.parse(stored) : [];
  } catch (error) {
    console.error('Error loading recently used models:', error);
    return [];
  }
}

/**
 * Add a model to the recently used list
 */
function addToRecentlyUsed(provider: string, model: string): void {
  try {
    let recentModels = getRecentlyUsedModels();

    // Remove existing entry if present
    recentModels = recentModels.filter((m) => !(m.provider === provider && m.model === model));

    // Add to front
    recentModels.unshift({ provider, model, lastUsed: Date.now() });

    // Keep only the most recent models
    recentModels = recentModels.slice(0, MAX_RECENTLY_USED_MODELS);

    localStorage.setItem(RECENTLY_USED_MODELS_KEY, JSON.stringify(recentModels));
  } catch (error) {
    console.error('Error saving recently used models:', error);
  }
}

/**
 * Load pricing data from localStorage cache - only for recently used models
 */
function loadPricingFromLocalStorage(): PricingCacheData | null {
  try {
    const cached = localStorage.getItem(PRICING_CACHE_KEY);
    const timestamp = localStorage.getItem(PRICING_CACHE_TIMESTAMP_KEY);

    if (cached && timestamp) {
      const cacheAge = Date.now() - parseInt(timestamp, 10);
      if (cacheAge < CACHE_TTL_MS) {
        const fullCache = JSON.parse(cached) as PricingCacheData;
        const recentModels = getRecentlyUsedModels();

        // Filter to only include recently used models
        const filteredPricing = fullCache.pricing.filter((p) =>
          recentModels.some((r) => r.provider === p.provider && r.model === p.model)
        );

        console.log(
          `Loading ${filteredPricing.length} recently used models from cache (out of ${fullCache.pricing.length} total)`
        );

        return {
          pricing: filteredPricing,
          timestamp: fullCache.timestamp,
        };
      } else {
        console.log('LocalStorage pricing cache expired');
      }
    }
  } catch (error) {
    console.error('Error loading pricing from localStorage:', error);
  }
  return null;
}

/**
 * Save pricing data to localStorage - merge with existing data
 */
function savePricingToLocalStorage(data: PricingCacheData, mergeWithExisting = true): void {
  try {
    if (mergeWithExisting) {
      // Load existing full cache
      const existingCached = localStorage.getItem(PRICING_CACHE_KEY);
      if (existingCached) {
        const existingData = JSON.parse(existingCached) as PricingCacheData;

        // Create a map of existing pricing for quick lookup
        const pricingMap = new Map<string, (typeof data.pricing)[0]>();
        existingData.pricing.forEach((p) => {
          pricingMap.set(`${p.provider}/${p.model}`, p);
        });

        // Update with new data
        data.pricing.forEach((p) => {
          pricingMap.set(`${p.provider}/${p.model}`, p);
        });

        // Convert back to array
        data = {
          pricing: Array.from(pricingMap.values()),
          timestamp: data.timestamp,
        };
      }
    }

    localStorage.setItem(PRICING_CACHE_KEY, JSON.stringify(data));
    localStorage.setItem(PRICING_CACHE_TIMESTAMP_KEY, data.timestamp.toString());
    console.log(`Saved ${data.pricing.length} models to localStorage cache`);
  } catch (error) {
    console.error('Error saving pricing to localStorage:', error);
  }
}

/**
 * Fetch pricing data from backend for specific provider/model
 */
async function fetchPricingForModel(
  provider: string,
  model: string
): Promise<ModelCostInfo | null> {
  try {
    const apiUrl = getApiUrl('/config/pricing');
    const secretKey = getSecretKey();

    console.log(`Fetching pricing for ${provider}/${model} from ${apiUrl}`);

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
      console.error('Failed to fetch pricing data:', response.status);
      throw new Error(`API request failed with status ${response.status}`);
    }

    const data = await response.json();
    console.log('Pricing response:', data);

    // Find the specific model pricing
    const pricing = data.pricing?.find(
      (p: {
        provider: string;
        model: string;
        input_token_cost: number;
        output_token_cost: number;
        currency: string;
      }) => {
        const providerMatch = p.provider.toLowerCase() === provider.toLowerCase();

        // More flexible model matching - handle versioned models
        let modelMatch = p.model === model;

        // If exact match fails, try matching without version suffix
        if (!modelMatch && model.includes('-20')) {
          // Remove date suffix like -20241022
          const modelWithoutDate = model.replace(/-20\d{6}$/, '');
          modelMatch = p.model === modelWithoutDate;

          // Also try with dots instead of dashes (claude-3-5-sonnet vs claude-3.5-sonnet)
          if (!modelMatch) {
            const modelWithDots = modelWithoutDate.replace(/-(\d)-/g, '.$1.');
            modelMatch = p.model === modelWithDots;
          }
        }

        console.log(
          `Comparing: ${p.provider}/${p.model} with ${provider}/${model} - Provider match: ${providerMatch}, Model match: ${modelMatch}`
        );
        return providerMatch && modelMatch;
      }
    );

    console.log(`Found pricing for ${provider}/${model}:`, pricing);

    if (pricing) {
      return {
        input_token_cost: pricing.input_token_cost,
        output_token_cost: pricing.output_token_cost,
        currency: pricing.currency || '$',
      };
    }

    console.log(
      `No pricing found for ${provider}/${model} in:`,
      data.pricing?.map((p: { provider: string; model: string }) => `${p.provider}/${p.model}`)
    );

    // API call succeeded but model not found in pricing data
    return null;
  } catch (error) {
    console.error('Error fetching pricing data:', error);
    // Re-throw the error so the caller can distinguish between API failure and model not found
    throw error;
  }
}

/**
 * Initialize the cost database - only load commonly used models on startup
 */
export async function initializeCostDatabase(): Promise<void> {
  try {
    // Clean up any existing large caches first
    cleanupPricingCache();

    // First check if we have valid cached data
    const cachedData = loadPricingFromLocalStorage();
    if (cachedData && cachedData.pricing.length > 0) {
      console.log('Using cached pricing data from localStorage');
      return;
    }

    // List of commonly used models to pre-fetch
    const commonModels = [
      { provider: 'openai', model: 'gpt-4o' },
      { provider: 'openai', model: 'gpt-4o-mini' },
      { provider: 'openai', model: 'gpt-4-turbo' },
      { provider: 'openai', model: 'gpt-4' },
      { provider: 'openai', model: 'gpt-3.5-turbo' },
      { provider: 'anthropic', model: 'claude-3-5-sonnet' },
      { provider: 'anthropic', model: 'claude-3-5-sonnet-20241022' },
      { provider: 'anthropic', model: 'claude-3-opus' },
      { provider: 'anthropic', model: 'claude-3-sonnet' },
      { provider: 'anthropic', model: 'claude-3-haiku' },
      { provider: 'google', model: 'gemini-1.5-pro' },
      { provider: 'google', model: 'gemini-1.5-flash' },
      { provider: 'deepseek', model: 'deepseek-chat' },
      { provider: 'deepseek', model: 'deepseek-reasoner' },
      { provider: 'meta-llama', model: 'llama-3.2-90b-text-preview' },
      { provider: 'meta-llama', model: 'llama-3.1-405b-instruct' },
    ];

    // Get recently used models
    const recentModels = getRecentlyUsedModels();

    // Combine common and recent models (deduplicated)
    const modelsToFetch = new Map<string, { provider: string; model: string }>();

    // Add common models
    commonModels.forEach((m) => {
      modelsToFetch.set(`${m.provider}/${m.model}`, m);
    });

    // Add recent models
    recentModels.forEach((m) => {
      modelsToFetch.set(`${m.provider}/${m.model}`, { provider: m.provider, model: m.model });
    });

    console.log(`Initializing cost database with ${modelsToFetch.size} models...`);

    // Fetch only the pricing we need
    const apiUrl = getApiUrl('/config/pricing');
    const secretKey = getSecretKey();

    const headers: HeadersInit = { 'Content-Type': 'application/json' };
    if (secretKey) {
      headers['X-Secret-Key'] = secretKey;
    }

    const response = await fetch(apiUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify({
        configured_only: false,
        models: Array.from(modelsToFetch.values()), // Send specific models if API supports it
      }),
    });

    if (!response.ok) {
      console.error('Failed to fetch initial pricing data:', response.status);
      return;
    }

    const data = await response.json();
    console.log(`Fetched pricing for ${data.pricing?.length || 0} models`);

    if (data.pricing && data.pricing.length > 0) {
      // Filter to only the models we requested (in case API returns all)
      const filteredPricing = data.pricing.filter((p: PricingItem) =>
        modelsToFetch.has(`${p.provider}/${p.model}`)
      );

      // Save to localStorage
      const cacheData: PricingCacheData = {
        pricing: filteredPricing.length > 0 ? filteredPricing : data.pricing.slice(0, 50), // Fallback to first 50 if filtering didn't work
        timestamp: Date.now(),
      };
      savePricingToLocalStorage(cacheData, false); // Don't merge on initial load
    }
  } catch (error) {
    console.error('Error initializing cost database:', error);
  }
}

/**
 * Update model costs from providers - no longer needed
 */
export async function updateAllModelCosts(): Promise<void> {
  // No-op - we fetch on demand now
}

/**
 * Get cost information for a specific model with caching
 */
export function getCostForModel(provider: string, model: string): ModelCostInfo | null {
  // Track this model as recently used
  addToRecentlyUsed(provider, model);

  // Check if it's the same model we already have cached in memory
  if (
    currentModelPricing &&
    currentModelPricing.provider === provider &&
    currentModelPricing.model === model
  ) {
    return currentModelPricing.costInfo;
  }

  // For local/free providers, return zero cost immediately
  const freeProviders = ['ollama', 'local', 'localhost'];
  if (freeProviders.includes(provider.toLowerCase())) {
    const zeroCost = {
      input_token_cost: 0,
      output_token_cost: 0,
      currency: '$',
    };
    currentModelPricing = { provider, model, costInfo: zeroCost };
    return zeroCost;
  }

  // Check localStorage cache (which now only contains recently used models)
  const cachedData = loadPricingFromLocalStorage();
  if (cachedData) {
    const pricing = cachedData.pricing.find((p) => {
      const providerMatch = p.provider.toLowerCase() === provider.toLowerCase();

      // More flexible model matching - handle versioned models
      let modelMatch = p.model === model;

      // If exact match fails, try matching without version suffix
      if (!modelMatch && model.includes('-20')) {
        // Remove date suffix like -20241022
        const modelWithoutDate = model.replace(/-20\d{6}$/, '');
        modelMatch = p.model === modelWithoutDate;

        // Also try with dots instead of dashes (claude-3-5-sonnet vs claude-3.5-sonnet)
        if (!modelMatch) {
          const modelWithDots = modelWithoutDate.replace(/-(\d)-/g, '.$1.');
          modelMatch = p.model === modelWithDots;
        }
      }

      return providerMatch && modelMatch;
    });

    if (pricing) {
      const costInfo = {
        input_token_cost: pricing.input_token_cost,
        output_token_cost: pricing.output_token_cost,
        currency: pricing.currency || '$',
      };
      currentModelPricing = { provider, model, costInfo };
      return costInfo;
    }
  }

  // Need to fetch new pricing - return null for now
  // The component will handle the async fetch
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
    const costInfo = await fetchPricingForModel(provider, model);

    if (costInfo) {
      // Cache the result in memory
      currentModelPricing = { provider, model, costInfo };

      // Update localStorage cache with this new data
      const cachedData = loadPricingFromLocalStorage();
      if (cachedData) {
        // Check if this model already exists in cache
        const existingIndex = cachedData.pricing.findIndex(
          (p) => p.provider.toLowerCase() === provider.toLowerCase() && p.model === model
        );

        const newPricing = {
          provider,
          model,
          input_token_cost: costInfo.input_token_cost,
          output_token_cost: costInfo.output_token_cost,
          currency: costInfo.currency,
        };

        if (existingIndex >= 0) {
          // Update existing
          cachedData.pricing[existingIndex] = newPricing;
        } else {
          // Add new
          cachedData.pricing.push(newPricing);
        }

        // Save updated cache
        savePricingToLocalStorage(cachedData);
      }

      return { costInfo };
    } else {
      // Cache the null result in memory
      currentModelPricing = { provider, model, costInfo: null };

      // Check if the API call succeeded but model wasn't found
      // We can determine this by checking if we got a response but no matching model
      return { costInfo: null, error: 'model_not_found' };
    }
  } catch (error) {
    console.error('Error in fetchAndCachePricing:', error);
    // This is a real API/network error
    return null;
  }
}

/**
 * Refresh pricing data from backend - only refresh recently used models
 */
export async function refreshPricing(): Promise<boolean> {
  try {
    const apiUrl = getApiUrl('/config/pricing');
    const secretKey = getSecretKey();

    const headers: HeadersInit = { 'Content-Type': 'application/json' };
    if (secretKey) {
      headers['X-Secret-Key'] = secretKey;
    }

    // Get recently used models to refresh
    const recentModels = getRecentlyUsedModels();

    // Add some common models as well
    const commonModels = [
      { provider: 'openai', model: 'gpt-4o' },
      { provider: 'openai', model: 'gpt-4o-mini' },
      { provider: 'anthropic', model: 'claude-3-5-sonnet-20241022' },
      { provider: 'google', model: 'gemini-1.5-pro' },
    ];

    // Combine and deduplicate
    const modelsToRefresh = new Map<string, { provider: string; model: string }>();

    commonModels.forEach((m) => {
      modelsToRefresh.set(`${m.provider}/${m.model}`, m);
    });

    recentModels.forEach((m) => {
      modelsToRefresh.set(`${m.provider}/${m.model}`, { provider: m.provider, model: m.model });
    });

    console.log(`Refreshing pricing for ${modelsToRefresh.size} models...`);

    const response = await fetch(apiUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify({
        configured_only: false,
        models: Array.from(modelsToRefresh.values()), // Send specific models if API supports it
      }),
    });

    if (response.ok) {
      const data = await response.json();

      if (data.pricing && data.pricing.length > 0) {
        // Filter to only the models we requested (in case API returns all)
        const filteredPricing = data.pricing.filter((p: PricingItem) =>
          modelsToRefresh.has(`${p.provider}/${p.model}`)
        );

        // Save fresh data to localStorage (merge with existing)
        const cacheData: PricingCacheData = {
          pricing: filteredPricing.length > 0 ? filteredPricing : data.pricing.slice(0, 50),
          timestamp: Date.now(),
        };
        savePricingToLocalStorage(cacheData, true); // Merge with existing
      }

      // Clear current memory cache to force re-fetch
      currentModelPricing = null;
      return true;
    }

    return false;
  } catch (error) {
    console.error('Error refreshing pricing data:', error);
    return false;
  }
}

/**
 * Clean up old/unused models from the cache
 */
export function cleanupPricingCache(): void {
  try {
    const recentModels = getRecentlyUsedModels();
    const cachedData = localStorage.getItem(PRICING_CACHE_KEY);

    if (!cachedData) return;

    const fullCache = JSON.parse(cachedData) as PricingCacheData;
    const recentModelKeys = new Set(recentModels.map((m) => `${m.provider}/${m.model}`));

    // Keep only recently used models and common models
    const commonModelKeys = new Set([
      'openai/gpt-4o',
      'openai/gpt-4o-mini',
      'openai/gpt-4-turbo',
      'anthropic/claude-3-5-sonnet',
      'anthropic/claude-3-5-sonnet-20241022',
      'google/gemini-1.5-pro',
      'google/gemini-1.5-flash',
    ]);

    const filteredPricing = fullCache.pricing.filter((p) => {
      const key = `${p.provider}/${p.model}`;
      return recentModelKeys.has(key) || commonModelKeys.has(key);
    });

    if (filteredPricing.length < fullCache.pricing.length) {
      console.log(
        `Cleaned up pricing cache: reduced from ${fullCache.pricing.length} to ${filteredPricing.length} models`
      );

      const cleanedCache: PricingCacheData = {
        pricing: filteredPricing,
        timestamp: fullCache.timestamp,
      };

      localStorage.setItem(PRICING_CACHE_KEY, JSON.stringify(cleanedCache));
    }
  } catch (error) {
    console.error('Error cleaning up pricing cache:', error);
  }
}
