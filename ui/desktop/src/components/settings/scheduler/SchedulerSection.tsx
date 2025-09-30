import { useState, useEffect } from 'react';
import { SchedulingEngine, Settings } from '../../../utils/settings';

interface SchedulingEngineOption {
  key: SchedulingEngine;
  label: string;
  description: string;
}

const schedulingEngineOptions: SchedulingEngineOption[] = [
  {
    key: 'builtin-cron',
    label: 'Built-in Cron (Default)',
    description:
      "Uses Goose's built-in cron scheduler. Simple and reliable for basic scheduling needs.",
  },
  {
    key: 'temporal',
    label: 'Temporal',
    description:
      'Uses Temporal workflow engine for advanced scheduling features. Requires Temporal CLI to be installed.',
  },
];

interface SchedulerSectionProps {
  onSchedulingEngineChange?: (engine: SchedulingEngine) => void;
}

export default function SchedulerSection({ onSchedulingEngineChange }: SchedulerSectionProps) {
  const [schedulingEngine, setSchedulingEngine] = useState<SchedulingEngine>('builtin-cron');

  useEffect(() => {
    const loadSchedulingEngine = async () => {
      try {
        const settings = (await window.electron.getSettings()) as Settings | null;
        if (settings?.schedulingEngine) {
          setSchedulingEngine(settings.schedulingEngine);
        }
      } catch (error) {
        console.error('Failed to load scheduling engine setting:', error);
      }
    };

    loadSchedulingEngine();
  }, []);

  const handleEngineChange = async (engine: SchedulingEngine) => {
    try {
      setSchedulingEngine(engine);

      await window.electron.setSchedulingEngine(engine);

      if (onSchedulingEngineChange) {
        onSchedulingEngineChange(engine);
      }
    } catch (error) {
      console.error('Failed to save scheduling engine setting:', error);
    }
  };

  return (
    <div className="space-y-1">
      {schedulingEngineOptions.map((option) => {
        const isChecked = schedulingEngine === option.key;

        return (
          <div key={option.key} className="group hover:cursor-pointer text-sm">
            <div
              className={`flex items-center justify-between text-text-default py-2 px-2 ${
                isChecked
                  ? 'bg-background-muted'
                  : 'bg-background-default hover:bg-background-muted'
              } rounded-lg transition-all`}
              onClick={() => handleEngineChange(option.key)}
            >
              <div className="flex">
                <div>
                  <h3 className="text-text-default">{option.label}</h3>
                  <p className="text-xs text-text-muted mt-[2px]">{option.description}</p>
                </div>
              </div>

              <div className="relative flex items-center gap-2">
                <input
                  type="radio"
                  name="schedulingEngine"
                  value={option.key}
                  checked={isChecked}
                  onChange={() => handleEngineChange(option.key)}
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
        );
      })}

      <div className="mt-4 p-3 bg-background-subtle rounded-md">
        <p className="text-xs text-text-muted">
          <strong>Note:</strong> Changing the scheduling engine will apply to new Goose sessions.
          You will need to restart Goose for the change to take full effect. <br />
          The scheduling engines do not share the list of schedules.
        </p>
      </div>
    </div>
  );
}
