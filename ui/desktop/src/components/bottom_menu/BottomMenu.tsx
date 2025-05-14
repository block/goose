import { useState, useEffect, useRef } from 'react';
import { useModel } from '../settings/models/ModelContext';
import { AlertType, useAlerts } from '../alerts';
import { useToolCount } from '../alerts/useToolCount';
import BottomMenuAlertPopover from './BottomMenuAlertPopover';
import type { View, ViewOptions } from '../../App';
import { BottomMenuModeSelection } from './BottomMenuModeSelection';
import ModelsBottomBar from '../settings_v2/models/bottom_bar/ModelsBottomBar';
import { useConfig } from '../ConfigContext';
import { getCurrentModelAndProvider } from '../settings_v2/models';
import { Message } from '../../types/message';
import { ManualSummarizeButton } from '../context_management/ManualSummaryButton';

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
  messages = [],
  isLoading = false,
  setMessages,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  numTokens?: number;
  messages?: Message[];
  isLoading?: boolean;
  setMessages: (messages: Message[]) => void;
}) {
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const { currentModel } = useModel();
  const { alerts, addAlert, clearAlerts } = useAlerts();
  const dropdownRef = useRef<HTMLDivElement>(null);
  const toolCount = useToolCount();
  const { getProviders, read } = useConfig();
  const [tokenLimit, setTokenLimit] = useState<number>(TOKEN_LIMIT_DEFAULT);

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
      // Get current model and provider first to avoid unnecessary provider fetches
      const { model, provider } = await getCurrentModelAndProvider({ readFromConfig: read });
      if (!model || !provider) {
        console.log('No model or provider found');
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
          return;
        }
      }

      // Fallback: Use pattern matching logic if no exact model match was found
      const modelLimit = await getModelLimits();
      const fallbackLimit = findModelLimit(model as string, modelLimit);
      if (fallbackLimit !== null) {
        setTokenLimit(fallbackLimit);
        return;
      }

      // If no match found, use the default model limit
      setTokenLimit(TOKEN_LIMIT_DEFAULT);
    } catch (err) {
      console.error('Error loading providers or token limit:', err);
      // Set default limit on error
      setTokenLimit(TOKEN_LIMIT_DEFAULT);
    }
  };

  // Initial load and refresh when model changes
  useEffect(() => {
    loadProviderDetails();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentModel]);

  // Handle tool count alerts
  useEffect(() => {
    clearAlerts();

    // Add token alerts if we have a token limit
    if (tokenLimit && numTokens > 0) {
      if (numTokens >= tokenLimit) {
        addAlert({
          type: AlertType.Error,
          message: `Token limit reached (${numTokens.toLocaleString()}/${tokenLimit.toLocaleString()}) \n You’ve reached the model’s conversation limit. The session will be saved — copy anything important and start a new one to continue.`,
          autoShow: true, // Auto-show token limit errors
        });
      } else if (numTokens >= tokenLimit * TOKEN_WARNING_THRESHOLD) {
        addAlert({
          type: AlertType.Warning,
          message: `Approaching token limit (${numTokens.toLocaleString()}/${tokenLimit.toLocaleString()}) \n You’re reaching the model’s conversation limit. The session will be saved — copy anything important and start a new one to continue.`,
          autoShow: true, // Auto-show token limit warnings
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
  }, [numTokens, toolCount, tokenLimit, addAlert, clearAlerts]);

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
    <div className="flex justify-between items-center transition-colors text-textSubtle relative text-xs align-middle">
      <div className="flex items-center pl-2">
        {/* Tool and Token count */}
        {<BottomMenuAlertPopover alerts={alerts} />}

        {/* Model Selector Dropdown */}
        <ModelsBottomBar dropdownRef={dropdownRef} setView={setView} />

        {/* Separator */}
        <div className="w-[1px] h-4 bg-borderSubtle mx-2" />

        {/* Goose Mode Selector Dropdown */}
        <BottomMenuModeSelection setView={setView} />

        {/* Summarize Context Button - ADD THIS */}
        {messages.length > 0 && (
          <>
            <div className="w-[1px] h-4 bg-borderSubtle mx-2" />
            <ManualSummarizeButton
              messages={messages}
              isLoading={isLoading}
              setMessages={setMessages}
            />
          </>
        )}
      </div>
    </div>
  );
}
