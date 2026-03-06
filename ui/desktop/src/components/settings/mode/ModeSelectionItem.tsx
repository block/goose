import { useEffect, useState, forwardRef } from 'react';
import { Gear } from '../../icons';
import { ConfigureApproveMode } from './ConfigureApproveMode';
import PermissionRulesModal from '../permission/PermissionRulesModal';
import type { TranslationKey } from '../../../i18n';

export interface GooseMode {
  key: string;
  label: string;
  description: string;
}

export const createGooseModes = (
  t: (key: TranslationKey, params?: Record<string, string | number>) => string
): GooseMode[] => [
  {
    key: 'auto',
    label: t('modes.autonomous.label'),
    description: t('modes.autonomous.description'),
  },
  {
    key: 'approve',
    label: t('modes.approve.label'),
    description: t('modes.approve.description'),
  },
  {
    key: 'smart_approve',
    label: t('modes.smartApprove.label'),
    description: t('modes.smartApprove.description'),
  },
  {
    key: 'chat',
    label: t('modes.chat.label'),
    description: t('modes.chat.description'),
  },
];

interface ModeSelectionItemProps {
  currentMode: string;
  mode: GooseMode;
  showDescription: boolean;
  isApproveModeConfigure: boolean;
  handleModeChange: (newMode: string) => void;
}

export const ModeSelectionItem = forwardRef<HTMLDivElement, ModeSelectionItemProps>(
  ({ currentMode, mode, showDescription, isApproveModeConfigure, handleModeChange }, ref) => {
    const [checked, setChecked] = useState(currentMode == mode.key);
    const [isDialogOpen, setIsDialogOpen] = useState(false);
    const [isPermissionModalOpen, setIsPermissionModalOpen] = useState(false);

    useEffect(() => {
      setChecked(currentMode === mode.key);
    }, [currentMode, mode.key]);

    return (
      <div ref={ref} className="group hover:cursor-pointer text-sm">
        <div
          className={`flex items-center justify-between text-text-primary py-2 px-2 ${checked ? 'bg-background-secondary' : 'bg-background-primary hover:bg-background-secondary'} rounded-lg transition-all`}
          onClick={() => handleModeChange(mode.key)}
        >
          <div className="flex">
            <div>
              <h3 className="text-text-primary">{mode.label}</h3>
              {showDescription && (
                <p className="text-text-secondary mt-[2px]">{mode.description}</p>
              )}
            </div>
          </div>

          <div className="relative flex items-center gap-2">
            {!isApproveModeConfigure && (mode.key == 'approve' || mode.key == 'smart_approve') && (
              <button
                onClick={(e) => {
                  e.stopPropagation(); // Prevent triggering the mode change
                  setIsPermissionModalOpen(true);
                }}
              >
                <Gear className="w-4 h-4 text-text-secondary hover:text-text-primary" />
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
              className="h-4 w-4 rounded-full border border-border-primary 
                    peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                    peer-checked:bg-white dark:peer-checked:bg-black
                    transition-all duration-200 ease-in-out group-hover:border-border-primary"
            ></div>
          </div>
        </div>
        <div>
          <div>
            {isDialogOpen ? (
                <ConfigureApproveMode
                  onClose={() => {
                    setIsDialogOpen(false);
                  }}
                  handleModeChange={handleModeChange}
                  currentMode={currentMode}
                />
              ) : null}
          </div>
        </div>

        <PermissionRulesModal
          isOpen={isPermissionModalOpen}
          onClose={() => setIsPermissionModalOpen(false)}
        />
      </div>
    );
  }
);

ModeSelectionItem.displayName = 'ModeSelectionItem';
