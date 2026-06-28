import { useEffect, useState, useCallback } from 'react';
import { all_goose_modes, ModeSelectionItem } from './ModeSelectionItem';
import { useConfig } from '../../ConfigContext';
import { ConversationLimitsDropdown } from './ConversationLimitsDropdown';

export const ModeSection = () => {
  const [currentMode, setCurrentMode] = useState('auto');
  const [maxTurns, setMaxTurns] = useState<number>(1000);
  const [contextLimit, setContextLimit] = useState<number | null>(null);
  const { config, read, upsert, remove } = useConfig();

  const handleModeChange = async (newMode: string) => {
    try {
      await upsert('GOOSE_MODE', newMode, false);
      setCurrentMode(newMode);
    } catch (error) {
      console.error('Error updating goose mode:', error);
      throw new Error(`Failed to store new goose mode: ${newMode}`);
    }
  };

  useEffect(() => {
    const mode = config.GOOSE_MODE as string | undefined;
    if (mode) {
      setCurrentMode(mode);
    }
  }, [config.GOOSE_MODE]);

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

  const fetchContextLimit = useCallback(async () => {
    try {
      const limit = (await read('GOOSE_CONTEXT_LIMIT', false)) as number;
      if (typeof limit === 'number' && limit > 0) {
        setContextLimit(limit);
      } else {
        setContextLimit(null);
      }
    } catch (error) {
      console.error('Error fetching context limit:', error);
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

  const handleContextLimitChange = async (value: number | null) => {
    try {
      if (value === null) {
        await remove('GOOSE_CONTEXT_LIMIT', false);
        setContextLimit(null);
        return;
      }
      await upsert('GOOSE_CONTEXT_LIMIT', value, false);
      setContextLimit(value);
    } catch (error) {
      console.error('Error updating context limit:', error);
    }
  };

  useEffect(() => {
    fetchMaxTurns();
    fetchContextLimit();
  }, [fetchMaxTurns, fetchContextLimit]);

  useEffect(() => {
    const limit = config.GOOSE_CONTEXT_LIMIT as number | undefined;
    if (typeof limit === 'number' && limit > 0) {
      setContextLimit(limit);
    } else if (limit === undefined) {
      setContextLimit(null);
    }
  }, [config.GOOSE_CONTEXT_LIMIT]);

  return (
    <div className="space-y-1">
      {/* Mode Selection */}
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

      {/* Conversation Limits Dropdown */}
      <ConversationLimitsDropdown
        maxTurns={maxTurns}
        onMaxTurnsChange={handleMaxTurnsChange}
        contextLimit={contextLimit}
        onContextLimitChange={handleContextLimitChange}
      />
    </div>
  );
};
