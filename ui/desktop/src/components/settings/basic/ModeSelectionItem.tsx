import React, { useState } from 'react';
import { Gear } from '../../icons';
import { ConfigureApproveMode } from './ConfigureApproeMode';

export interface GooseMode {
  key: string;
  label: string;
  description: string;
}

export const all_goose_modes: GooseMode[] = [
  {
    key: 'auto',
    label: 'Completely Autonomous',
    description: 'Full file modification capabilities, edit, create, and delete files freely.',
  },
  {
    key: 'approve',
    label: 'Manual Approval',
    description: 'All tools, extensions and file modificatio will require human approval',
  },
  {
    key: 'write_approve',
    label: 'Write Approval',
    description:
      'Classifies the tool as either a read-only tool or write tool. Write tools will ask for human approval.',
  },
  {
    key: 'chat',
    label: 'Chat Only',
    description: 'Engage with the selected provider without using tools or extensions.',
  },
];

export function filterGooseModes(currentMode: string, modes: GooseMode[]) {
  return modes.filter((mode) => {
    if (['auto', 'chat'].includes(mode.key)) {
      return true; // Always keep 'auto' and 'chat'
    }
    if (currentMode === 'approve' && mode.key === 'approve') {
      return true; // Keep 'approve' if currentMode is 'approve'
    }
    if (currentMode !== 'approve' && mode.key === 'write_approve') {
      return true; // Keep 'write_approve' if currentMode is not 'approve'
    }
    return false; // Exclude other modes
  });
}

interface ModeSelectionItemProps {
  currentMode: string;
  mode: GooseMode;
  showDescription: boolean;
  isApproveModeConfigure: boolean;
  handleModeChange: (newMode: string) => void;
}

export function ModeSelectionItem({
  currentMode,
  mode,
  showDescription,
  isApproveModeConfigure,
  handleModeChange,
}: ModeSelectionItemProps) {
  const [isDislogOpen, setIsDislogOpen] = useState(false);

  return (
    <div>
      <div
        className="flex items-center justify-between p-2 text-textStandard hover:bg-bgSubtle transition-colors"
        onClick={() => handleModeChange(mode.key)}
      >
        <div>
          <h3 className="text-sm font-semibold text-textStandard dark:text-gray-200">
            {mode.label}
          </h3>
          {showDescription && (
            <p className="text-xs text-textSubtle dark:text-gray-400 mt-[2px]">
              {mode.description}
            </p>
          )}
        </div>
        <div className="relative flex items-center gap-3">
          {!isApproveModeConfigure && (mode.key == 'approve' || mode.key == 'write_approve') && (
            <button
              onClick={() => {
                setIsDislogOpen(true);
              }}
            >
              <Gear className="w-5 h-5 text-textSubtle hover:text-textStandard" />
            </button>
          )}
          <input
            type="radio"
            name="modes"
            value={mode.key}
            checked={currentMode === mode.key}
            onChange={() => handleModeChange(mode.key)}
            className="peer sr-only"
          />
          <div
            className="h-5 w-5 rounded-full border border-gray-400 dark:border-gray-500
                  peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                  peer-checked:bg-white dark:peer-checked:bg-black
                  transition-all duration-200 ease-in-out"
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
