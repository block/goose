import React, { useState } from 'react';
import { FolderDot } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/Tooltip';

interface DirSwitcherProps {
  className?: string;
}

export const DirSwitcher: React.FC<DirSwitcherProps> = ({ className = '' }) => {
  const [isTooltipOpen, setIsTooltipOpen] = useState(false);
  const [isDirectoryChooserOpen, setIsDirectoryChooserOpen] = useState(false);

  const handleDirectoryChange = async () => {
    if (isDirectoryChooserOpen) return;
    setIsDirectoryChooserOpen(true);
    try {
      await window.electron.directoryChooser(true);
    } finally {
      setIsDirectoryChooserOpen(false);
    }
  };

  const handleDirectoryClick = async (event: React.MouseEvent) => {
    if (isDirectoryChooserOpen) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
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
      <Tooltip
        open={isTooltipOpen && !isDirectoryChooserOpen}
        onOpenChange={(open) => {
          if (!isDirectoryChooserOpen) setIsTooltipOpen(open);
        }}
      >
        <TooltipTrigger asChild>
          <button
            className={`z-[100] ${isDirectoryChooserOpen ? 'opacity-50' : 'hover:cursor-pointer hover:text-text-default'} text-text-default/70 text-xs flex items-center transition-colors pl-1 [&>svg]:size-4 ${className}`}
            onClick={handleDirectoryClick}
            disabled={isDirectoryChooserOpen}
          >
            <FolderDot className="mr-1" size={16} />
            <div className="max-w-[200px] truncate [direction:rtl]">
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
