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
              setCostInfo(costData);
            }
          } else {
            // Fallback: Try to get cost info from a local cost database
            const costData = await getCostDataForModel(currentProvider, currentModel);
            setCostInfo(costData);
          }
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
    if (!costInfo || !costInfo.input_token_cost || !costInfo.output_token_cost) {
      return 0;
    }

    const inputCost = (inputTokens / 1000) * costInfo.input_token_cost;
    const outputCost = (outputTokens / 1000) * costInfo.output_token_cost;

    return inputCost + outputCost;
  };

  const formatCost = (cost: number): string => {
    if (cost === 0) return '0.00';
    if (cost < 0.01) return cost.toFixed(4);
    if (cost < 1) return cost.toFixed(3);
    return cost.toFixed(2);
  };

  // Always show the cost tracker if we have cost info, even with 0 tokens
  if (isLoading || !costInfo || (!costInfo.input_token_cost && !costInfo.output_token_cost)) {
    return null; // Don't show anything if we don't have cost data
  }

  const totalCost = calculateCost();

  return (
    <div
      className="flex items-center gap-1 text-textSubtle hover:text-textStandard transition-colors cursor-default"
      title={`Input: ${inputTokens.toLocaleString()} tokens, Output: ${outputTokens.toLocaleString()} tokens`}
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
      'claude-3-5-haiku-20241022': {
        input_token_cost: 0.001,
        output_token_cost: 0.005,
        currency: '$',
      },
      'claude-3-opus-20240229': {
        input_token_cost: 0.015,
        output_token_cost: 0.075,
        currency: '$',
      },
      'claude-3-sonnet-20240229': {
        input_token_cost: 0.003,
        output_token_cost: 0.015,
        currency: '$',
      },
      'claude-3-haiku-20240307': {
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
    // Add more providers and models as needed
  };

  const providerData = costDatabase[provider.toLowerCase()];
  if (!providerData) return null;

  // Try exact match first
  if (providerData[model]) {
    return providerData[model];
  }

  // Try to find a partial match (for versioned models)
  const modelLower = model.toLowerCase();
  for (const [key, value] of Object.entries(providerData)) {
    if (modelLower.includes(key.toLowerCase()) || key.toLowerCase().includes(modelLower)) {
      return value;
    }
  }

  return null;
}
