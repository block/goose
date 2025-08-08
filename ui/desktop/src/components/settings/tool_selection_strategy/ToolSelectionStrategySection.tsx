import { useEffect, useState, useCallback } from 'react';
import { useConfig } from '../../ConfigContext';
import { getApiUrl } from '../../../config';

interface ToolSelectionStrategy {
  key: string;
  label: string;
  description: string;
}

export const all_tool_selection_strategies: ToolSelectionStrategy[] = [
  {
    key: 'default',
    label: 'Default',
    description: 'Loads all tools from enabled extensions',
  },
  {
    key: 'vector',
    label: 'Vector',
    description: 'Filter tools based on vector similarity.',
  },
  {
    key: 'llm',
    label: 'LLM-based',
    description:
      'Uses LLM to intelligently select the most relevant tools based on the user query context.',
  },
];

export const ToolSelectionStrategySection = () => {
  const [currentStrategy, setCurrentStrategy] = useState('default');
  const [_error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const { read, upsert } = useConfig();

  const handleStrategyChange = async (newStrategy: string) => {
    if (isLoading) return; // Prevent multiple simultaneous requests

    setError(null); // Clear any previous errors
    setIsLoading(true);

    try {
      // First update the configuration
      try {
        await upsert('GOOSE_ROUTER_TOOL_SELECTION_STRATEGY', newStrategy, false);
      } catch (error) {
        console.error('Error updating configuration:', error);
        setError(`Failed to update configuration: ${error}`);
        setIsLoading(false);
        return;
      }

      // Then update the backend
      try {
        const response = await fetch(getApiUrl('/agent/update_router_tool_selector'), {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': await window.electron.getSecretKey(),
          },
        });

        if (!response.ok) {
          const errorData = await response
            .json()
            .catch(() => ({ error: 'Unknown error from backend' }));
          throw new Error(errorData.error || 'Unknown error from backend');
        }

        // Parse the success response
        const data = await response
          .json()
          .catch(() => ({ message: 'Tool selection strategy updated successfully' }));
        if (data.error) {
          throw new Error(data.error);
        }
      } catch (error) {
        console.error('Error updating backend:', error);
        setError(`Failed to update backend: ${error}`);
        setIsLoading(false);
        return;
      }

      // If both succeeded, update the UI
      setCurrentStrategy(newStrategy);
    } catch (error) {
      console.error('Error updating tool selection strategy:', error);
      setError(`Failed to update tool selection strategy: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const fetchCurrentStrategy = useCallback(async () => {
    try {
      const strategy = (await read('GOOSE_ROUTER_TOOL_SELECTION_STRATEGY', false)) as string;
      if (strategy) {
        setCurrentStrategy(strategy);
      }
    } catch (error) {
      console.error('Error fetching current tool selection strategy:', error);
      setError(`Failed to fetch current strategy: ${error}`);
    }
  }, [read]);

  useEffect(() => {
    fetchCurrentStrategy();
  }, [fetchCurrentStrategy]);

  return (
    <div className="space-y-1">
      {all_tool_selection_strategies.map((strategy) => (
        <div className="group hover:cursor-pointer" key={strategy.key}>
          <div
            className={`flex items-center justify-between text-text-default py-2 px-2 ${currentStrategy === strategy.key ? 'bg-background-muted' : 'bg-background-default hover:bg-background-muted'} rounded-lg transition-all`}
            onClick={() => handleStrategyChange(strategy.key)}
          >
            <div className="flex">
              <div>
                <h3 className="text-text-default text-xs">{strategy.label}</h3>
                <p className="text-xs text-text-muted mt-[2px]">{strategy.description}</p>
              </div>
            </div>

            <div className="relative flex items-center gap-2">
              <input
                type="radio"
                name="tool-selection-strategy"
                value={strategy.key}
                checked={currentStrategy === strategy.key}
                onChange={() => handleStrategyChange(strategy.key)}
                disabled={isLoading}
                className="peer sr-only"
              />
              <div
                className="h-4 w-4 rounded-full border border-border-default
                      peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                      peer-checked:bg-white dark:peer-checked:bg-black
                      transition-all duration-200 ease-in-out group-hover:border-border-default"
              ></div>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
};
