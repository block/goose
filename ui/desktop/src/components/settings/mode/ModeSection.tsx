import { useEffect, useState } from 'react';
import { all_goose_modes, ModeSelectionItem } from './ModeSelectionItem';
import { useConfig } from '../../ConfigContext';
import { ConversationLimitsDropdown } from './ConversationLimitsDropdown';
import { updateSession, type GooseMode } from '../../../api';

export const ModeSection = ({ sessionId }: { sessionId?: string }) => {
  const [currentMode, setCurrentMode] = useState('auto');
  const [maxTurns, setMaxTurns] = useState<number>(1000);
  const { config, update } = useConfig();

  const handleModeChange = async (newMode: string) => {
    try {
      if (sessionId) {
        await updateSession({ body: { session_id: sessionId, goose_mode: newMode } });
      }
      await update({ GOOSE_MODE: newMode as GooseMode });
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

  useEffect(() => {
    const turns = config.GOOSE_MAX_TURNS;
    if (turns !== undefined && turns !== null) {
      setMaxTurns(Number(turns));
    }
  }, [config.GOOSE_MAX_TURNS]);

  const handleMaxTurnsChange = async (value: number) => {
    try {
      await update({ GOOSE_MAX_TURNS: value });
      setMaxTurns(value);
    } catch (error) {
      console.error('Error updating max turns:', error);
    }
  };

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
      <ConversationLimitsDropdown maxTurns={maxTurns} onMaxTurnsChange={handleMaxTurnsChange} />
    </div>
  );
};
