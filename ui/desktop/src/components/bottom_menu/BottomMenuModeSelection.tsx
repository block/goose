import React, { useEffect, useRef, useState, useCallback } from 'react';
import { all_goose_modes, ModeSelectionItem } from '../settings_v2/mode/ModeSelectionItem';
import { useConfig } from '../ConfigContext';
import { View, ViewOptions } from '../../App';
import { Orbit } from 'lucide-react';

interface BottomMenuModeSelectionProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

export const BottomMenuModeSelection = ({ setView }: BottomMenuModeSelectionProps) => {
  const [isGooseModeMenuOpen, setIsGooseModeMenuOpen] = useState(false);
  const [gooseMode, setGooseMode] = useState('auto');
  const gooseModeDropdownRef = useRef<HTMLDivElement>(null);
  const { read, upsert } = useConfig();

  const fetchCurrentMode = useCallback(async () => {
    try {
      const mode = (await read('GOOSE_MODE', false)) as string;
      if (mode) {
        setGooseMode(mode);
      }
    } catch (error) {
      console.error('Error fetching current mode:', error);
    }
  }, [read]);

  useEffect(() => {
    fetchCurrentMode();
  }, [fetchCurrentMode]);

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

    try {
      await upsert('GOOSE_MODE', newMode, false);
      setGooseMode(newMode);
    } catch (error) {
      console.error('Error updating goose mode:', error);
      throw new Error(`Failed to store new goose mode: ${newMode}`);
    }
  };

  function getValueByKey(key: string) {
    const mode = all_goose_modes.find((mode) => mode.key === key);
    return mode ? mode.label : 'auto';
  }

  return (
    <div className="relative flex items-center" ref={gooseModeDropdownRef}>
      <button
        className="flex items-center justify-center text-textSubtle hover:text-textStandard h-6 [&_svg]:size-4"
        onClick={() => setIsGooseModeMenuOpen(!isGooseModeMenuOpen)}
      >
        <span className="pr-1.5">{getValueByKey(gooseMode).toLowerCase()}</span>
        <Orbit />
      </button>

      {/* Dropdown Menu */}
      {isGooseModeMenuOpen && (
        <div className="absolute bottom-[24px] right-0 w-[240px] py-2 bg-bgApp rounded-lg border border-borderSubtle">
          <div>
            {all_goose_modes.map((mode) => (
              <ModeSelectionItem
                key={mode.key}
                mode={mode}
                currentMode={gooseMode}
                showDescription={false}
                isApproveModeConfigure={false}
                parentView="chat"
                setView={setView}
                handleModeChange={handleModeChange}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
