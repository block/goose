import { Sliders } from 'lucide-react';
import React, { useEffect, useState, useRef } from 'react';
import { useModelAndProvider } from '../../../ModelAndProviderContext';
import { AddModelModal } from '../subcomponents/AddModelModal';
import { LeadWorkerSettings } from '../subcomponents/LeadWorkerSettings';
import { View } from '../../../../App';
import { Tooltip, TooltipTrigger, TooltipContent, TooltipProvider } from '../../../ui/Tooltip';
import Modal from '../../../Modal';
import { useCurrentModelInfo } from '../../../ChatView';
import { useConfig } from '../../../ConfigContext';

interface ModelsBottomBarProps {
  dropdownRef: React.RefObject<HTMLDivElement>;
  setView: (view: View) => void;
}
export default function ModelsBottomBar({ dropdownRef, setView }: ModelsBottomBarProps) {
  const { currentModel, currentProvider, getCurrentModelAndProviderForDisplay } =
    useModelAndProvider();
  const currentModelInfo = useCurrentModelInfo();
  const { read } = useConfig();
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const [displayProvider, setDisplayProvider] = useState<string | null>(null);
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);
  const [isLeadWorkerModalOpen, setIsLeadWorkerModalOpen] = useState(false);
  const [isLeadWorkerActive, setIsLeadWorkerActive] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const [isModelTruncated, setIsModelTruncated] = useState(false);
  // eslint-disable-next-line no-undef
  const modelRef = useRef<HTMLSpanElement>(null);
  const [isTooltipOpen, setIsTooltipOpen] = useState(false);

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

  // Update display provider when current provider changes
  useEffect(() => {
    if (currentProvider) {
      (async () => {
        const modelProvider = await getCurrentModelAndProviderForDisplay();
        setDisplayProvider(modelProvider.provider);
      })();
    }
  }, [currentProvider, getCurrentModelAndProviderForDisplay]);

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

  // Add click outside handler
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setIsModelMenuOpen(false);
      }
    }

    // Add the event listener when the menu is open
    if (isModelMenuOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    // Clean up the event listener
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isModelMenuOpen]);

  return (
    <div className="relative flex items-center" ref={dropdownRef}>
      <div ref={menuRef} className="relative">
        <div
          className="flex items-center hover:cursor-pointer max-w-[180px] md:max-w-[200px] lg:max-w-[380px] min-w-0 group hover:text-textStandard transition-colors"
          onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
        >
          <TooltipProvider>
            <Tooltip open={isTooltipOpen} onOpenChange={setIsTooltipOpen}>
              <TooltipTrigger asChild>
                <span
                  ref={modelRef}
                  className="truncate max-w-[130px] md:max-w-[200px] lg:max-w-[360px] min-w-0 block"
                >
                  {displayModel}
                  {isLeadWorkerActive && modelMode && (
                    <span className="ml-1 text-[10px] opacity-60">({modelMode})</span>
                  )}
                </span>
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
          </TooltipProvider>
        </div>

        {/* Dropdown Menu */}
        {isModelMenuOpen && (
          <div className="absolute bottom-[24px] right-[-55px] w-[300px] bg-bgApp rounded-lg border border-borderSubtle">
            <div className="">
              <div className="text-sm text-textProminent mt-2 ml-2">Current:</div>
              <div className="flex items-center justify-between text-sm ml-2">
                {currentModel} -- {displayProvider}
              </div>
              <div
                className="flex items-center justify-between text-textStandard p-2 cursor-pointer transition-colors hover:bg-bgStandard
                    border-t border-borderSubtle mt-2"
                onClick={() => {
                  setIsModelMenuOpen(false);
                  setIsAddModelModalOpen(true);
                }}
              >
                <span className="text-sm">Change Model</span>
                <Sliders className="w-4 h-4 ml-2 rotate-90" />
              </div>
              <div
                className="flex items-center justify-between text-textStandard p-2 cursor-pointer transition-colors hover:bg-bgStandard
                    border-t border-borderSubtle"
                onClick={() => {
                  setIsModelMenuOpen(false);
                  setIsLeadWorkerModalOpen(true);
                }}
              >
                <span className="text-sm">Lead/Worker Settings</span>
                <Sliders className="w-4 h-4 ml-2" />
              </div>
            </div>
          </div>
        )}
      </div>

      {isAddModelModalOpen ? (
        <AddModelModal setView={setView} onClose={() => setIsAddModelModalOpen(false)} />
      ) : null}

      {isLeadWorkerModalOpen ? (
        <Modal onClose={() => setIsLeadWorkerModalOpen(false)}>
          <LeadWorkerSettings onClose={() => setIsLeadWorkerModalOpen(false)} />
        </Modal>
      ) : null}
    </div>
  );
}
