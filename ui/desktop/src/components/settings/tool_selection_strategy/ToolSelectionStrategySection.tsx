import { useEffect, useState, useCallback } from 'react';
import { View, ViewOptions } from '../../../App';
import { useConfig } from '../../ConfigContext';

interface ToolSelectionStrategySectionProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

export const all_tool_selection_strategies = [
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

export const ToolSelectionStrategySection = ({
  setView: _setView,
}: ToolSelectionStrategySectionProps) => {
  const [currentStrategy, setCurrentStrategy] = useState('default');
  const [error, setError] = useState<string | null>(null);
  const { read, upsert } = useConfig();

  const handleStrategyChange = async (newStrategy: string) => {
    setError(null); // Clear any previous errors
    try {
      // First update the configuration
      try {
        await upsert('GOOSE_ROUTER_TOOL_SELECTION_STRATEGY', newStrategy, false);
      } catch (error) {
        console.error('Error updating configuration:', error);
        setError(`Failed to update configuration: ${error}`);
        return;
      }

      // Then update the backend
      try {
        const response = await fetch('/api/agent/update_tool_selection_strategy', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
        });
        
        if (!response.ok) {
          const errorData = await response.json();
          throw new Error(errorData.error || 'Unknown error from backend');
        }
      } catch (error) {
        console.error('Error updating backend:', error);
        setError(`Failed to update backend: ${error}`);
        return;
      }

      // If both succeeded, update the UI
      setCurrentStrategy(newStrategy);
    } catch (error) {
      console.error('Error updating tool selection strategy:', error);
      setError(`Failed to update tool selection strategy: ${error}`);
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
    <section id="tool-selection-strategy" className="px-8">
      <div className="flex justify-between items-center mb-2">
        <h2 className="text-xl font-medium text-textStandard">Tool Selection Strategy (preview)</h2>
      </div>
      <div className="border-b border-borderSubtle pb-8">
        <p className="text-sm text-textStandard mb-6">
          Configure how Goose selects tools for your requests. Recommended when many extensions are
          enabled. Available only with Claude models served on Databricks for now.
        </p>
        {error && (
          <div className="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
            {error}
          </div>
        )}
        <div>
          {all_tool_selection_strategies.map((strategy) => (
            <div className="group hover:cursor-pointer" key={strategy.key}>
              <div
                className="flex items-center justify-between text-textStandard py-2 px-4 hover:bg-bgSubtle"
                onClick={() => handleStrategyChange(strategy.key)}
              >
                <div className="flex">
                  <div>
                    <h3 className="text-textStandard">{strategy.label}</h3>
                    <p className="text-xs text-textSubtle mt-[2px]">{strategy.description}</p>
                  </div>
                </div>

                <div className="relative flex items-center gap-2">
                  <input
                    type="radio"
                    name="tool-selection-strategy"
                    value={strategy.key}
                    checked={currentStrategy === strategy.key}
                    onChange={() => handleStrategyChange(strategy.key)}
                    className="peer sr-only"
                  />
                  <div
                    className="h-4 w-4 rounded-full border border-borderStandard 
                          peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                          peer-checked:bg-white dark:peer-checked:bg-black
                          transition-all duration-200 ease-in-out group-hover:border-borderProminent"
                  ></div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
};
