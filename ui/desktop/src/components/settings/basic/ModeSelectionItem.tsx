import React, { useEffect, useState } from 'react';
import { Gear } from '../../icons';
import { ConfigureApproveMode } from './ConfigureApproveMode';
import { RadioInput } from '../../ui/RadioInput';

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
    key: 'smart_approve',
    label: 'Smart Approval',
    description: 'Intelligently determine which actions need approval based on risk level ',
  },
  {
    key: 'chat',
    label: 'Chat Only',
    description: 'Engage with the selected provider without using tools or extensions.',
  },
];

export function filterGooseModes(
  currentMode: string,
  modes: GooseMode[],
  previousApproveMode: string
) {
  return modes.filter((mode) => {
    const approveList = ['approve', 'smart_approve'];
    const nonApproveList = ['auto', 'chat'];
    // Always keep 'auto' and 'chat'
    if (nonApproveList.includes(mode.key)) {
      return true;
    }
    // If current mode is non approve mode, we display write approve by default.
    if (nonApproveList.includes(currentMode) && !previousApproveMode) {
      return mode.key === 'smart_approve';
    }

    // Always include the current and previou approve mode
    if (mode.key === currentMode) {
      return true;
    }

    // Current mode and previous approve mode cannot exist at the same time.
    if (approveList.includes(currentMode) && approveList.includes(previousApproveMode)) {
      return false;
    }

    if (mode.key === previousApproveMode) {
      return true;
    }

    return false;
  });
}

const ConfigureButton = ({ onClick }) => (
  <button
    onClick={(event) => {
      event.stopPropagation();
      onClick();
    }}
  >
    <Gear className="w-5 h-5 text-textSubtle hover:text-textStandard" />
  </button>
);

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
  const [checked, setChecked] = useState(currentMode == mode.key);
  const [isDislogOpen, setIsDislogOpen] = useState(false);

  useEffect(() => {
    setChecked(currentMode === mode.key);
  }, [currentMode, mode.key]);

  const showConfigure =
    !isApproveModeConfigure && (mode.key == 'approve' || mode.key == 'smart_approve');

  return (
    <div>
      <RadioInput
        label={mode.label}
        description={showDescription ? mode.description : null}
        Accessory={() =>
          showConfigure ? <ConfigureButton onClick={() => setIsDislogOpen(true)} /> : null
        }
        isChecked={checked}
        onClick={() => handleModeChange(mode.key)}
        value={mode.key}
      />
      <div>
        {isDislogOpen ? (
          <ConfigureApproveMode
            onClose={() => setIsDislogOpen(false)}
            handleModeChange={handleModeChange}
            currentMode={currentMode}
          />
        ) : null}
      </div>
    </div>
  );
}
