import { Sliders } from 'lucide-react';
import React, { useEffect, useState, useRef, useCallback } from 'react';
import { useModelAndProvider } from '../../../ModelAndProviderContext';
import { AddModelModal } from '../subcomponents/AddModelModal';
import { LeadWorkerSettings } from '../subcomponents/LeadWorkerSettings';
import { View } from '../../../../App';
import { Tooltip, TooltipTrigger, TooltipContent } from '../../../ui/Tooltip';
import { Dialog, DialogContent } from '../../../ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '../../../ui/dropdown-menu';
import { useCurrentModelInfo } from '../../../ChatView';
import { useConfig } from '../../../ConfigContext';

interface ModelsBottomBarProps {
  dropdownRef: React.RefObject<HTMLDivElement>;
  setView: (view: View) => void;
}
export default function ModelsBottomBar({ dropdownRef, setView }: ModelsBottomBarProps) {
  const { currentModel } = useModelAndProvider();
  const currentModelInfo = useCurrentModelInfo();
  const { read } = useConfig();
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);
  const [isLeadWorkerModalOpen, setIsLeadWorkerModalOpen] = useState(false);
  const [isLeadWorkerActive, setIsLeadWorkerActive] = useState(false);
  const [isModelTruncated, setIsModelTruncated] = useState(false);
  // eslint-disable-next-line no-undef
  const modelRef = useRef<HTMLSpanElement>(null);
  const [isTooltipOpen, setIsTooltipOpen] = useState(false);
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);

  // Check if lead/worker mode is active
  useEffect(() => {
    const checkLeadWorker = async () => {
      try {
        const leadModel = await read('GOOSE_LEAD_MODEL', false);
        setIsLeadWorkerActive(!!leadModel);
      } catch (error) {
        setIsLeadWorkerActive(false);
      }
    };
    checkLeadWorker();
  }, [read]);

  // Determine which model to display - activeModel takes priority when lead/worker is active
  const displayModel =
    isLeadWorkerActive && currentModelInfo?.model
      ? currentModelInfo.model
      : currentModel || 'Select Model';
  const modelMode = currentModelInfo?.mode;

  useEffect(() => {
    const checkTruncation = () => {
      if (modelRef.current) {
        setIsModelTruncated(modelRef.current.scrollWidth > modelRef.current.clientWidth);
      }
    };
    checkTruncation();
    window.addEventListener('resize', checkTruncation);
    return () => window.removeEventListener('resize', checkTruncation);
  }, [displayModel]);

  useEffect(() => {
    setIsTooltipOpen(false);
  }, [isModelTruncated]);

  const handleTooltipOpenChange = useCallback(
    (open: boolean) => {
      // Only allow tooltip to open if dropdown is closed
      if (!isDropdownOpen) {
        setIsTooltipOpen(open);
      }
    },
    [isDropdownOpen]
  );

  const handleDropdownOpenChange = useCallback((open: boolean) => {
    setIsDropdownOpen(open);
    // Close tooltip when dropdown opens
    if (open) {
      setIsTooltipOpen(false);
    }
  }, []);

  return (
    <div className="relative flex items-center" ref={dropdownRef}>
      <Tooltip open={isTooltipOpen && !isDropdownOpen} onOpenChange={handleTooltipOpenChange}>
        <TooltipTrigger>
          <DropdownMenu open={isDropdownOpen} onOpenChange={handleDropdownOpenChange}>
            <DropdownMenuTrigger asChild>
              <div className="flex items-center hover:cursor-pointer max-w-[180px] md:max-w-[200px] lg:max-w-[380px] min-w-0 group hover:text-textStandard transition-colors">
                <span
                  ref={modelRef}
                  className="truncate text-text-default/70 hover:text-text-default hover:scale-100 hover:bg-transparent text-xs max-w-[130px] md:max-w-[200px] lg:max-w-[360px] min-w-0 block"
                >
                  {displayModel}
                  {isLeadWorkerActive && modelMode && (
                    <span className="ml-1 text-[10px] opacity-60">({modelMode})</span>
                  )}
                </span>
              </div>
            </DropdownMenuTrigger>
            <DropdownMenuContent side="top" align="center" className="w-64 text-sm">
              <DropdownMenuItem onClick={() => setIsAddModelModalOpen(true)}>
                <span>Change Model</span>
                <Sliders className="ml-auto h-4 w-4 rotate-90" />
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setIsLeadWorkerModalOpen(true)}>
                <span>Lead/Worker Settings</span>
                <Sliders className="ml-auto h-4 w-4" />
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </TooltipTrigger>
        {isModelTruncated && (
          <TooltipContent className="max-w-96 overflow-auto scrollbar-thin" side="top">
            {displayModel}
            {isLeadWorkerActive && modelMode && (
              <span className="ml-1 text-[10px] opacity-60">({modelMode})</span>
            )}
          </TooltipContent>
        )}
      </Tooltip>

      {isAddModelModalOpen ? (
        <AddModelModal setView={setView} onClose={() => setIsAddModelModalOpen(false)} />
      ) : null}

      {isLeadWorkerModalOpen ? (
        <Dialog
          open={isLeadWorkerModalOpen}
          onOpenChange={(open) => !open && setIsLeadWorkerModalOpen(false)}
        >
          <DialogContent className="sm:max-w-[500px]">
            <LeadWorkerSettings onClose={() => setIsLeadWorkerModalOpen(false)} />
          </DialogContent>
        </Dialog>
      ) : null}
    </div>
  );
}
