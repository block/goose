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
    description:
      'Filter tools based on vector similarity.',
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
  const { read, upsert } = useConfig();

  const handleStrategyChange = async (newStrategy: string) => {
    try {
      await upsert('GOOSE_ROUTER_TOOL_SELECTION_STRATEGY', newStrategy, false);
      setCurrentStrategy(newStrategy);
    } catch (error) {
      console.error('Error updating tool selection strategy:', error);
      throw new Error(`Failed to store new tool selection strategy: ${newStrategy}`);
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
          Configure how Goose selects tools for your requests. Recommended when many extensions are enabled. 
          Available only with Claude models served on Databricks for now.
        </p>
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
