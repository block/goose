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
            className="w-8 h-8 rounded-full bg-transparent text-zinc-400 dark:text-zinc-500 hover:bg-zinc-100 dark:hover:bg-zinc-800 flex items-center justify-center transition-colors"
          >
            <Settings className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Chat Settings</TooltipContent>
      </Tooltip>

      {isOpen && (
        <div
          ref={popoverRef}
          className="fixed z-50 bg-white/95 dark:bg-neutral-900/95 backdrop-blur-xl border border-black/10 dark:border-white/10 rounded-[32px] shadow-[0px_8px_24px_rgba(0,0,0,0.12)] dark:shadow-[0px_8px_24px_rgba(0,0,0,0.4)] min-w-80 max-w-md animate-in fade-in zoom-in-95 slide-in-from-bottom-2 duration-200"
          style={{
            left: '50%',
            bottom: '120px',
            transform: 'translateX(-50%)',
          }}
        >
          <div className="p-4 space-y-1" ref={dropdownRef}>
            {/* Model Selection */}
            <div className="flex items-center justify-between w-full px-3 py-2 hover:bg-zinc-50 dark:hover:bg-zinc-800/50 rounded-lg transition-colors">
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

            {/* Mode Selection */}
            <div className="flex items-center justify-between w-full px-3 py-2 hover:bg-zinc-50 dark:hover:bg-zinc-800/50 rounded-lg transition-colors">
              <BottomMenuModeSelection shouldShowIconOnly={false} />
            </div>

            {/* Cost Tracker */}
            {COST_TRACKING_ENABLED && (
              <div className="flex items-center justify-between w-full px-3 py-2 hover:bg-zinc-50 dark:hover:bg-zinc-800/50 rounded-lg transition-colors [&>div:last-child]:hidden">
                <CostTracker
                  inputTokens={inputTokens}
                  outputTokens={outputTokens}
                  sessionCosts={sessionCosts}
                  shouldShowIconOnly={false}
                />
              </div>
            )}

            {/* Hints Button */}
            <Button
              type="button"
              onClick={handleHintsClick}
              variant="ghost"
              size="sm"
              className="w-full flex items-center justify-start text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors px-3 py-2"
            >
              <FolderKey size={16} className="mr-2" />
              <span className="text-xs">Configure Goosehints</span>
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
