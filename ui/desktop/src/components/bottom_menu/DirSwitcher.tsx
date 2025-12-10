import React, { useState, useEffect } from 'react';
import { FolderDot } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/Tooltip';
import { updateSessionWorkingDir, restartAgent, getSession } from '../../api';
import { toast } from 'react-toastify';
import { setWorkingDir, getWorkingDir } from '../../store/newChatState';

interface DirSwitcherProps {
  className: string;
  sessionId: string | undefined;
}

export const DirSwitcher: React.FC<DirSwitcherProps> = ({ className, sessionId }) => {
  const [isTooltipOpen, setIsTooltipOpen] = useState(false);
  const [isDirectoryChooserOpen, setIsDirectoryChooserOpen] = useState(false);

  const [sessionWorkingDir, setSessionWorkingDir] = useState<string | null>(null);

  // Fetch the working directory from the session when sessionId changes
  useEffect(() => {
    if (!sessionId) {
      setSessionWorkingDir(null);
      return;
    }

    const fetchSessionWorkingDir = async () => {
      try {
        const response = await getSession({ path: { session_id: sessionId } });
        if (response.data?.working_dir) {
          setSessionWorkingDir(response.data.working_dir);
        }
      } catch (error) {
        console.error('[DirSwitcher] Failed to fetch session working dir:', error);
      }
    };

    fetchSessionWorkingDir();
  }, [sessionId]);

  const currentDir = sessionWorkingDir ?? getWorkingDir();

  const handleDirectoryChange = async () => {
    if (isDirectoryChooserOpen) return;
    setIsDirectoryChooserOpen(true);

    let result;
    try {
      result = await window.electron.directoryChooser();
    } finally {
      setIsDirectoryChooserOpen(false);
    }

    if (result.canceled || result.filePaths.length === 0) {
      return;
    }

    const newDir = result.filePaths[0];
    setWorkingDir(newDir);

    if (sessionId) {
      try {
        await updateSessionWorkingDir({
          path: { session_id: sessionId },
          body: { workingDir: newDir },
        });

        try {
          await restartAgent({ body: { session_id: sessionId } });
        } catch (restartError) {
          console.error('[DirSwitcher] Failed to restart agent:', restartError);
          toast.error('Failed to update working directory');
        }

        setSessionWorkingDir(newDir);
      } catch (error) {
        console.error('[DirSwitcher] Failed to update working directory:', error);
        toast.error('Failed to update working directory');
      }
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
      await window.electron.openDirectoryInExplorer(currentDir);
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
            <div className="max-w-[200px] truncate [direction:rtl]">{currentDir}</div>
          </button>
        </TooltipTrigger>
        <TooltipContent side="top">{currentDir}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
};
