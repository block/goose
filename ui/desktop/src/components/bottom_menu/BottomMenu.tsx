import React, { useState, useEffect, useRef } from 'react';
import { useModel } from '../settings/models/ModelContext';
import { Sliders } from 'lucide-react';
import { AlertType, useAlerts } from '../alerts';
import { useToolCount } from '../alerts/useToolCount';
import BottomMenuAlertPopover from './BottomMenuAlertPopover';
import { ModelRadioList } from '../settings/models/ModelRadioList';
import { Document, ChevronUp, ChevronDown } from '../icons';
import type { View, ViewOptions } from '../../App';
import { settingsV2Enabled } from '../../flags';
import { BottomMenuModeSelection } from './BottomMenuModeSelection';
import ModelsBottomBar from '../settings_v2/models/bottom_bar/ModelsBottomBar';
import { useConfig } from '../ConfigContext';
import { getCurrentModelAndProvider } from '../settings_v2/models/index';

const TOKEN_LIMIT_DEFAULT = 128000; // fallback for custom models that the backend doesn't know about
const TOKEN_WARNING_THRESHOLD = 0.8; // warning shows at 80% of the token limit
const TOOLS_MAX_SUGGESTED = 25; // max number of tools before we show a warning

export default function BottomMenu({
  hasMessages,
  setView,
  numTokens = 0,
}: {
  hasMessages: boolean;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  numTokens?: number;
}) {
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const { currentModel } = useModel();
  const { alerts, addAlert, clearAlerts } = useAlerts();
  const dropdownRef = useRef<HTMLDivElement>(null);
  const toolCount = useToolCount();
  const { getProviders, read } = useConfig();
  const [tokenLimit, setTokenLimit] = useState<number>(TOKEN_LIMIT_DEFAULT);

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
        // Find the model's token limit
        const modelConfig = currentProvider.metadata.known_models.find((m) => m.name === model);
        if (modelConfig?.context_limit) {
          setTokenLimit(modelConfig.context_limit);
        }
      }
    } catch (err) {
      console.error('Error loading providers or token limit:', err);
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
        addAlert(
          AlertType.Error,
          `Token limit reached (${numTokens.toLocaleString()}/${tokenLimit.toLocaleString()})`
        );
      } else if (numTokens >= tokenLimit * TOKEN_WARNING_THRESHOLD) {
        addAlert(
          AlertType.Warning,
          `Approaching token limit (${numTokens.toLocaleString()}/${tokenLimit.toLocaleString()})`
        );
      }
    }

    // Add tool count alert if we have the data
    if (toolCount !== null && toolCount > TOOLS_MAX_SUGGESTED) {
      addAlert(
        AlertType.Warning,
        `Too many tools can degrade performance.\nTool count: ${toolCount} (recommend: ${TOOLS_MAX_SUGGESTED})`,
        {
          text: 'View extensions',
          onClick: () => setView('settings'),
        }
      );
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
    <div className="flex justify-between items-center text-textSubtle relative bg-bgSubtle border-t border-borderSubtle text-xs pl-4 h-[40px] pb-1 align-middle">
      {/* Directory Chooser - Always visible */}
      <span
        className="cursor-pointer flex items-center [&>svg]:size-4"
        onClick={async () => {
          if (hasMessages) {
            window.electron.directoryChooser();
          } else {
            window.electron.directoryChooser(true);
          }
        }}
      >
        <Document className="mr-1" />
        Working in {window.appConfig.get('GOOSE_WORKING_DIR')}
        <ChevronUp className="ml-1" />
      </span>

      {/* Goose Mode Selector Dropdown */}
      <BottomMenuModeSelection setView={setView} />

      {/* Right-side section with ToolCount and Model Selector together */}
      <div className="flex items-center mr-4 space-x-1">
        {/* Tool and Token count */}
        {<BottomMenuAlertPopover alerts={alerts} />}
        {/* Model Selector Dropdown */}
        {settingsV2Enabled ? (
          <ModelsBottomBar dropdownRef={dropdownRef} setView={setView} />
        ) : (
          <div className="relative flex items-center ml-0 mr-4" ref={dropdownRef}>
            <div
              className="flex items-center cursor-pointer"
              onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
            >
              <span>{(currentModel?.alias ?? currentModel?.name) || 'Select Model'}</span>
              {isModelMenuOpen ? (
                <ChevronDown className="w-4 h-4 ml-1" />
              ) : (
                <ChevronUp className="w-4 h-4 ml-1" />
              )}
            </div>

            {/* Dropdown Menu */}
            {isModelMenuOpen && (
              <div className="absolute bottom-[24px] right-0 w-[300px] bg-bgApp rounded-lg border border-borderSubtle">
                <div className="">
                  <ModelRadioList
                    className="divide-y divide-borderSubtle"
                    renderItem={({ model, isSelected, onSelect }) => (
                      <label key={model.alias ?? model.name} className="block cursor-pointer">
                        <div
                          className="flex items-center justify-between p-2 text-textStandard hover:bg-bgSubtle transition-colors"
                          onClick={onSelect}
                        >
                          <div>
                            <p className="text-sm ">{model.alias ?? model.name}</p>
                            <p className="text-xs text-textSubtle">
                              {model.subtext ?? model.provider}
                            </p>
                          </div>
                          <div className="relative">
                            <input
                              type="radio"
                              name="recentModels"
                              value={model.name}
                              checked={isSelected}
                              onChange={onSelect}
                              className="peer sr-only"
                            />
                            <div
                              className="h-4 w-4 rounded-full border border-gray-400 dark:border-gray-500
                          peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                          peer-checked:bg-white dark:peer-checked:bg-black
                          transition-all duration-200 ease-in-out"
                            ></div>
                          </div>
                        </div>
                      </label>
                    )}
                  />
                  <div
                    className="flex items-center justify-between text-textStandard p-2 cursor-pointer hover:bg-bgStandard
                  border-t border-borderSubtle mt-2"
                    onClick={() => {
                      setIsModelMenuOpen(false);
                      setView('settings');
                    }}
                  >
                    <span className="text-sm">Tools and Settings</span>
                    <Sliders className="w-5 h-5 ml-2 rotate-90" />
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
