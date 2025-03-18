import React, { useEffect, useRef, useState } from 'react';
import { getApiUrl, getSecretKey } from '../config';
import { ChevronDown, ChevronUp } from './icons';
import {
  all_goose_modes,
  filterGooseModes,
  ModeSelectionItem,
} from './settings/basic/ModeSelectionItem';

export const BottomMenuModeSelection = () => {
  const [isGooseModeMenuOpen, setIsGooseModeMenuOpen] = useState(false);
  const [gooseMode, setGooseMode] = useState('auto');
  const [previousApproveModel, setPreviousApproveModel] = useState('');
  const gooseModeDropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const fetchCurrentMode = async () => {
      try {
        const response = await fetch(getApiUrl('/configs/get?key=GOOSE_MODE'), {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
        });

        if (response.ok) {
          const { value } = await response.json();
          if (value) {
            setGooseMode(value);
          }
        }
      } catch (error) {
        console.error('Error fetching current mode:', error);
      }
    };

    fetchCurrentMode();
  }, []);

  useEffect(() => {
    const handleEsc = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsGooseModeMenuOpen(false);
      }
    };

    if (isGooseModeMenuOpen) {
      window.addEventListener('keydown', handleEsc);
    }

    return () => {
      window.removeEventListener('keydown', handleEsc);
    };
  }, [isGooseModeMenuOpen]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        gooseModeDropdownRef.current &&
        !gooseModeDropdownRef.current.contains(event.target as Node)
      ) {
        setIsGooseModeMenuOpen(false);
      }
    };

    if (isGooseModeMenuOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isGooseModeMenuOpen]);

  const handleModeChange = async (newMode: string) => {
    if (gooseMode === newMode) {
      return;
    }
    const storeResponse = await fetch(getApiUrl('/configs/store'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({
        key: 'GOOSE_MODE',
        value: newMode,
        isSecret: false,
      }),
    });

    if (!storeResponse.ok) {
      const errorText = await storeResponse.text();
      console.error('Store response error:', errorText);
      throw new Error(`Failed to store new goose mode: ${newMode}`);
    }
    if (gooseMode.includes('approve')) {
      setPreviousApproveModel(gooseMode);
    }
    setGooseMode(newMode);
  };

  function getValueByKey(key) {
    const mode = all_goose_modes.find((mode) => mode.key === key);
    return mode ? mode.label : 'auto';
  }

  return (
    <div className="relative flex items-center ml-6" ref={gooseModeDropdownRef}>
      <div
        className="flex items-center cursor-pointer"
        onClick={() => setIsGooseModeMenuOpen(!isGooseModeMenuOpen)}
      >
        <span className="truncate w-[170px]">Goose Mode: {getValueByKey(gooseMode)}</span>
        {isGooseModeMenuOpen ? (
          <ChevronDown className="w-4 h-4 ml-1" />
        ) : (
          <ChevronUp className="w-4 h-4 ml-1" />
        )}
      </div>

      {/* Dropdown Menu */}
      {isGooseModeMenuOpen && (
        <div className="absolute bottom-[24px] right-0 w-[240px] bg-bgApp rounded-lg border border-borderSubtle">
          <div className="divide-y divide-borderSubtle">
            {filterGooseModes(gooseMode, all_goose_modes, previousApproveModel).map((mode) => (
              <ModeSelectionItem
                key={mode.key}
                mode={mode}
                currentMode={gooseMode}
                showDescription={false}
                isApproveModeConfigure={false}
                handleModeChange={handleModeChange}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
