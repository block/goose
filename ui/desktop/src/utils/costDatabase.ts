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
const CACHE_TTL_MS = 7 * 24 * 60 * 60 * 1000; // 7 days in milliseconds

interface PricingCacheData {
  pricing: Array<{
    provider: string;
    model: string;
    input_token_cost: number;
    output_token_cost: number;
    currency: string;
  }>;
  timestamp: number;
}

/**
 * Load pricing data from localStorage cache
 */
function loadPricingFromLocalStorage(): PricingCacheData | null {
  try {
    const cached = localStorage.getItem(PRICING_CACHE_KEY);
    const timestamp = localStorage.getItem(PRICING_CACHE_TIMESTAMP_KEY);
    
    if (cached && timestamp) {
      const cacheAge = Date.now() - parseInt(timestamp, 10);
      if (cacheAge < CACHE_TTL_MS) {
        console.log(`Loading pricing from localStorage (age: ${Math.round(cacheAge / 1000 / 60)} minutes)`);
        return JSON.parse(cached);
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
 * Save pricing data to localStorage
 */
function savePricingToLocalStorage(data: PricingCacheData): void {
  try {
    localStorage.setItem(PRICING_CACHE_KEY, JSON.stringify(data));
    localStorage.setItem(PRICING_CACHE_TIMESTAMP_KEY, data.timestamp.toString());
    console.log('Saved pricing data to localStorage');
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
      return null;
    }

    const data = await response.json();
    console.log('Pricing response:', data);

    // Find the specific model pricing
    const pricing = data.pricing?.find((p: {
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
    });

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
      data.pricing?.map((p: {
        provider: string;
        model: string;
      }) => `${p.provider}/${p.model}`)
    );
    return null;
  } catch (error) {
    console.error('Error fetching pricing data:', error);
    return null;
  }
}

/**
 * Initialize the cost database - load all pricing data on startup
 */
export async function initializeCostDatabase(): Promise<void> {
  try {
    // First check if we have valid cached data
    const cachedData = loadPricingFromLocalStorage();
    if (cachedData) {
      console.log('Using cached pricing data from localStorage');
      return;
    }

    // Fetch fresh pricing data from backend
    const apiUrl = getApiUrl('/config/pricing');
    const secretKey = getSecretKey();

    console.log('Fetching all pricing data on startup...');

    const headers: HeadersInit = { 'Content-Type': 'application/json' };
    if (secretKey) {
      headers['X-Secret-Key'] = secretKey;
    }

    const response = await fetch(apiUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify({ configured_only: false }), // Get all pricing data
    });

    if (!response.ok) {
      console.error('Failed to fetch initial pricing data:', response.status);
      return;
    }

    const data = await response.json();
    console.log(`Fetched pricing for ${data.pricing?.length || 0} models`);

    if (data.pricing && data.pricing.length > 0) {
      // Save to localStorage
      const cacheData: PricingCacheData = {
        pricing: data.pricing,
        timestamp: Date.now(),
      };
      savePricingToLocalStorage(cacheData);
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

  // Check localStorage cache
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
): Promise<ModelCostInfo | null> {
  try {
    const costInfo = await fetchPricingForModel(provider, model);

    // Cache the result in memory (even if null)
    currentModelPricing = { provider, model, costInfo };

    // If we got pricing data, update localStorage cache with this new data
    if (costInfo) {
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
    }

    return costInfo;
  } catch (error) {
    console.error('Error in fetchAndCachePricing:', error);
    return null;
  }
}

/**
 * Refresh pricing data from backend
 */
export async function refreshPricing(): Promise<boolean> {
  try {
    const apiUrl = getApiUrl('/config/pricing');
    const secretKey = getSecretKey();

    const headers: HeadersInit = { 'Content-Type': 'application/json' };
    if (secretKey) {
      headers['X-Secret-Key'] = secretKey;
    }

    const response = await fetch(apiUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify({ configured_only: false }), // Force refresh
    });

    if (response.ok) {
      const data = await response.json();
      
      if (data.pricing && data.pricing.length > 0) {
        // Save fresh data to localStorage
        const cacheData: PricingCacheData = {
          pricing: data.pricing,
          timestamp: Date.now(),
        };
        savePricingToLocalStorage(cacheData);
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
