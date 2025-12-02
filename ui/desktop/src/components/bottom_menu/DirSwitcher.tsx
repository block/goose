import React, { useState } from 'react';
import { FolderDot } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/Tooltip';
import { useChatContext } from '../../contexts/ChatContext';
import { updateSessionWorkingDir, restartAgent } from '../../api';
import { toast } from 'react-toastify';

interface DirSwitcherProps {
  className?: string;
}

export const DirSwitcher: React.FC<DirSwitcherProps> = ({ className = '' }) => {
  const [isTooltipOpen, setIsTooltipOpen] = useState(false);
  const chatContext = useChatContext();
  const sessionId = chatContext?.chat?.sessionId;

  const [currentDir, setCurrentDir] = useState(String(window.appConfig.get('GOOSE_WORKING_DIR')));

  const handleDirectoryChange = async () => {
    console.log('[DirSwitcher] Starting directory change process');
    console.log('[DirSwitcher] Current sessionId:', JSON.stringify(sessionId));
    console.log('[DirSwitcher] Current working dir:', JSON.stringify(currentDir));

    // Open directory chooser dialog for in-place change
    const result = await window.electron.directoryChooser(true);
    console.log('[DirSwitcher] Directory chooser result:', JSON.stringify(result));

    if (!result.canceled && result.filePaths.length > 0 && sessionId) {
      const newDir = result.filePaths[0];
      console.log('[DirSwitcher] New directory selected:', JSON.stringify(newDir));

      try {
        // Update the working directory on the backend
        const updateRequest = {
          path: {
            session_id: sessionId,
          },
          body: {
            workingDir: newDir,
          },
        };
        console.log('[DirSwitcher] Sending update request:', JSON.stringify(updateRequest));

        const response = await updateSessionWorkingDir(updateRequest);
        console.log('[DirSwitcher] Update response:', JSON.stringify(response));

        // Restart the agent to pick up the new working directory
        console.log('[DirSwitcher] Restarting agent to apply new working directory...');
        const restartRequest = {
          body: {
            session_id: sessionId,
          },
        };

        try {
          await restartAgent(restartRequest);
          console.log('[DirSwitcher] Agent restarted successfully');
        } catch (restartError) {
          console.error('[DirSwitcher] Failed to restart agent:', JSON.stringify(restartError));
          // Continue anyway - the working directory is still updated in the session
        }

        // Update the local state and config
        setCurrentDir(newDir);

        // Send an IPC message to update the config in the main process
        window.electron.emit('update-working-dir', sessionId, newDir);

        // Show success message
        toast.success(`Working directory changed to ${newDir} and agent restarted`);

        console.log('[DirSwitcher] Working directory updated and agent restarted');
        console.log('[DirSwitcher] Agent will now use:', newDir);
      } catch (error) {
        console.error('[DirSwitcher] Failed to update working directory:', JSON.stringify(error));
        console.error('[DirSwitcher] Error details:', error);
        toast.error('Failed to update working directory');
      }
    } else {
      console.log('[DirSwitcher] Directory change canceled or no sessionId');
      console.log('[DirSwitcher] Canceled:', result.canceled);
      console.log('[DirSwitcher] SessionId:', JSON.stringify(sessionId));
    }
  };

  const handleDirectoryClick = async (event: React.MouseEvent) => {
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
      <Tooltip open={isTooltipOpen} onOpenChange={setIsTooltipOpen}>
        <TooltipTrigger asChild>
          <button
            className={`z-[100] hover:cursor-pointer text-text-default/70 hover:text-text-default text-xs flex items-center transition-colors pl-1 [&>svg]:size-4 ${className}`}
            onClick={handleDirectoryClick}
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
