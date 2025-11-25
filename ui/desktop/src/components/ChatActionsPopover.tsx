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
            className="flex items-center text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors !px-0"
          >
            <Workflow className={`w-4 h-4 ${shouldShowIconOnly ? '' : 'mr-1'}`} />
            {!shouldShowIconOnly && <span className="text-xs">Tools</span>}
          </Button>
        </TooltipTrigger>
        <TooltipContent>Actions & Files</TooltipContent>
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
              <h3 className="text-sm font-semibold text-text-default">Tools</h3>
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

            {/* Directory Switcher */}
            <div className="space-y-2">
              <label className="text-xs font-medium text-text-muted">Working Directory</label>
              <DirSwitcher shouldShowIconOnly={false} />
            </div>

            {/* Actions */}
            <div className="space-y-2 pt-2 border-t border-border-default">
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
