import { useState } from 'react';
import MoreMenu from './MoreMenu';
import type { View, ViewOptions } from '../../App';
import { Document } from '../icons';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/Tooltip';

export default function MoreMenuLayout({
  hasMessages,
  showMenu = true,
  setView,
  setIsGoosehintsModalOpen,
}: {
  hasMessages?: boolean;
  showMenu?: boolean;
  setView?: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}) {
  const [isTooltipOpen, setIsTooltipOpen] = useState(false);

  // Assume macOS if not explicitly set
  const safeIsMacOS = (window?.electron?.platform || 'darwin') === 'darwin';

  return (
    <div
      className="relative flex items-center h-14 border-b border-borderSubtle w-full"
      style={{ WebkitAppRegion: 'drag' }}
    >
      {showMenu && (
        <div
          className={`flex items-center justify-between w-full h-full ${safeIsMacOS ? 'pl-[86px]' : 'pl-[8px]'} pr-4`}
        >
          <TooltipProvider>
            <Tooltip open={isTooltipOpen} onOpenChange={setIsTooltipOpen}>
              <TooltipTrigger asChild>
                <button
                  className="z-[100] no-drag hover:cursor-pointer border border-borderSubtle hover:border-borderStandard rounded-lg p-2 pr-3 text-textSubtle hover:text-textStandard text-sm flex items-center transition-colors [&>svg]:size-4 "
                  onClick={async () => {
                    if (hasMessages) {
                      window.electron.directoryChooser();
                    } else {
                      window.electron.directoryChooser(true);
                    }
                  }}
                  style={{ minWidth: 0 }}
                >
                  <Document className="mr-1" />
                  <span
                    className="flex-grow block text-ellipsis overflow-hidden"
                    style={{
                      direction: 'rtl',
                      textAlign: 'left',
                      unicodeBidi: 'plaintext',
                      minWidth: 0,
                      maxWidth: '100%',
                    }}
                  >
                    {String(window.appConfig.get('GOOSE_WORKING_DIR'))}
                  </span>
                </button>
              </TooltipTrigger>
              <TooltipContent className="max-w-96 overflow-auto scrollbar-thin" side="top">
                {window.appConfig.get('GOOSE_WORKING_DIR') as string}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>

          <MoreMenu
            setView={setView || (() => {})}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen || (() => {})}
          />
        </div>
      )}
    </div>
  );
}
