import { ChevronDown, ChevronUp } from '../../../icons';
import { Sliders } from 'lucide-react';
import React, { useEffect, useState, useRef } from 'react';
import { useConfig } from '../../../ConfigContext';
import { getCurrentModelAndProviderForDisplay } from '../index';
import { AddModelModal } from '../subcomponents/AddModelModal';
import { View } from '../../../../App';
import { Tooltip, TooltipTrigger, TooltipContent, TooltipProvider } from '../../../ui/Tooltip';

interface ModelsBottomBarProps {
  dropdownRef: React.RefObject<HTMLDivElement>;
  setView: (view: View) => void;
}
export default function ModelsBottomBar({ dropdownRef, setView }: ModelsBottomBarProps) {
  const { read, getProviders } = useConfig();
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const [provider, setProvider] = useState<string | null>(null);
  const [model, setModel] = useState<string>('');
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const [isModelTruncated, setIsModelTruncated] = useState(false);
  // eslint-disable-next-line no-undef
  const modelRef = useRef<HTMLSpanElement>(null);
  const [isTooltipOpen, setIsTooltipOpen] = useState(false);

  useEffect(() => {
    (async () => {
      const modelProvider = await getCurrentModelAndProviderForDisplay({
        readFromConfig: read,
        getProviders,
      });
      setProvider(modelProvider.provider as string | null);
      setModel(modelProvider.model as string);
    })();
  });

  useEffect(() => {
    const checkTruncation = () => {
      if (modelRef.current) {
        setIsModelTruncated(modelRef.current.scrollWidth > modelRef.current.clientWidth);
      }
    };
    checkTruncation();
    window.addEventListener('resize', checkTruncation);
    return () => window.removeEventListener('resize', checkTruncation);
  }, [model]);

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
    <div className="relative flex items-center ml-auto mr-4" ref={dropdownRef}>
      <div ref={menuRef} className="relative">
        <div
          className="flex items-center cursor-pointer max-w-[180px] md:max-w-[200px] lg:max-w-[380px] min-w-0 group"
          onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
        >
          <TooltipProvider>
            <Tooltip open={isTooltipOpen} onOpenChange={setIsTooltipOpen}>
              <TooltipTrigger asChild>
                <span
                  ref={modelRef}
                  className="truncate max-w-[130px] md:max-w-[200px] lg:max-w-[360px] min-w-0 block"
                >
                  {model || 'Select Model'}
                </span>
              </TooltipTrigger>
              {isModelTruncated && (
                <TooltipContent className="max-w-96 overflow-auto scrollbar-thin" side="top">
                  {model || 'Select Model'}
                </TooltipContent>
              )}
            </Tooltip>
          </TooltipProvider>
          {isModelMenuOpen ? (
            <ChevronDown className="w-4 h-4 ml-1 flex-shrink-0" />
          ) : (
            <ChevronUp className="w-4 h-4 ml-1 flex-shrink-0" />
          )}
        </div>

        {/* Dropdown Menu */}
        {isModelMenuOpen && (
          <div className="absolute bottom-[24px] right-0 w-[300px] bg-bgApp rounded-lg border border-borderSubtle">
            <div className="">
              <div className="text-sm text-textProminent mt-3 ml-2">Current:</div>
              <div className="flex items-center justify-between text-sm ml-2">
                {model} -- {provider}
              </div>
              <div
                className="flex items-center justify-between text-textStandard p-2 cursor-pointer hover:bg-bgStandard
                    border-t border-borderSubtle mt-2"
                onClick={() => {
                  setIsModelMenuOpen(false);
                  setIsAddModelModalOpen(true);
                }}
              >
                <span className="text-sm">Change Model</span>
                <Sliders className="w-5 h-5 ml-2 rotate-90" />
              </div>
            </div>
          </div>
        )}
      </div>

      {isAddModelModalOpen ? (
        <AddModelModal setView={setView} onClose={() => setIsAddModelModalOpen(false)} />
      ) : null}
    </div>
  );
}
