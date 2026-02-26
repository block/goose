import { useEffect, useRef, useState } from 'react';
import type { Session } from '@/api';
import { useModelAndProvider } from '@/contexts/ModelAndProviderContext';
import { fetchModelPricing } from '@/utils/pricing';

type ModelKey = `${string}/${string}`;

interface UseCostTrackingProps {
  sessionInputTokens: number;
  sessionOutputTokens: number;
  localInputTokens: number;
  localOutputTokens: number;
  session?: Session | null;
}

type ModelSessionCost = {
  inputTokens: number;
  outputTokens: number;
  totalCost: number;
};

type PricingInfo = {
  input_token_cost?: number | null;
  output_token_cost?: number | null;
  currency?: string | null;
};

export const useCostTracking = ({
  sessionInputTokens,
  sessionOutputTokens,
  localInputTokens,
  localOutputTokens,
  session,
}: UseCostTrackingProps) => {
  const [sessionCosts, setSessionCosts] = useState<Record<ModelKey, ModelSessionCost>>({});

  const { currentModel, currentProvider } = useModelAndProvider();

  const pricingCacheRef = useRef<Map<ModelKey, PricingInfo | null>>(new Map());

  const sessionId = session?.id ?? null;
  const lastSessionIdRef = useRef<string | null>(null);

  const lastTokensRef = useRef<{ input: number; output: number }>({ input: 0, output: 0 });

  const isSessionBacked = sessionId !== null;
  const totalInputTokens = isSessionBacked ? sessionInputTokens : localInputTokens;
  const totalOutputTokens = isSessionBacked ? sessionOutputTokens : localOutputTokens;

  useEffect(() => {
    if (lastSessionIdRef.current !== sessionId) {
      lastSessionIdRef.current = sessionId;
      pricingCacheRef.current = new Map();
      lastTokensRef.current = { input: 0, output: 0 };
      setSessionCosts({});
    }
  }, [sessionId]);

  useEffect(() => {
    let cancelled = false;

    const updateCosts = async () => {
      if (!currentModel || !currentProvider) {
        // Don't advance the token snapshot until we know which model/provider to attribute costs to.
        return;
      }

      const currentKey = `${currentProvider}/${currentModel}` as ModelKey;
      const deltaInput = Math.max(0, totalInputTokens - lastTokensRef.current.input);
      const deltaOutput = Math.max(0, totalOutputTokens - lastTokensRef.current.output);

      // Always update the snapshot so we don't repeatedly attempt attribution.
      lastTokensRef.current = { input: totalInputTokens, output: totalOutputTokens };

      if (deltaInput === 0 && deltaOutput === 0) return;

      const cached = pricingCacheRef.current.get(currentKey);
      const pricing =
        cached !== undefined
          ? cached
          : await fetchModelPricing(currentProvider, currentModel).then((info) => {
              pricingCacheRef.current.set(currentKey, info);
              return info;
            });

      if (cancelled) return;
      if (!pricing) return;

      const inputRate = pricing.input_token_cost ?? 0;
      const outputRate = pricing.output_token_cost ?? 0;
      const deltaCost = deltaInput * inputRate + deltaOutput * outputRate;

      setSessionCosts((prev) => {
        const existing = prev[currentKey] ?? { inputTokens: 0, outputTokens: 0, totalCost: 0 };
        return {
          ...prev,
          [currentKey]: {
            inputTokens: existing.inputTokens + deltaInput,
            outputTokens: existing.outputTokens + deltaOutput,
            totalCost: existing.totalCost + deltaCost,
          },
        };
      });
    };

    void updateCosts();

    return () => {
      cancelled = true;
    };
  }, [
    currentModel,
    currentProvider,
    totalInputTokens,
    totalOutputTokens,
  ]);

  return {
    sessionCosts,
  };
};
