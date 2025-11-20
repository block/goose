import { useEffect, useRef, useState } from 'react';
import { useModelAndProvider } from '../components/ModelAndProviderContext';
import { getCostForModel } from '../utils/costDatabase';
import { Session } from '../api';

interface UseCostTrackingProps {
  sessionInputTokens: number;
  sessionOutputTokens: number;
  sessionCacheReadTokens: number;
  sessionCacheWriteTokens: number;
  localInputTokens: number;
  localOutputTokens: number;
  localCacheReadTokens: number;
  localCacheWriteTokens: number;
  session?: Session | null;
}

export const useCostTracking = ({
  sessionInputTokens,
  sessionOutputTokens,
  sessionCacheReadTokens,
  sessionCacheWriteTokens,
  localInputTokens,
  localOutputTokens,
  localCacheReadTokens,
  localCacheWriteTokens,
  session,
}: UseCostTrackingProps) => {
  const [sessionCosts, setSessionCosts] = useState<{
    [key: string]: {
      inputTokens: number;
      outputTokens: number;
      cacheReadTokens?: number;
      cacheWriteTokens?: number;
      totalCost: number;
    };
  }>({});

  const { currentModel, currentProvider } = useModelAndProvider();
  const prevModelRef = useRef<string | undefined>(undefined);
  const prevProviderRef = useRef<string | undefined>(undefined);

  // Handle model changes and accumulate costs
  useEffect(() => {
    if (
      prevModelRef.current !== undefined &&
      prevProviderRef.current !== undefined &&
      (prevModelRef.current !== currentModel || prevProviderRef.current !== currentProvider)
    ) {
      // Model/provider has changed, save the costs for the previous model
      const prevKey = `${prevProviderRef.current}/${prevModelRef.current}`;

      // Get pricing info for the previous model
      const prevCostInfo = getCostForModel(prevProviderRef.current, prevModelRef.current);

      if (prevCostInfo) {
        const prevInputCost =
          (sessionInputTokens || localInputTokens) * (prevCostInfo.input_token_cost || 0);
        const prevOutputCost =
          (sessionOutputTokens || localOutputTokens) * (prevCostInfo.output_token_cost || 0);
        const prevCacheReadCost =
          (sessionCacheReadTokens || localCacheReadTokens) *
          (prevCostInfo.cache_read_input_token_cost || 0);
        const prevCacheWriteCost =
          (sessionCacheWriteTokens || localCacheWriteTokens) *
          (prevCostInfo.cache_write_input_token_cost || 0);
        const prevTotalCost =
          prevInputCost + prevOutputCost + prevCacheReadCost + prevCacheWriteCost;

        // Save the accumulated costs for this model
        setSessionCosts((prev) => ({
          ...prev,
          [prevKey]: {
            inputTokens: sessionInputTokens || localInputTokens,
            outputTokens: sessionOutputTokens || localOutputTokens,
            cacheReadTokens: sessionCacheReadTokens || localCacheReadTokens,
            cacheWriteTokens: sessionCacheWriteTokens || localCacheWriteTokens,
            totalCost: prevTotalCost,
          },
        }));
      }

      console.log(
        'Model changed from',
        `${prevProviderRef.current}/${prevModelRef.current}`,
        'to',
        `${currentProvider}/${currentModel}`,
        '- saved costs and restored session token counters'
      );
    }

    prevModelRef.current = currentModel || undefined;
    prevProviderRef.current = currentProvider || undefined;
  }, [
    currentModel,
    currentProvider,
    sessionInputTokens,
    sessionOutputTokens,
    sessionCacheReadTokens,
    sessionCacheWriteTokens,
    localInputTokens,
    localOutputTokens,
    localCacheReadTokens,
    localCacheWriteTokens,
    session,
  ]);

  return {
    sessionCosts,
  };
};
