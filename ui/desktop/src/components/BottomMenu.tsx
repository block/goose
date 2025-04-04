import React, { useState, useEffect, useRef } from 'react';
import { useModel } from './settings/models/ModelContext';
import { Document, ChevronUp, ChevronDown, Attach } from './icons';
import type { View } from '../App';
import { Sliders } from 'lucide-react';
import { BottomMenuModeSelection } from './BottomMenuModeSelection';

export default function BottomMenu({
  hasMessages,
  setView,
}: {
  hasMessages: boolean;
  setView: (view: View) => void;
}) {
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const { currentModel } = useModel();
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsModelMenuOpen(false);
      }
    };

    if (isModelMenuOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isModelMenuOpen]);

  useEffect(() => {
    const handleEsc = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsModelMenuOpen(false);
      }
    };

    if (isModelMenuOpen) {
      window.addEventListener('keydown', handleEsc);
    }

    return () => {
      window.removeEventListener('keydown', handleEsc);
    };
  }, [isModelMenuOpen]);

  const handleFileSelect = async () => {
    const path = await window.electron.selectFileOrDirectory();
    if (path) {
      console.log("Selected path:", path);
    }
  };

  return (
    <div className="flex justify-between items-center text-textSubtle relative bg-bgSubtle border-t border-borderSubtle text-xs pl-4 h-[40px] pb-1 align-middle">
      {/* Left section - File Tools */}
      <div className="flex items-center gap-4">
        <div className="flex items-center cursor-pointer" onClick={async () => {
          if (hasMessages) {
            window.electron.directoryChooser();
          } else {
            window.electron.directoryChooser(true);
          }
        }}>
          <Document className="text-textSubtle hover:text-textStandard w-4 h-4" />
        </div>
        <div className="flex items-center cursor-pointer" onClick={handleFileSelect}>
          <Attach className="text-textSubtle hover:text-textStandard w-4 h-4" />
        </div>
        <BottomMenuModeSelection />
      </div>

      {/* Right section - Model Selection */}
      <div className="relative flex items-center ml-auto mr-4" ref={dropdownRef}>
        <div className="relative">
          <div className="flex items-center cursor-pointer" onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}>
            goose
            <svg width="1.5rem" height="1.5rem" fill="none" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" aria-hidden="true" className="w-4 h-4 ml-1">
              <path fillRule="evenodd" clipRule="evenodd" d="M12.293 10.293a1 1 0 0 1 1.414 0L18 14.586l4.293-4.293a1 1 0 1 1 1.414 1.414l-5 5a1 1 0 0 1-1.414 0l-5-5a1 1 0 0 1 0-1.414Z" fill="currentColor"/>
            </svg>
          </div>
          {isModelMenuOpen && (
            <div className="absolute bottom-[24px] right-0 w-[300px] bg-bgApp rounded-lg border border-borderSubtle">
              <div>
                <div className="text-sm text-textProminent mt-3 ml-2">Current:</div>
                <div className="flex items-center justify-between text-sm ml-2">goose -- Databricks</div>
                <div className="flex items-center justify-between text-textStandard p-2 cursor-pointer hover:bg-bgStandard border-t border-borderSubtle mt-2" onClick={() => setView('settings')}>
                  <span className="text-sm">Change Model</span>
                  <Sliders className="w-5 h-5 ml-2 rotate-90" />
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}