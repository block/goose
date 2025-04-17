import { useEffect, useState } from 'react';
import { Gear } from '../../icons';
import { ConfigureApproveMode } from './ConfigureApproveMode';
import { View, ViewOptions } from '../../../App';

export interface GooseMode {
  key: string;
  label: string;
  description: string;
}

export const all_goose_modes: GooseMode[] = [
  {
    key: 'auto',
    label: 'Completely autonomous',
    description: 'Full file modification capabilities, edit, create, and delete files freely.',
  },
  {
    key: 'approve',
    label: 'Manual approval',
    description: 'All tools, extensions and file modifications will require human approval',
  },
  {
    key: 'smart_approve',
    label: 'Smart approval',
    description: 'Intelligently determine which actions need approval based on risk level ',
  },
  {
    key: 'chat',
    label: 'Chat only',
    description: 'Engage with the selected provider without using tools or extensions.',
  },
];

interface ModeSelectionItemProps {
  currentMode: string;
  mode: GooseMode;
  showDescription: boolean;
  isApproveModeConfigure: boolean;
  parentView: View;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  handleModeChange: (newMode: string) => void;
}

export function ModeSelectionItem({
  currentMode,
  mode,
  showDescription,
  isApproveModeConfigure,
  parentView,
  setView,
  handleModeChange,
}: ModeSelectionItemProps) {
  const [checked, setChecked] = useState(currentMode == mode.key);
  const [isDislogOpen, setIsDislogOpen] = useState(false);

  useEffect(() => {
    setChecked(currentMode === mode.key);
  }, [currentMode, mode.key]);

  return (
    <div className="group hover:cursor-pointer">
      <div
        className="flex items-center justify-between text-textStandard mb-4"
        onClick={() => handleModeChange(mode.key)}
      >
        <div className="flex">
          <div>
            <h3 className="text-textStandard dark:text-gray-200">{mode.label}</h3>
            {showDescription && (
              <p className="text-xs text-textSubtle dark:text-gray-400 mt-[2px]">
                {mode.description}
              </p>
            )}
          </div>
        </div>

        <div className="relative flex items-center gap-3 mr-4">
          {!isApproveModeConfigure && (mode.key == 'approve' || mode.key == 'smart_approve') && (
            <button
              onClick={() => {
                setView('permission', {
                  parentView,
                });
              }}
            >
              <Gear className="w-5 h-5 text-textSubtle hover:text-textStandard" />
            </button>
          )}
          <input
            type="radio"
            name="modes"
            value={mode.key}
            checked={checked}
            onChange={() => handleModeChange(mode.key)}
            className="peer sr-only"
          />
          <div
            className="h-4 w-4 rounded-full border border-borderStandard 
                  peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                  peer-checked:bg-white dark:peer-checked:bg-black
                  transition-all duration-200 ease-in-out group-hover:border-borderProminent"
          ></div>
        </div>
      </div>
      <div>
        <div>
          {isDislogOpen ? (
            <ConfigureApproveMode
              onClose={() => {
                setIsDislogOpen(false);
              }}
              handleModeChange={handleModeChange}
              currentMode={currentMode}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}
