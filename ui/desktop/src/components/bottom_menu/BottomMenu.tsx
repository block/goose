import { useState, useEffect, useRef } from 'react';
import { AlertType, useAlerts } from '../alerts';
import { useToolCount } from '../alerts/useToolCount';
import BottomMenuAlertPopover from './BottomMenuAlertPopover';
import type { View, ViewOptions } from '../../App';
import { BottomMenuModeSelection } from './BottomMenuModeSelection';
import ModelsBottomBar from '../settings/models/bottom_bar/ModelsBottomBar';
import { useConfig } from '../ConfigContext';
import { useModelAndProvider } from '../ModelAndProviderContext';
import { Message } from '../../types/message';
import { ManualSummarizeButton } from '../context_management/ManualSummaryButton';
import { CostTracker } from './CostTracker';
import { COST_TRACKING_ENABLED } from '../../updates';

const TOKEN_LIMIT_DEFAULT = 128000; // fallback for custom models that the backend doesn't know about
const TOKEN_WARNING_THRESHOLD = 0.8; // warning shows at 80% of the token limit
const TOOLS_MAX_SUGGESTED = 60; // max number of tools before we show a warning

interface ModelLimit {
  pattern: string;
  context_limit: number;
}

export default function BottomMenu({
  setView,
  numTokens = 0,
  inputTokens = 0,
  outputTokens = 0,
  messages = [],
  isLoading = false,
  setMessages,
  sessionCosts,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  numTokens?: number;
  inputTokens?: number;
  outputTokens?: number;
  messages?: Message[];
  isLoading?: boolean;
  setMessages: (messages: Message[]) => void;
  sessionCosts?: {
    [key: string]: {
      inputTokens: number;
      outputTokens: number;
      totalCost: number;
    };
  };
}) {
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const { alerts, addAlert, clearAlerts } = useAlerts();
  const dropdownRef = useRef<HTMLDivElement>(null);
  const toolCount = useToolCount();
  const { getProviders, read } = useConfig();
  const { getCurrentModelAndProvider, currentModel, currentProvider } = useModelAndProvider();
  const [tokenLimit, setTokenLimit] = useState<number>(TOKEN_LIMIT_DEFAULT);
  const [isTokenLimitLoaded, setIsTokenLimitLoaded] = useState(false);

  // Load model limits from the API
  const getModelLimits = async () => {
    try {
      const response = await read('model-limits', false);
      if (response) {
        // The response is already parsed, no need for JSON.parse
        return response as ModelLimit[];
      }
    } catch (err) {
      console.error('Error fetching model limits:', err);
    }
    return [];
  };

  // Helper function to find model limit using pattern matching
  const findModelLimit = (modelName: string, modelLimits: ModelLimit[]): number | null => {
    if (!modelName) return null;
    const matchingLimit = modelLimits.find((limit) =>
      modelName.toLowerCase().includes(limit.pattern.toLowerCase())
    );
    return matchingLimit ? matchingLimit.context_limit : null;
  };

  // Load providers and get current model's token limit
  const loadProviderDetails = async () => {
    try {
      // Reset token limit loaded state
      setIsTokenLimitLoaded(false);

      // Get current model and provider first to avoid unnecessary provider fetches
      const { model, provider } = await getCurrentModelAndProvider();
      if (!model || !provider) {
        console.log('No model or provider found');
        setIsTokenLimitLoaded(true);
        return;
      }

      const providers = await getProviders(true);

      // Find the provider details for the current provider
      const currentProvider = providers.find((p) => p.name === provider);
      if (currentProvider?.metadata?.known_models) {
        // Find the model's token limit from the backend response
        const modelConfig = currentProvider.metadata.known_models.find((m) => m.name === model);
        if (modelConfig?.context_limit) {
          setTokenLimit(modelConfig.context_limit);
          setIsTokenLimitLoaded(true);
          return;
        }
      }

      // Fallback: Use pattern matching logic if no exact model match was found
      const modelLimit = await getModelLimits();
      const fallbackLimit = findModelLimit(model as string, modelLimit);
      if (fallbackLimit !== null) {
        setTokenLimit(fallbackLimit);
        setIsTokenLimitLoaded(true);
        return;
      }

      // If no match found, use the default model limit
      setTokenLimit(TOKEN_LIMIT_DEFAULT);
      setIsTokenLimitLoaded(true);
    } catch (err) {
      console.error('Error loading providers or token limit:', err);
      // Set default limit on error
      setTokenLimit(TOKEN_LIMIT_DEFAULT);
      setIsTokenLimitLoaded(true);
    }
  };

  // Initial load and refresh when model changes
  useEffect(() => {
    loadProviderDetails();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentModel, currentProvider]);

  // Handle tool count alerts and token usage
  useEffect(() => {
    clearAlerts();

    // Only show token alerts if we have loaded the real token limit
    if (isTokenLimitLoaded && tokenLimit && numTokens > 0) {
      if (numTokens >= tokenLimit) {
        // Only show error alert when limit reached
        addAlert({
          type: AlertType.Error,
          message: `Token limit reached (${numTokens.toLocaleString()}/${tokenLimit.toLocaleString()}) \n You've reached the model's conversation limit. The session will be saved — copy anything important and start a new one to continue.`,
          autoShow: true, // Auto-show token limit errors
        });
      } else if (numTokens >= tokenLimit * TOKEN_WARNING_THRESHOLD) {
        // Only show warning alert when approaching limit
        addAlert({
          type: AlertType.Warning,
          message: `Approaching token limit (${numTokens.toLocaleString()}/${tokenLimit.toLocaleString()}) \n You're reaching the model's conversation limit. The session will be saved — copy anything important and start a new one to continue.`,
          autoShow: true, // Auto-show token limit warnings
        });
      } else {
        // Show info alert only when not in warning/error state
        addAlert({
          type: AlertType.Info,
          message: 'Context window',
          progress: {
            current: numTokens,
            total: tokenLimit,
          },
        });
      }
    }

    // Add tool count alert if we have the data
    if (toolCount !== null && toolCount > TOOLS_MAX_SUGGESTED) {
      addAlert({
        type: AlertType.Warning,
        message: `Too many tools can degrade performance.\nTool count: ${toolCount} (recommend: ${TOOLS_MAX_SUGGESTED})`,
        action: {
          text: 'View extensions',
          onClick: () => setView('settings'),
        },
        autoShow: false, // Don't auto-show tool count warnings
      });
    }
    // We intentionally omit setView as it shouldn't trigger a re-render of alerts
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [numTokens, toolCount, tokenLimit, isTokenLimitLoaded, addAlert, clearAlerts]);

  // Add effect to handle clicks outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsModelMenuOpen(false);
      }
    };

    if (isModelMenuOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isModelMenuOpen]);

  // Add effect to handle Escape key
  useEffect(() => {
    const handleEsc = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsModelMenuOpen(false);
      }
    };

    if (isModelMenuOpen) {
      window.addEventListener('keydown', handleEsc);
    }

    return () => {
      window.removeEventListener('keydown', handleEsc);
    };
  }, [isModelMenuOpen]);

  return (
    <div className="flex justify-between items-center transition-colors text-textSubtle relative text-xs h-6">
      <div className="flex items-center h-full">
        {/* Tool and Token count */}
        <div className="flex items-center h-full pl-2">
          {<BottomMenuAlertPopover alerts={alerts} />}
        </div>

        {/* Cost Tracker - no separator before it */}
        {COST_TRACKING_ENABLED && (
          <>
            <div className="flex items-center h-full ml-1">
              <CostTracker
                inputTokens={inputTokens}
                outputTokens={outputTokens}
                sessionCosts={sessionCosts}
              />
            </div>
            <div className="w-[1px] h-4 bg-borderSubtle mx-1.5" />
          </>
        )}

        {/* Model Selector Dropdown */}
        <div className="flex items-center h-full">
          <ModelsBottomBar dropdownRef={dropdownRef} setView={setView} />
        </div>

        {/* Separator */}
        <div className="w-[1px] h-4 bg-borderSubtle mx-1.5" />

        {/* Goose Mode Selector Dropdown */}
        <div className="flex items-center h-full">
          <BottomMenuModeSelection setView={setView} />
        </div>

        {/* Summarize Context Button */}
        {messages.length > 0 && (
          <>
            <div className="w-[1px] h-4 bg-borderSubtle mx-1.5" />
            <div className="flex items-center h-full">
              <ManualSummarizeButton
                messages={messages}
                isLoading={isLoading}
                setMessages={setMessages}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
