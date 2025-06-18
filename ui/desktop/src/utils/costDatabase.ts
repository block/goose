import { useConfig } from '../components/ConfigContext';

export interface ModelCostInfo {
  input_token_cost: number; // Cost per 1K input tokens
  output_token_cost: number; // Cost per 1K output tokens
  currency: string; // Currency symbol
}

// Static cost database for known models
const STATIC_COST_DATABASE: Record<string, Record<string, ModelCostInfo>> = {
  openai: {
    'gpt-4o': { input_token_cost: 0.0025, output_token_cost: 0.01, currency: '$' },
    'gpt-4o-2024-11-20': { input_token_cost: 0.0025, output_token_cost: 0.01, currency: '$' },
    'gpt-4o-mini': { input_token_cost: 0.00015, output_token_cost: 0.0006, currency: '$' },
    'gpt-4o-mini-2024-07-18': {
      input_token_cost: 0.00015,
      output_token_cost: 0.0006,
      currency: '$',
    },
    o1: { input_token_cost: 0.015, output_token_cost: 0.06, currency: '$' },
    'o1-mini': { input_token_cost: 0.003, output_token_cost: 0.012, currency: '$' },
    'gpt-4-turbo': { input_token_cost: 0.01, output_token_cost: 0.03, currency: '$' },
    'gpt-3.5-turbo': { input_token_cost: 0.0005, output_token_cost: 0.0015, currency: '$' },
  },
  anthropic: {
    'claude-3-5-sonnet-20241022': {
      input_token_cost: 0.003,
      output_token_cost: 0.015,
      currency: '$',
    },
    'claude-3-5-sonnet': {
      input_token_cost: 0.003,
      output_token_cost: 0.015,
      currency: '$',
    },
    'claude-3.5-sonnet': {
      input_token_cost: 0.003,
      output_token_cost: 0.015,
      currency: '$',
    },
    'claude-3-5-haiku-20241022': {
      input_token_cost: 0.001,
      output_token_cost: 0.005,
      currency: '$',
    },
    'claude-3-5-haiku': {
      input_token_cost: 0.001,
      output_token_cost: 0.005,
      currency: '$',
    },
    'claude-3-opus-20240229': {
      input_token_cost: 0.015,
      output_token_cost: 0.075,
      currency: '$',
    },
    'claude-3-opus': {
      input_token_cost: 0.015,
      output_token_cost: 0.075,
      currency: '$',
    },
    'claude-opus-4-20250514': {
      input_token_cost: 0.015,
      output_token_cost: 0.075,
      currency: '$',
    },
    'claude-opus-4': {
      input_token_cost: 0.015,
      output_token_cost: 0.075,
      currency: '$',
    },
    'claude-3-sonnet-20240229': {
      input_token_cost: 0.003,
      output_token_cost: 0.015,
      currency: '$',
    },
    'claude-3-sonnet': {
      input_token_cost: 0.003,
      output_token_cost: 0.015,
      currency: '$',
    },
    'claude-3-haiku-20240307': {
      input_token_cost: 0.00025,
      output_token_cost: 0.00125,
      currency: '$',
    },
    'claude-3-haiku': {
      input_token_cost: 0.00025,
      output_token_cost: 0.00125,
      currency: '$',
    },
  },
  google: {
    'gemini-2.0-flash-exp': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'gemini-1.5-flash': { input_token_cost: 0.000075, output_token_cost: 0.0003, currency: '$' },
    'gemini-1.5-flash-8b': {
      input_token_cost: 0.0000375,
      output_token_cost: 0.00015,
      currency: '$',
    },
    'gemini-1.5-pro': { input_token_cost: 0.00125, output_token_cost: 0.005, currency: '$' },
    'gemini-1.0-pro': { input_token_cost: 0.00025, output_token_cost: 0.00125, currency: '$' },
  },
  groq: {
    'llama-3.3-70b-versatile': {
      input_token_cost: 0.00059,
      output_token_cost: 0.00079,
      currency: '$',
    },
    'llama-3.1-70b-versatile': {
      input_token_cost: 0.00059,
      output_token_cost: 0.00079,
      currency: '$',
    },
    'llama-3.1-8b-instant': {
      input_token_cost: 0.00005,
      output_token_cost: 0.00008,
      currency: '$',
    },
    'mixtral-8x7b-32768': {
      input_token_cost: 0.00024,
      output_token_cost: 0.00024,
      currency: '$',
    },
  },
  deepseek: {
    'deepseek-chat': { input_token_cost: 0.00014, output_token_cost: 0.00028, currency: '$' },
    'deepseek-reasoner': { input_token_cost: 0.00055, output_token_cost: 0.00219, currency: '$' },
  },
  // Local and custom models
  ollama: {
    'llama3.2': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'llama3.1': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'llama3': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'llama2': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'mistral': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'mixtral': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'codellama': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'deepseek-coder': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'phi': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    'qwen': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
  },
  local: {
    default: { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
  },
};

// Local storage key for custom cost data
const CUSTOM_COSTS_KEY = 'goose_model_costs';

// Cache for cost data
let costCache: Record<string, Record<string, ModelCostInfo>> = {};

/**
 * Load custom costs from local storage
 */
function loadCustomCosts(): Record<string, Record<string, ModelCostInfo>> {
  try {
    const stored = localStorage.getItem(CUSTOM_COSTS_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch (error) {
    console.error('Error loading custom costs:', error);
  }
  return {};
}

/**
 * Save custom costs to local storage
 */
function saveCustomCosts(costs: Record<string, Record<string, ModelCostInfo>>) {
  try {
    localStorage.setItem(CUSTOM_COSTS_KEY, JSON.stringify(costs));
  } catch (error) {
    console.error('Error saving custom costs:', error);
  }
}

/**
 * Initialize cost database by merging static and custom costs
 */
export function initializeCostDatabase() {
  const customCosts = loadCustomCosts();
  costCache = { ...STATIC_COST_DATABASE };
  
  // Merge custom costs into cache
  for (const [provider, models] of Object.entries(customCosts)) {
    if (!costCache[provider]) {
      costCache[provider] = {};
    }
    Object.assign(costCache[provider], models);
  }
}

/**
 * Get cost data for a specific model
 */
export function getCostForModel(provider: string, model: string): ModelCostInfo | null {
  // Initialize if not already done
  if (Object.keys(costCache).length === 0) {
    initializeCostDatabase();
  }

  const providerData = costCache[provider.toLowerCase()];
  if (!providerData) {
    return null;
  }

  // Try exact match first
  if (providerData[model]) {
    return providerData[model];
  }

  // Try partial match
  const modelLower = model.toLowerCase();
  for (const [key, value] of Object.entries(providerData)) {
    if (modelLower.includes(key.toLowerCase()) || key.toLowerCase().includes(modelLower)) {
      return value;
    }
  }

  // Special handling for opus models
  if (modelLower.includes('opus')) {
    return providerData['claude-3-opus'] || providerData['claude-opus-4'] || null;
  }

  return null;
}

/**
 * Update cost for a specific model
 */
export function updateModelCost(
  provider: string,
  model: string,
  costInfo: ModelCostInfo
) {
  const customCosts = loadCustomCosts();
  
  if (!customCosts[provider]) {
    customCosts[provider] = {};
  }
  
  customCosts[provider][model] = costInfo;
  saveCustomCosts(customCosts);
  
  // Update cache
  if (!costCache[provider]) {
    costCache[provider] = {};
  }
  costCache[provider][model] = costInfo;
}

/**
 * Fetch and update costs for all configured models
 * This can be called on app startup or manually
 */
export async function updateAllModelCosts(getProviders: () => Promise<any[]>) {
  try {
    const providers = await getProviders(true);
    
    for (const provider of providers) {
      if (provider.metadata?.known_models) {
        for (const model of provider.metadata.known_models) {
          // Check if model has cost info in metadata
          if (model.input_token_cost !== undefined) {
            updateModelCost(provider.name, model.name, {
              input_token_cost: model.input_token_cost,
              output_token_cost: model.output_token_cost || 0,
              currency: model.currency || '$',
            });
          }
        }
      }
    }
  } catch (error) {
    console.error('Error updating model costs:', error);
  }
}

/**
 * Get all known costs
 */
export function getAllCosts(): Record<string, Record<string, ModelCostInfo>> {
  if (Object.keys(costCache).length === 0) {
    initializeCostDatabase();
  }
  return { ...costCache };
}