import { useState, useEffect } from 'react';
import { CoinIcon } from '../icons';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/Tooltip';
import { fetchCanonicalModelInfo, type CanonicalModelInfo } from '../../utils/canonical';
import { defineMessages, useIntl } from '../../i18n';
import type { ProviderUsageEntry } from '../../types/chat';

const i18n = defineMessages({
  pricingUnavailable: {
    id: 'costTracker.pricingUnavailable',
    defaultMessage: 'Pricing data unavailable for {model}',
  },
  costUnavailable: {
    id: 'costTracker.costUnavailable',
    defaultMessage: 'Cost data not available for {model} ({inputTokens} input, {outputTokens} output tokens)',
  },
  totalSessionCost: {
    id: 'costTracker.totalSessionCost',
    defaultMessage: 'Total session cost: {cost}',
  },
  inputOutputTooltip: {
    id: 'costTracker.inputOutputTooltip',
    defaultMessage: 'Input: {inputTokens} tokens ({inputCost}) | Output: {outputTokens} tokens ({outputCost})',
  },
});

interface CostTrackerProps {
  inputTokens?: number;
  outputTokens?: number;
  accumulatedCost?: number | null;
  providerUsage?: ProviderUsageEntry[];
  model: string | null;
  provider: string | null;
}

export function CostTracker({
  inputTokens = 0,
  outputTokens = 0,
  accumulatedCost,
  providerUsage,
  model: currentModel,
  provider: currentProvider,
}: CostTrackerProps) {
  const intl = useIntl();
  const [costEstimate, setCostEstimate] = useState<CostEstimate | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [showPricing, setShowPricing] = useState(true);
  const [pricingFailed, setPricingFailed] = useState(false);

  // Check if pricing is enabled
  useEffect(() => {
    const loadPricingSetting = async () => {
      const enabled = await window.electron.getSetting('showPricing');
      setShowPricing(enabled);
    };

    loadPricingSetting();

    const handlePricingChange = () => {
      loadPricingSetting();
    };

    window.addEventListener('showPricingChanged', handlePricingChange);
    return () => window.removeEventListener('showPricingChanged', handlePricingChange);
  }, []);

  useEffect(() => {
    const loadCostInfo = async () => {
      if (!currentModel || !currentProvider) {
        setCostEstimate(null);
        setIsLoading(false);
        return;
      }

      if (!providerUsage?.length && accumulatedCost != null) {
        setCostEstimate(null);
        setPricingFailed(false);
        setIsLoading(false);
        return;
      }

      setIsLoading(true);
      try {
        const estimate = await calculateUsageCost(
          providerUsage?.length
            ? providerUsage
            : [
                {
                  providerId: currentProvider,
                  modelId: currentModel,
                  lastUsedAt: '',
                  inputTokens,
                  outputTokens,
                },
              ]
        );
        setCostEstimate(estimate);
        setPricingFailed(!estimate);
      } catch {
        setPricingFailed(true);
        setCostEstimate(null);
      } finally {
        setIsLoading(false);
      }
    };

    loadCostInfo();
  }, [currentModel, currentProvider, inputTokens, outputTokens, accumulatedCost, providerUsage]);

  // Return null early if pricing is disabled
  if (!showPricing) {
    return null;
  }

  const renderCost = (displayCost: string, tooltip: string) => (
    <Tooltip>
      <TooltipTrigger asChild>
        <div className="flex items-center justify-center h-full transition-colors cursor-default translate-y-[1px] text-text-primary/70 hover:text-text-primary">
          <CoinIcon className="mr-1" size={16} />
          <span className="text-xs font-mono">{displayCost}</span>
        </div>
      </TooltipTrigger>
      <TooltipContent>{tooltip}</TooltipContent>
    </Tooltip>
  );

  // Show loading state or when we don't have model/provider info
  if (!currentModel || !currentProvider) {
    return null;
  }

  // If still loading, show a placeholder
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full text-text-secondary translate-y-[1px]">
        <span className="text-xs font-mono">...</span>
      </div>
    );
  }

  if (accumulatedCost == null && !costEstimate) {
    const freeProviders = ['ollama', 'local', 'localhost'];
    if (freeProviders.includes(currentProvider.toLowerCase())) {
      return (
        <div className="flex items-center justify-center h-full text-text-primary/70 transition-colors cursor-default translate-y-[1px]">
          <span className="text-xs font-mono">
            {inputTokens.toLocaleString()}↑ {outputTokens.toLocaleString()}↓
          </span>
        </div>
      );
    }

    // Otherwise show as unavailable
    return renderCost(
      '0.0000',
      pricingFailed
        ? intl.formatMessage(i18n.pricingUnavailable, { model: currentModel })
        : intl.formatMessage(i18n.costUnavailable, {
            model: currentModel,
            inputTokens: inputTokens.toLocaleString(),
            outputTokens: outputTokens.toLocaleString(),
          })
    );
  }

  const totalCost = costEstimate ? formatCostTotals(costEstimate.total) : null;
  const displayCost =
    totalCost || (accumulatedCost == null ? '0.0000' : accumulatedCost.toFixed(2));

  // Build tooltip content
  const getTooltipContent = (): string => {
    if (pricingFailed && accumulatedCost == null) {
      return intl.formatMessage(i18n.pricingUnavailable, { model: `${currentProvider}/${currentModel}` });
    }

    if (costEstimate) {
      return intl.formatMessage(i18n.totalSessionCost, { cost: totalCost })
        + `\n` + intl.formatMessage(i18n.inputOutputTooltip, {
          inputTokens: costEstimate.inputTokens.toLocaleString(),
          inputCost: formatCostTotals(costEstimate.input),
          outputTokens: costEstimate.outputTokens.toLocaleString(),
          outputCost: formatCostTotals(costEstimate.output),
        });
    }

    return intl.formatMessage(i18n.totalSessionCost, { cost: displayCost })
      + `\n` + intl.formatMessage(i18n.inputOutputTooltip, {
        inputTokens: inputTokens.toLocaleString(),
        inputCost: 'unknown',
        outputTokens: outputTokens.toLocaleString(),
        outputCost: 'unknown',
      });
  };

  return renderCost(displayCost, getTooltipContent());
}

type CostEstimate = {
  total: Record<string, number>;
  input: Record<string, number>;
  output: Record<string, number>;
  inputTokens: number;
  outputTokens: number;
};

async function calculateUsageCost(entries: ProviderUsageEntry[]): Promise<CostEstimate | null> {
  const estimate: CostEstimate = {
    total: {},
    input: {},
    output: {},
    inputTokens: 0,
    outputTokens: 0,
  };

  for (const entry of entries) {
    const modelInfo = await fetchCanonicalModelInfo(entry.providerId, entry.modelId);
    const cost = calculateEntryCost(entry, modelInfo);
    if (!cost) {
      return null;
    }
    const currency = modelInfo?.currency || '$';
    estimate.inputTokens += entry.inputTokens;
    estimate.outputTokens += entry.outputTokens;
    addCost(estimate.input, currency, cost.input);
    addCost(estimate.output, currency, cost.output);
    addCost(estimate.total, currency, cost.input + cost.output);
  }

  return Object.keys(estimate.total).length ? estimate : null;
}

function calculateEntryCost(
  entry: ProviderUsageEntry,
  modelInfo: CanonicalModelInfo | null
): { input: number; output: number } | null {
  const inputTokenCost = modelInfo?.inputTokenCost;
  const outputTokenCost = modelInfo?.outputTokenCost;
  if (inputTokenCost == null || outputTokenCost == null) {
    return null;
  }

  const cacheReadTokens = entry.cacheReadInputTokens ?? 0;
  const cacheWriteTokens = entry.cacheWriteInputTokens ?? 0;
  const uncachedInputTokens = Math.max(
    0,
    entry.inputTokens - cacheReadTokens - cacheWriteTokens
  );
  const cacheReadCost = modelInfo?.cacheReadTokenCost ?? inputTokenCost;
  const cacheWriteCost = modelInfo?.cacheWriteTokenCost ?? inputTokenCost;

  const input =
    (uncachedInputTokens * inputTokenCost +
      cacheReadTokens * cacheReadCost +
      cacheWriteTokens * cacheWriteCost) /
    1_000_000;
  const output = (entry.outputTokens * outputTokenCost) / 1_000_000;

  return { input, output };
}

function addCost(totals: Record<string, number>, currency: string, amount: number): void {
  totals[currency] = (totals[currency] ?? 0) + amount;
}

function formatCostTotals(totals: Record<string, number>): string {
  const entries = Object.entries(totals);
  const displayEntries = entries.filter(([, amount]) => amount > 0);

  return (displayEntries.length ? displayEntries : entries.slice(0, 1))
    .map(([currency, amount]) => formatMoney(amount, currency))
    .join(' + ');
}

function formatMoney(amount: number, currency: string, digits = 2): string {
  const unit = currency.trim().toUpperCase();
  const value = amount.toFixed(digits);
  const symbol =
    {
      USD: '$',
      EUR: '€',
      GBP: '£',
      RUB: '₽',
    }[unit] ?? currency.trim();

  return /^[A-Z]{3}$/.test(symbol) ? `${value} ${symbol}` : `${symbol}${value}`;
}
