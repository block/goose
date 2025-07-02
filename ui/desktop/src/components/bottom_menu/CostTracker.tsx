import { useState, useEffect } from 'react';
import { useModelAndProvider } from '../ModelAndProviderContext';
import { useConfig } from '../ConfigContext';
import {
  getCostForModel,
  initializeCostDatabase,
  updateAllModelCosts,
  fetchAndCachePricing,
} from '../../utils/costDatabase';

interface CostTrackerProps {
  inputTokens?: number;
  outputTokens?: number;
  sessionCosts?: {
    [key: string]: {
      inputTokens: number;
      outputTokens: number;
      totalCost: number;
    };
  };
}

export function CostTracker({ inputTokens = 0, outputTokens = 0, sessionCosts }: CostTrackerProps) {
  const { currentModel, currentProvider } = useModelAndProvider();
  const { getProviders } = useConfig();
  const [costInfo, setCostInfo] = useState<{
    input_token_cost?: number;
    output_token_cost?: number;
    currency?: string;
  } | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [showPricing, setShowPricing] = useState(true);
  const [pricingFailed, setPricingFailed] = useState(false);
  const [modelNotFound, setModelNotFound] = useState(false);
  const [hasAttemptedFetch, setHasAttemptedFetch] = useState(false);
  const [initialLoadComplete, setInitialLoadComplete] = useState(false);

  // Check if pricing is enabled
  useEffect(() => {
    const checkPricingSetting = () => {
      const stored = localStorage.getItem('show_pricing');
      setShowPricing(stored !== 'false');
    };

    // Check on mount
    checkPricingSetting();

    // Listen for storage changes
    window.addEventListener('storage', checkPricingSetting);
    return () => window.removeEventListener('storage', checkPricingSetting);
  }, []);

  // Set initial load complete after a short delay
  useEffect(() => {
    const timer = setTimeout(() => {
      setInitialLoadComplete(true);
    }, 3000); // Give 3 seconds for initial load

    return () => window.clearTimeout(timer);
  }, []);

  // Debug log props removed

  // Initialize cost database on mount
  useEffect(() => {
    initializeCostDatabase();

    // Update costs for all models in background
    updateAllModelCosts().catch(() => {});
  }, [getProviders]);

  useEffect(() => {
    const loadCostInfo = async () => {
      if (!currentModel || !currentProvider) {
        setIsLoading(false);
        return;
      }

      try {
        // First check sync cache
        let costData = getCostForModel(currentProvider, currentModel);

        if (costData) {
          // We have cached data
          setCostInfo(costData);
          setPricingFailed(false);
          setModelNotFound(false);
          setIsLoading(false);
          setHasAttemptedFetch(true);
        } else {
          // Need to fetch from backend
          setIsLoading(true);
          const result = await fetchAndCachePricing(currentProvider, currentModel);
          setHasAttemptedFetch(true);

          if (result && result.costInfo) {
            setCostInfo(result.costInfo);
            setPricingFailed(false);
            setModelNotFound(false);
          } else if (result && result.error === 'model_not_found') {
            // Model not found in pricing database, but API call succeeded
            setModelNotFound(true);
            setPricingFailed(false);
          } else {
            // API call failed or other error
            const freeProviders = ['ollama', 'local', 'localhost'];
            if (!freeProviders.includes(currentProvider.toLowerCase())) {
              setPricingFailed(true);
              setModelNotFound(false);
            }
          }
          setIsLoading(false);
        }
      } catch (error) {
        setHasAttemptedFetch(true);
        // Only set pricing failed if we're not dealing with a known free provider
        const freeProviders = ['ollama', 'local', 'localhost'];
        if (!freeProviders.includes(currentProvider.toLowerCase())) {
          setPricingFailed(true);
          setModelNotFound(false);
        }
        setIsLoading(false);
      }
    };

    loadCostInfo();
  }, [currentModel, currentProvider]);

  // Return null early if pricing is disabled
  if (!showPricing) {
    return null;
  }

  const calculateCost = (): number => {
    // If we have session costs, calculate the total across all models
    if (sessionCosts) {
      let totalCost = 0;

      // Add up all historical costs from different models
      Object.values(sessionCosts).forEach((modelCost) => {
        totalCost += modelCost.totalCost;
      });

      // Add current model cost if we have pricing info
      if (
        costInfo &&
        (costInfo.input_token_cost !== undefined || costInfo.output_token_cost !== undefined)
      ) {
        const currentInputCost = inputTokens * (costInfo.input_token_cost || 0);
        const currentOutputCost = outputTokens * (costInfo.output_token_cost || 0);
        totalCost += currentInputCost + currentOutputCost;
      }

      return totalCost;
    }

    // Fallback to simple calculation for current model only
    if (
      !costInfo ||
      (costInfo.input_token_cost === undefined && costInfo.output_token_cost === undefined)
    ) {
      return 0;
    }

    const inputCost = inputTokens * (costInfo.input_token_cost || 0);
    const outputCost = outputTokens * (costInfo.output_token_cost || 0);
    const total = inputCost + outputCost;

    return total;
  };

  const formatCost = (cost: number): string => {
    // Always show 6 decimal places for consistency
    return cost.toFixed(6);
  };

  // Show loading state or when we don't have model/provider info
  if (!currentModel || !currentProvider) {
    return null;
  }

  // If still loading, show a placeholder
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full text-textSubtle translate-y-[1px]">
        <span className="text-xs font-mono">...</span>
      </div>
    );
  }

  // If no cost info found, try to return a default
  if (
    !costInfo ||
    (costInfo.input_token_cost === undefined && costInfo.output_token_cost === undefined)
  ) {
    // If it's a known free/local provider, show $0.000000 without "not available" message
    const freeProviders = ['ollama', 'local', 'localhost'];
    if (freeProviders.includes(currentProvider.toLowerCase())) {
      return (
        <div
          className="flex items-center justify-center h-full text-textSubtle hover:text-textStandard transition-colors cursor-default translate-y-[1px]"
          title={`Local model (${inputTokens.toLocaleString()} input, ${outputTokens.toLocaleString()} output tokens)`}
        >
          <span className="text-xs font-mono">$0.000000</span>
        </div>
      );
    }

    // Otherwise show as unavailable
    const getUnavailableTooltip = () => {
      if (pricingFailed && hasAttemptedFetch && initialLoadComplete) {
        return `Pricing data unavailable - OpenRouter connection failed. Click refresh in settings to retry.`;
      }
      // If we reach here, it must be modelNotFound (since we only get here after attempting fetch)
      return `Cost data not available for ${currentModel} (${inputTokens.toLocaleString()} input, ${outputTokens.toLocaleString()} output tokens)`;
    };

    return (
      <div
        className={`flex items-center justify-center h-full transition-colors cursor-default translate-y-[1px] ${
          (pricingFailed || modelNotFound) && hasAttemptedFetch && initialLoadComplete
            ? 'text-red-500 hover:text-red-400'
            : 'text-textSubtle hover:text-textStandard'
        }`}
        title={getUnavailableTooltip()}
      >
        <span className="text-xs font-mono">$0.000000</span>
      </div>
    );
  }

  const totalCost = calculateCost();

  // Build tooltip content
  const getTooltipContent = (): string => {
    // Handle error states first
    if (pricingFailed && hasAttemptedFetch && initialLoadComplete) {
      return `Pricing data unavailable - OpenRouter connection failed. Click refresh in settings to retry.`;
    }

    if (modelNotFound && hasAttemptedFetch && initialLoadComplete) {
      return `Pricing not available for ${currentProvider}/${currentModel}. This model may not be supported by the pricing service.`;
    }

    // Handle session costs
    if (sessionCosts && Object.keys(sessionCosts).length > 0) {
      // Show session breakdown
      let tooltip = 'Session cost breakdown:\n';

      Object.entries(sessionCosts).forEach(([modelKey, cost]) => {
        const costStr = `${costInfo?.currency || '$'}${cost.totalCost.toFixed(6)}`;
        tooltip += `${modelKey}: ${costStr} (${cost.inputTokens.toLocaleString()} in, ${cost.outputTokens.toLocaleString()} out)\n`;
      });

      // Add current model if it has costs
      if (costInfo && (inputTokens > 0 || outputTokens > 0)) {
        const currentCost =
          inputTokens * (costInfo.input_token_cost || 0) +
          outputTokens * (costInfo.output_token_cost || 0);
        if (currentCost > 0) {
          tooltip += `${currentProvider}/${currentModel} (current): ${costInfo.currency || '$'}${currentCost.toFixed(6)} (${inputTokens.toLocaleString()} in, ${outputTokens.toLocaleString()} out)\n`;
        }
      }

      tooltip += `\nTotal session cost: ${costInfo?.currency || '$'}${totalCost.toFixed(6)}`;
      return tooltip;
    }

    // Default tooltip for single model
    return `Input: ${inputTokens.toLocaleString()} tokens (${costInfo?.currency || '$'}${(inputTokens * (costInfo?.input_token_cost || 0)).toFixed(6)}) | Output: ${outputTokens.toLocaleString()} tokens (${costInfo?.currency || '$'}${(outputTokens * (costInfo?.output_token_cost || 0)).toFixed(6)})`;
  };

  return (
    <div
      className={`flex items-center justify-center h-full transition-colors cursor-default translate-y-[1px] ${
        (pricingFailed || modelNotFound) && hasAttemptedFetch && initialLoadComplete
          ? 'text-red-500 hover:text-red-400'
          : 'text-textSubtle hover:text-textStandard'
      }`}
      title={getTooltipContent()}
    >
      <span className="text-xs font-mono">
        {costInfo.currency || '$'}
        {formatCost(totalCost)}
      </span>
    </div>
  );
}
