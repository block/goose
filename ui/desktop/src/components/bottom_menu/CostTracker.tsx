import { useState, useEffect } from 'react';
import { useModelAndProvider } from '../ModelAndProviderContext';
import { useConfig } from '../ConfigContext';
import { Coins } from '../icons';
import { getCostForModel, initializeCostDatabase, updateAllModelCosts } from '../../utils/costDatabase';

interface CostTrackerProps {
  inputTokens?: number;
  outputTokens?: number;
}

export function CostTracker({ inputTokens = 0, outputTokens = 0 }: CostTrackerProps) {
  const { currentModel, currentProvider } = useModelAndProvider();
  const { getProviders } = useConfig();
  const [costInfo, setCostInfo] = useState<{ input_token_cost?: number; output_token_cost?: number; currency?: string } | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Debug log props
  console.log('CostTracker props:', { inputTokens, outputTokens });

  // Initialize cost database on mount
  useEffect(() => {
    initializeCostDatabase();
    
    // Update costs for all models in background
    updateAllModelCosts(getProviders).catch(error => {
      console.error('Failed to update model costs:', error);
    });
  }, [getProviders]);

  useEffect(() => {
    const loadCostInfo = async () => {
      if (!currentModel || !currentProvider) {
        setIsLoading(false);
        return;
      }

      try {
        // Get cost from centralized database
        const costData = getCostForModel(currentProvider, currentModel);
        console.log('Got cost data from database:', costData);
        setCostInfo(costData);
      } catch (error) {
        console.error('Error loading cost info:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadCostInfo();
  }, [currentModel, currentProvider]);

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
    // Always show 6 decimal places for consistency
    return cost.toFixed(6);
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
      <div className="flex items-center gap-1 text-textSubtle">
        <Coins className="w-3 h-3" />
        <span className="text-xs font-mono">...</span>
      </div>
    );
  }

  // If no cost info found, try to return a default
  if (!costInfo || (costInfo.input_token_cost === undefined && costInfo.output_token_cost === undefined)) {
    console.log('CostTracker: No cost info, checking for local/free model');
    
    // If it's a known free/local provider, show $0.000000 without "not available" message
    const freeProviders = ['ollama', 'local', 'localhost'];
    if (freeProviders.includes(currentProvider.toLowerCase())) {
      return (
        <div
          className="flex items-center gap-1 text-textSubtle hover:text-textStandard transition-colors cursor-default"
          title={`Local model (${inputTokens.toLocaleString()} input, ${outputTokens.toLocaleString()} output tokens)`}
        >
          <Coins className="w-3 h-3" />
          <span className="text-xs font-mono">$0.000000</span>
        </div>
      );
    }
    
    // Otherwise show as unavailable
    return (
      <div
        className="flex items-center gap-1 text-textSubtle hover:text-textStandard transition-colors cursor-default"
        title={`Cost data not available for ${currentModel} (${inputTokens.toLocaleString()} input, ${outputTokens.toLocaleString()} output tokens)`}
      >
        <Coins className="w-3 h-3" />
        <span className="text-xs font-mono">$0.000000</span>
      </div>
    );
  }

  const totalCost = calculateCost();

  return (
    <div
      className="flex items-center gap-1 text-textSubtle hover:text-textStandard transition-colors cursor-default"
      title={`Input: ${inputTokens.toLocaleString()} tokens (${costInfo.currency || '$'}${((inputTokens / 1000) * (costInfo.input_token_cost || 0)).toFixed(4)}) | Output: ${outputTokens.toLocaleString()} tokens (${costInfo.currency || '$'}${((outputTokens / 1000) * (costInfo.output_token_cost || 0)).toFixed(4)})`}
    >
      <Coins className="w-3 h-3" />
      <span className="text-xs font-mono">
        {costInfo.currency || '$'}
        {formatCost(totalCost)}
      </span>
    </div>
  );
}
