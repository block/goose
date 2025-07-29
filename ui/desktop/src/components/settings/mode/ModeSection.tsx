import { useEffect, useState, useCallback } from 'react';
import { all_goose_modes, ModeSelectionItem } from './ModeSelectionItem';
import { useConfig } from '../../ConfigContext';
import { Input } from '../../ui/input';

export const ModeSection = () => {
  const [currentMode, setCurrentMode] = useState('auto');
  const [maxTurns, setMaxTurns] = useState<number>(1000);
  const { read, upsert } = useConfig();

  const handleModeChange = async (newMode: string) => {
    try {
      await upsert('GOOSE_MODE', newMode, false);
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

  const fetchMaxTurns = useCallback(async () => {
    try {
      const turns = (await read('GOOSE_MAX_TURNS', false)) as number;
      if (turns) {
        setMaxTurns(turns);
      }
    } catch (error) {
      console.error('Error fetching max turns:', error);
    }
  }, [read]);

  const handleMaxTurnsChange = async (value: number) => {
    try {
      await upsert('GOOSE_MAX_TURNS', value, false);
      setMaxTurns(value);
    } catch (error) {
      console.error('Error updating max turns:', error);
    }
  };

  useEffect(() => {
    fetchCurrentMode();
    fetchMaxTurns();
  }, [fetchCurrentMode, fetchMaxTurns]);

  return (
    <div className="space-y-1">
      {all_goose_modes.map((mode) => (
        <ModeSelectionItem
          key={mode.key}
          mode={mode}
          currentMode={currentMode}
          showDescription={true}
          isApproveModeConfigure={false}
          handleModeChange={handleModeChange}
        />
      ))}

      <div className="pt-6">
        <h3 className="text-textStandard mb-4 pl-2">Conversation Limits</h3>
        <div className="flex items-center justify-between py-2 px-4">
          <div>
            <h4 className="text-textStandard">Max Turns</h4>
            <p className="text-xs text-textSubtle mt-[2px]">
              Maximum agent turns before Goose asks for user input
            </p>
          </div>
          <Input
            type="number"
            min="1"
            value={maxTurns}
            onChange={(e) => handleMaxTurnsChange(Number(e.target.value))}
            className="w-20"
          />
        </div>
      </div>
    </div>
  );
};
