import { useCallback, useEffect, useState } from 'react';
import { useConfig } from '../../../contexts/ConfigContext';
import { ConversationLimitsDropdown } from './ConversationLimitsDropdown';
import { all_goose_modes, ModeSelectionItem } from './ModeSelectionItem';

export const ModeSection = () => {
  const [currentMode, setCurrentMode] = useState('auto');
  const [maxTurns, setMaxTurns] = useState<number>(1000);
  const [orchestratorMaxConcurrency, setOrchestratorMaxConcurrency] = useState<number>(3);
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
      if (typeof turns === 'number' && turns > 0) {
        setMaxTurns(turns);
      }
    } catch (error) {
      console.error('Error fetching max turns:', error);
    }
  }, [read]);

  const fetchOrchestratorMaxConcurrency = useCallback(async () => {
    try {
      const value = (await read('GOOSE_ORCHESTRATOR_MAX_CONCURRENCY', false)) as number;
      if (typeof value === 'number' && value > 0) {
        setOrchestratorMaxConcurrency(value);
      }
    } catch (error) {
      console.error('Error fetching orchestrator max concurrency:', error);
    }
  }, [read]);

  const handleMaxTurnsChange = async (value: number) => {
    if (!Number.isFinite(value) || value < 1) {
      return;
    }

    try {
      await upsert('GOOSE_MAX_TURNS', value, false);
      setMaxTurns(value);
    } catch (error) {
      console.error('Error updating max turns:', error);
    }
  };

  const handleOrchestratorMaxConcurrencyChange = async (value: number) => {
    if (!Number.isFinite(value) || value < 1) {
      return;
    }

    try {
      await upsert('GOOSE_ORCHESTRATOR_MAX_CONCURRENCY', value, false);
      setOrchestratorMaxConcurrency(value);
    } catch (error) {
      console.error('Error updating orchestrator max concurrency:', error);
    }
  };

  useEffect(() => {
    fetchCurrentMode();
    fetchMaxTurns();
    fetchOrchestratorMaxConcurrency();
  }, [fetchCurrentMode, fetchMaxTurns, fetchOrchestratorMaxConcurrency]);

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

      <ConversationLimitsDropdown
        maxTurns={maxTurns}
        onMaxTurnsChange={handleMaxTurnsChange}
        orchestratorMaxConcurrency={orchestratorMaxConcurrency}
        onOrchestratorMaxConcurrencyChange={handleOrchestratorMaxConcurrencyChange}
      />
    </div>
  );
};
