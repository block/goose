import React, { useState, useEffect, useRef } from 'react';
import { useModel } from './settings/models/ModelContext';
import { useRecentModels } from './settings/models/RecentModels'; // Hook for recent models
import { ChevronUp, ChevronDown, Sliders } from 'lucide-react';
import { ModelRadioList } from './settings/models/ModelRadioList';
import { useNavigate } from 'react-router-dom';

export default function BottomMenu({ hasMessages }) {
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const { currentModel } = useModel();
  const { recentModels } = useRecentModels(); // Get recent models
  const navigate = useNavigate();
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Add effect to handle clicks outside
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

  // Add effect to handle Escape key
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

  return (
    <div className="flex justify-between relative border-t dark:border-gray-700 text-bottom-menu dark:text-bottom-menu-dark pl-[15px] text-[10px] h-[30px] leading-[30px] align-middle bg-bottom-menu-background dark:bg-bottom-menu-background-dark rounded-b-2xl">
      {/* Directory Chooser - Always visible */}
      <span
        className="cursor-pointer"
        onClick={async () => {
          console.log('Opening directory chooser');
          if (hasMessages) {
            window.electron.directoryChooser();
          } else {
            window.electron.directoryChooser(true);
          }
        }}
      >
        Working in {window.appConfig.get('GOOSE_WORKING_DIR')}
      </span>

      {/* Model Selector Dropdown - Only in development */}
      <div className="relative flex items-center ml-auto mr-4" ref={dropdownRef}>
        <div
          className="flex items-center cursor-pointer"
          onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
        >
          <span>{currentModel?.name || 'Select Model'}</span>
          {isModelMenuOpen ? (
            <ChevronUp className="w-4 h-4 ml-1" />
          ) : (
            <ChevronDown className="w-4 h-4 ml-1" />
          )}
        </div>

        {/* Dropdown Menu */}
        {isModelMenuOpen && (
          <div className="absolute bottom-[30px] right-0 w-[300px] bg-white dark:bg-gray-800 border dark:border-gray-700 rounded-lg shadow-lg">
            <div className="p-2">
              <ModelRadioList
                className="divide-y divide-gray-100 dark:divide-gray-700"
                renderItem={({ model, isSelected, onSelect }) => (
                  <label key={model.name} className="block cursor-pointer">
                    <div
                      className="flex items-center justify-between p-3 transition-colors
                      hover:text-gray-900 dark:hover:text-white"
                      onClick={onSelect}
                    >
                      <div>
                        <p className="text-sm font-semibold">{model.name}</p>
                        <p className="text-xs text-muted-foreground">{model.provider}</p>
                      </div>
                      <div className="relative">
                        <input
                          type="radio"
                          name="recentModels"
                          value={model.name}
                          checked={isSelected}
                          onChange={onSelect}
                          className="peer sr-only"
                        />
                        <div
                          className="h-4 w-4 rounded-full border border-gray-400 dark:border-gray-500
                          peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                          peer-checked:bg-white dark:peer-checked:bg-black
                          transition-all duration-200 ease-in-out"
                        ></div>
                      </div>
                    </div>
                  </label>
                )}
              />
              <div
                className="flex items-center justify-between p-3 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700
                  border-t-2 border-gray-200 dark:border-gray-600"
                onClick={() => {
                  setIsModelMenuOpen(false);
                  navigate('/settings');
                }}
              >
                <span className="text-sm">Tools and Settings</span>
                <Sliders className="w-5 h-5 ml-2 rotate-90" />
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
