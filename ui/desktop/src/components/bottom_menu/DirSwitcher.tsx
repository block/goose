import React, { useState } from 'react';
import { FolderDot } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/Tooltip';

interface DirSwitcherProps {
  className?: string;
  shouldShowIconOnly?: boolean;
}

export const DirSwitcher: React.FC<DirSwitcherProps> = ({ className = '', shouldShowIconOnly = false }) => {
  const [isTooltipOpen, setIsTooltipOpen] = useState(false);

  const handleDirectoryChange = async () => {
    try {
      const result = await window.electron.directoryChooser(true);
      const selectedPath = result?.filePaths?.[0];
      const fallbackPath = window.appConfig.get('GOOSE_WORKING_DIR') as string;
      const resolvedPath = selectedPath || fallbackPath;

      if (resolvedPath) {
        window.dispatchEvent(
          new CustomEvent('goose-working-dir-changed', {
            detail: { path: resolvedPath },
          })
        );
      }
    } catch (error) {
      console.error('Failed to change working directory:', error);
    }
  };

  const handleDirectoryClick = async (event: React.MouseEvent) => {
    const isCmdOrCtrlClick = event.metaKey || event.ctrlKey;

    if (isCmdOrCtrlClick) {
      event.preventDefault();
      event.stopPropagation();
      const workingDir = window.appConfig.get('GOOSE_WORKING_DIR') as string;
      await window.electron.openDirectoryInExplorer(workingDir);
    } else {
      await handleDirectoryChange();
    }
  };

  return (
    <TooltipProvider>
      <Tooltip open={isTooltipOpen} onOpenChange={setIsTooltipOpen}>
        <TooltipTrigger asChild>
          <button
            className={`z-[100] hover:cursor-pointer text-text-default/70 hover:text-text-default text-xs flex items-center transition-colors px-1 [&>svg]:size-4 ${className}`}
            onClick={handleDirectoryClick}
          >
            <FolderDot className={shouldShowIconOnly ? '' : 'mr-1'} size={16} />
            <div className={`max-w-[200px] truncate [direction:rtl] ${shouldShowIconOnly ? 'hidden' : 'block'}`}>
              {String(window.appConfig.get('GOOSE_WORKING_DIR'))}
            </div>
          </button>
        </TooltipTrigger>
        <TooltipContent side="top">
          {window.appConfig.get('GOOSE_WORKING_DIR') as string}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
};
