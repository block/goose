import React, { useEffect, useState, useCallback } from 'react';
import { all_goose_modes, filterGooseModes, ModeSelectionItem } from './ModeSelectionItem';
import { useConfig } from '../../ConfigContext';

export const ModeSelection = () => {
  const [currentMode, setCurrentMode] = useState('auto');
  const [previousApproveModel, setPreviousApproveModel] = useState('');
  const { read, upsert } = useConfig();

  const handleModeChange = async (newMode: string) => {
    try {
      await upsert('GOOSE_MODE', newMode, false);
      // Only track the previous approve if current mode is approve related but new mode is not.
      if (currentMode.includes('approve') && !newMode.includes('approve')) {
        setPreviousApproveModel(currentMode);
      }
      setCurrentMode(newMode);
    } catch (error) {
      console.error('Error updating goose mode:', error);
      throw new Error(`Failed to store new goose mode: ${newMode}`);
    }
  };

  const fetchCurrentMode = useCallback(async () => {
    try {
      const mode = (await read('GOOSE_MODE', false)) as string;
      if (mode) {
        setCurrentMode(mode);
      }
    } catch (error) {
      console.error('Error fetching current mode:', error);
    }
  }, [read]);

  useEffect(() => {
    fetchCurrentMode();
  }, [fetchCurrentMode]);

  return (
    <div>
      <div>
        {filterGooseModes(currentMode, all_goose_modes, previousApproveModel).map((mode) => (
          <ModeSelectionItem
            key={mode.key}
            mode={mode}
            currentMode={currentMode}
            showDescription={true}
            isApproveModeConfigure={false}
            handleModeChange={handleModeChange}
          />
        ))}
      </div>
    </div>
  );
};
