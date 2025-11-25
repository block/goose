import React, { useState, useRef } from 'react';
import { Workflow } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { DirSwitcher } from './bottom_menu/DirSwitcher';
import { Action, Attach } from './icons';

interface ChatActionsPopoverProps {
  shouldShowIconOnly: boolean;
  onActionButtonClick: (event: React.MouseEvent<HTMLButtonElement>) => void;
  onAttachClick: () => void;
}

export function ChatActionsPopover({
  shouldShowIconOnly,
  onActionButtonClick,
  onAttachClick,
}: ChatActionsPopoverProps) {
  const [isOpen, setIsOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

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

  const handleActionClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    setIsOpen(false);
    onActionButtonClick(e);
  };

  const handleAttachClickInternal = () => {
    setIsOpen(false);
    onAttachClick();
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
            <Workflow className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Actions & Files</TooltipContent>
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
          <div className="p-4">
            {/* All Actions */}
            <div className="space-y-1">
              <Button
                type="button"
                onClick={async () => {
                  await window.electron.directoryChooser(true);
                  setIsOpen(false);
                }}
                variant="ghost"
                size="sm"
                className="w-full flex items-center justify-start text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors"
              >
                <svg className="w-4 h-4 mr-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/>
                  <circle cx="12" cy="13" r="2"/>
                </svg>
                <span className="text-xs truncate max-w-[200px]">
                  {String(window.appConfig.get('GOOSE_WORKING_DIR'))}
                </span>
              </Button>

              <Button
                type="button"
                onClick={handleActionClick}
                variant="ghost"
                size="sm"
                className="w-full flex items-center justify-start text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors"
              >
                <Action className="w-4 h-4 mr-2" />
                <span className="text-xs">Quick Actions</span>
              </Button>

              <Button
                type="button"
                onClick={handleAttachClickInternal}
                variant="ghost"
                size="sm"
                className="w-full flex items-center justify-start text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors"
              >
                <Attach className="w-4 h-4 mr-2" />
                <span className="text-xs">Attach File or Directory</span>
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
