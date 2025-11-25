import React, { useState, useRef } from 'react';
import { Settings } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import ModelsBottomBar from './settings/models/bottom_bar/ModelsBottomBar';
import { BottomMenuModeSelection } from './bottom_menu/BottomMenuModeSelection';
import { CostTracker } from './bottom_menu/CostTracker';
import { FolderKey } from 'lucide-react';
import { AlertType } from './alerts';
import type { View } from '../utils/navigationUtils';
import { Recipe } from '../recipe';
import { COST_TRACKING_ENABLED } from '../updates';

interface ChatSettingsPopoverProps {
  sessionId: string | null;
  setView: (view: View) => void;
  alerts: Array<{ type: AlertType; message: string }>;
  recipeConfig?: Recipe | null;
  hasMessages: boolean;
  shouldShowIconOnly: boolean;
  inputTokens?: number;
  outputTokens?: number;
  sessionCosts?: {
    [key: string]: {
      inputTokens: number;
      outputTokens: number;
      totalCost: number;
    };
  };
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}

export function ChatSettingsPopover({
  sessionId,
  setView,
  alerts,
  recipeConfig,
  hasMessages,
  shouldShowIconOnly,
  inputTokens,
  outputTokens,
  sessionCosts,
  setIsGoosehintsModalOpen,
}: ChatSettingsPopoverProps) {
  const [isOpen, setIsOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Close popover when clicking outside
  React.useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        isOpen &&
        popoverRef.current &&
        buttonRef.current &&
        !popoverRef.current.contains(event.target as Node) &&
        !buttonRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [isOpen]);

  const handleHintsClick = () => {
    setIsGoosehintsModalOpen?.(true);
    setIsOpen(false);
  };

  return (
    <div className="relative">
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            ref={buttonRef}
            type="button"
            onClick={() => setIsOpen(!isOpen)}
            variant="ghost"
            size="sm"
            className={`flex items-center gap-1.5 text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-all hover:bg-bgSubtle ${
              shouldShowIconOnly 
                ? 'rounded-full p-2' 
                : 'rounded-full px-3 py-1.5 border border-border-default'
            }`}
          >
            <Settings className="w-4 h-4" />
            {!shouldShowIconOnly && <span className="text-xs font-medium">Settings</span>}
          </Button>
        </TooltipTrigger>
        <TooltipContent>Chat Settings</TooltipContent>
      </Tooltip>

      {isOpen && (
        <div
          ref={popoverRef}
          className="fixed z-50 bg-white/95 dark:bg-neutral-900/95 backdrop-blur-xl border border-black/10 dark:border-white/10 rounded-lg shadow-[0px_8px_24px_rgba(0,0,0,0.12)] dark:shadow-[0px_8px_24px_rgba(0,0,0,0.4)] min-w-80 max-w-md"
          style={{
            left: '50%',
            bottom: '120px',
            transform: 'translateX(-50%)',
          }}
        >
          <div className="p-4 space-y-4">
            {/* Header */}
            <div className="flex items-center justify-between border-b border-border-default pb-2">
              <h3 className="text-sm font-semibold text-text-default">Chat Settings</h3>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setIsOpen(false)}
                className="h-6 w-6 p-0"
              >
                ×
              </Button>
            </div>

            {/* Model Selection */}
            <div className="space-y-2">
              <label className="text-xs font-medium text-text-muted">Model</label>
              <div ref={dropdownRef}>
                <ModelsBottomBar
                  sessionId={sessionId}
                  dropdownRef={dropdownRef}
                  setView={setView}
                  alerts={alerts}
                  recipeConfig={recipeConfig}
                  hasMessages={hasMessages}
                  shouldShowIconOnly={false}
                />
              </div>
            </div>

            {/* Mode Selection */}
            <div className="space-y-2">
              <label className="text-xs font-medium text-text-muted">Mode</label>
              <BottomMenuModeSelection shouldShowIconOnly={false} />
            </div>

            {/* Cost Tracker */}
            {COST_TRACKING_ENABLED && (
              <div className="space-y-2">
                <label className="text-xs font-medium text-text-muted">Cost</label>
                <CostTracker
                  inputTokens={inputTokens}
                  outputTokens={outputTokens}
                  sessionCosts={sessionCosts}
                  shouldShowIconOnly={false}
                />
              </div>
            )}

            {/* Hints Button */}
            <div className="space-y-2 pt-2 border-t border-border-default">
              <Button
                type="button"
                onClick={handleHintsClick}
                variant="ghost"
                size="sm"
                className="w-full flex items-center justify-start text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors"
              >
                <FolderKey size={16} className="mr-2" />
                <span className="text-xs">Configure Goosehints</span>
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
