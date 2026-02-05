import { useEffect, useRef, useState } from 'react';
import { useModelAndProvider } from '../components/ModelAndProviderContext';
import { fetchModelPricing } from '../utils/pricing';
import { Session } from '../api';

interface UseCostTrackingProps {
  sessionInputTokens: number;
  sessionOutputTokens: number;
  localInputTokens: number;
  localOutputTokens: number;
  session?: Session | null;
}

export const useCostTracking = ({
  sessionInputTokens,
  sessionOutputTokens,
  localInputTokens,
  localOutputTokens,
  session,
}: UseCostTrackingProps) => {
  const [sessionCosts, setSessionCosts] = useState<{
    [key: string]: {
      inputTokens: number;
      outputTokens: number;
      totalCost: number;
    };
  }>({});

  const { currentModel, currentProvider } = useModelAndProvider();
  const prevModelRef = useRef<string | undefined>(undefined);
  const prevProviderRef = useRef<string | undefined>(undefined);

  // Handle model changes and accumulate costs
  useEffect(() => {
    const handleModelChange = async () => {
      if (
        prevModelRef.current !== undefined &&
        prevProviderRef.current !== undefined &&
        (prevModelRef.current !== currentModel || prevProviderRef.current !== currentProvider)
      ) {
        // Model/provider has changed, save the costs for the previous model
        const prevKey = `${prevProviderRef.current}/${prevModelRef.current}`;

        // Get pricing info for the previous model
        const prevCostInfo = await fetchModelPricing(prevProviderRef.current, prevModelRef.current);

        if (prevCostInfo) {
          const prevInputCost =
            (sessionInputTokens || localInputTokens) * (prevCostInfo.input_token_cost || 0);
          const prevOutputCost =
            (sessionOutputTokens || localOutputTokens) * (prevCostInfo.output_token_cost || 0);
          const prevTotalCost = prevInputCost + prevOutputCost;

          // Save the accumulated costs for this model
          setSessionCosts((prev) => ({
            ...prev,
            [prevKey]: {
              inputTokens: sessionInputTokens || localInputTokens,
              outputTokens: sessionOutputTokens || localOutputTokens,
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
    };

    handleModelChange();
  }, [
    currentModel,
    currentProvider,
    sessionInputTokens,
    sessionOutputTokens,
    localInputTokens,
    localOutputTokens,
    session,
  ]);

  return {
    sessionCosts,
  };
};
