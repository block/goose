import { ChevronDown, ChevronUp } from '../../../icons';
import { Sliders } from 'lucide-react';
import React, { useEffect, useState, useRef } from 'react';
import { useConfig } from '../../../ConfigContext';
import { getCurrentModelAndProviderForDisplay } from '../index';
import { AddModelModal } from '../subcomponents/AddModelModal';
import type { View } from '../../../../App';

interface ModelsBottomBarProps {
  dropdownRef: any;
  setView: (view: View) => void;
}
export default function ModelsBottomBar({ dropdownRef, setView }: ModelsBottomBarProps) {
  const { read, getProviders } = useConfig();
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const [provider, setProvider] = useState<string | null>(null);
  const [model, setModel] = useState<string>('');
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    (async () => {
      const modelProvider = await getCurrentModelAndProviderForDisplay({
        readFromConfig: read,
        getProviders,
      });
      setProvider(modelProvider.provider);
      setModel(modelProvider.model);
    })();
  }, [read, getProviders]);

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
          className="flex items-center cursor-pointer"
          onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
        >
          {model}
          {isModelMenuOpen ? (
            <ChevronDown className="w-4 h-4 ml-1" />
          ) : (
            <ChevronUp className="w-4 h-4 ml-1" />
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
