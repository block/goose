import { useState, useEffect } from 'react';
import { useModelAndProvider } from '../ModelAndProviderContext';
import { useConfig } from '../ConfigContext';
import { Coins } from '../icons';
import type { ModelInfo } from '../../api/types.gen';

interface ModelCostInfo {
  input_token_cost?: number; // Cost per 1K input tokens
  output_token_cost?: number; // Cost per 1K output tokens
  currency?: string; // Currency symbol
}

interface CostTrackerProps {
  inputTokens?: number;
  outputTokens?: number;
}

export function CostTracker({ inputTokens = 0, outputTokens = 0 }: CostTrackerProps) {
  const { currentModel, currentProvider } = useModelAndProvider();
  const { getProviders } = useConfig();
  const [costInfo, setCostInfo] = useState<ModelCostInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Debug log props
  console.log('CostTracker props:', { inputTokens, outputTokens });

  useEffect(() => {
    const loadCostInfo = async () => {
      if (!currentModel || !currentProvider) {
        setIsLoading(false);
        return;
      }

      try {
        const providers = await getProviders(true);
        const provider = providers.find((p) => p.name === currentProvider);

        if (provider?.metadata?.known_models) {
          const modelConfig = provider.metadata.known_models.find((m) => m.name === currentModel);

          // For now, we'll check if the model has cost information in its metadata
          // This would need to be added to the backend provider configuration
          if (modelConfig) {
            const modelWithCost = modelConfig as ModelInfo & {
              input_token_cost?: number;
              output_token_cost?: number;
              currency?: string;
            };

            if (modelWithCost.input_token_cost !== undefined) {
              setCostInfo({
                input_token_cost: modelWithCost.input_token_cost,
                output_token_cost: modelWithCost.output_token_cost,
                currency: modelWithCost.currency || '$',
              });
            } else {
              // Fallback: Try to get cost info from a local cost database
              const costData = await getCostDataForModel(currentProvider, currentModel);
              console.log('Got cost data from local database:', costData);
              setCostInfo(costData);
            }
          } else {
            // Fallback: Try to get cost info from a local cost database
            const costData = await getCostDataForModel(currentProvider, currentModel);
            console.log('Got cost data from local database (no model config):', costData);
            setCostInfo(costData);
          }
        } else {
          // No known models, use local database
          const costData = await getCostDataForModel(currentProvider, currentModel);
          console.log('Got cost data from local database (no known models):', costData);
          setCostInfo(costData);
        }
      } catch (error) {
        console.error('Error loading cost info:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadCostInfo();
  }, [currentModel, currentProvider, getProviders]);

  const calculateCost = (): number => {
    if (!costInfo || (costInfo.input_token_cost === undefined && costInfo.output_token_cost === undefined)) {
      return 0;
    }

    const inputCost = (inputTokens / 1000) * (costInfo.input_token_cost || 0);
    const outputCost = (outputTokens / 1000) * (costInfo.output_token_cost || 0);
    const total = inputCost + outputCost;
    
    console.log('Cost calculation:', {
      inputTokens,
      outputTokens,
      inputCostPer1k: costInfo.input_token_cost,
      outputCostPer1k: costInfo.output_token_cost,
      inputCost,
      outputCost,
      total
    });

    return total;
  };

  const formatCost = (cost: number): string => {
    if (cost === 0) return '0.00';
    if (cost < 0.01) return cost.toFixed(4);
    if (cost < 1) return cost.toFixed(3);
    return cost.toFixed(2);
  };

  // Debug logging
  console.log('CostTracker state:', {
    isLoading,
    costInfo,
    inputTokens,
    outputTokens,
    currentModel,
    currentProvider,
  });

  // Show loading state or when we don't have model/provider info
  if (!currentModel || !currentProvider) {
    console.log('CostTracker: No model or provider');
    return null;
  }

  // If still loading, show a placeholder
  if (isLoading) {
    return (
      <div className="flex items-center gap-1 text-textSubtle px-2">
        <Coins className="w-3 h-3" />
        <span className="text-xs">...</span>
      </div>
    );
  }

  // If no cost info found, try to return a default
  if (!costInfo || (costInfo.input_token_cost === undefined && costInfo.output_token_cost === undefined)) {
    console.log('CostTracker: No cost info, checking for local/free model');
    
    // If it's a known free/local provider, show $0.00 without "not available" message
    const freeProviders = ['ollama', 'local', 'localhost'];
    if (freeProviders.includes(currentProvider.toLowerCase())) {
      return (
        <div
          className="flex items-center gap-1 text-textSubtle hover:text-textStandard transition-colors cursor-default px-2"
          title={`Local model (${inputTokens.toLocaleString()} input, ${outputTokens.toLocaleString()} output tokens)`}
        >
          <Coins className="w-3 h-3" />
          <span className="text-xs">$0.00</span>
        </div>
      );
    }
    
    // Otherwise show as unavailable
    return (
      <div
        className="flex items-center gap-1 text-textSubtle hover:text-textStandard transition-colors cursor-default px-2"
        title={`Cost data not available for ${currentModel} (${inputTokens.toLocaleString()} input, ${outputTokens.toLocaleString()} output tokens)`}
      >
        <Coins className="w-3 h-3" />
        <span className="text-xs">$0.00</span>
      </div>
    );
  }

  const totalCost = calculateCost();

  return (
    <div
      className="flex items-center gap-1 text-textSubtle hover:text-textStandard transition-colors cursor-default px-2"
      title={`Input: ${inputTokens.toLocaleString()} tokens (${costInfo.currency || '$'}${((inputTokens / 1000) * (costInfo.input_token_cost || 0)).toFixed(4)}) | Output: ${outputTokens.toLocaleString()} tokens (${costInfo.currency || '$'}${((outputTokens / 1000) * (costInfo.output_token_cost || 0)).toFixed(4)})`}
    >
      <Coins className="w-3 h-3" />
      <span className="text-xs">
        {costInfo.currency || '$'}
        {formatCost(totalCost)}
      </span>
    </div>
  );
}

// Local cost database - this would be maintained and updated periodically
async function getCostDataForModel(provider: string, model: string): Promise<ModelCostInfo | null> {
  // This is a simplified cost database. In production, this would be fetched from
  // the backend or a regularly updated configuration file
  const costDatabase: Record<string, Record<string, ModelCostInfo>> = {
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
      'gemini-2.0-flash-exp': { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' }, // Free experimental
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
    // Local and custom models - add common ones with estimated costs
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
      // For local models, show $0.00 as they don't have usage costs
      default: { input_token_cost: 0.0, output_token_cost: 0.0, currency: '$' },
    },
    // Add more providers and models as needed
  };

  console.log('getCostDataForModel called with:', { provider, model });

  const providerData = costDatabase[provider.toLowerCase()];
  if (!providerData) {
    console.log('No provider data found for:', provider);
    return null;
  }

  // Try exact match first
  if (providerData[model]) {
    console.log('Exact match found for model:', model);
    return providerData[model];
  }

  // Try to find a partial match (for versioned models)
  const modelLower = model.toLowerCase();
  for (const [key, value] of Object.entries(providerData)) {
    if (modelLower.includes(key.toLowerCase()) || key.toLowerCase().includes(modelLower)) {
      console.log('Partial match found:', key, 'for model:', model);
      return value;
    }
  }

  // Special handling for opus models
  if (modelLower.includes('opus')) {
    console.log('Opus model detected, using claude-3-opus pricing');
    return providerData['claude-3-opus'] || providerData['claude-opus-4'] || null;
  }

  console.log('No cost data found for model:', model);
  return null;
}
