import { useCallback, useEffect, useState } from 'react';
import { Tornado } from 'lucide-react';
import { createGooseModes, ModeSelectionItem } from '../settings/mode/ModeSelectionItem';
import { useConfig } from '../ConfigContext';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import { trackModeChanged } from '../../utils/analytics';
import { useLocalization } from '../../contexts/LocalizationContext';

export const BottomMenuModeSelection = () => {
  const { t } = useLocalization();
  const allGooseModes = createGooseModes(t);
  const [gooseMode, setGooseMode] = useState('auto');
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

  const handleModeChange = async (newMode: string) => {
    if (gooseMode === newMode) {
      return;
    }

    try {
      await upsert('GOOSE_MODE', newMode, false);
      setGooseMode(newMode);
      trackModeChanged(gooseMode, newMode);
    } catch (error) {
      console.error('Error updating goose mode:', error);
      throw new Error(`Failed to store new goose mode: ${newMode}`);
    }
  };

  function getValueByKey(key: string) {
    const mode = allGooseModes.find((mode) => mode.key === key);
    return mode ? mode.label : t('modes.autonomous.label');
  }

  function getModeDescription(key: string) {
    const mode = allGooseModes.find((mode) => mode.key === key);
    return mode ? mode.description : t('modes.autonomous.description');
  }

  return (
    <div title={`Current mode: ${getValueByKey(gooseMode)} - ${getModeDescription(gooseMode)}`}>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <span className="flex items-center cursor-pointer [&_svg]:size-4 text-text-primary/70 hover:text-text-primary hover:scale-100 hover:bg-transparent text-xs">
            <Tornado className="mr-1 h-4 w-4" />
            {getValueByKey(gooseMode).toLowerCase()}
          </span>
        </DropdownMenuTrigger>
        <DropdownMenuContent className="w-64" side="top" align="center">
          {allGooseModes.map((mode) => (
            <DropdownMenuItem key={mode.key} asChild>
              <ModeSelectionItem
                mode={mode}
                currentMode={gooseMode}
                showDescription={false}
                isApproveModeConfigure={false}
                handleModeChange={handleModeChange}
              />
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
};
