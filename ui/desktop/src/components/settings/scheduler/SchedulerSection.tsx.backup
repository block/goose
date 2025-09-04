import { useState, useEffect } from 'react';
import { SchedulingEngine, Settings } from '../../../utils/settings';

interface SchedulerSectionProps {
  onSchedulingEngineChange?: (engine: SchedulingEngine) => void;
}

export default function SchedulerSection({ onSchedulingEngineChange }: SchedulerSectionProps) {
  const [schedulingEngine, setSchedulingEngine] = useState<SchedulingEngine>('builtin-cron');

  useEffect(() => {
    // Load current scheduling engine setting
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

      // Save the setting
      await window.electron.setSchedulingEngine(engine);

      // Notify parent component
      if (onSchedulingEngineChange) {
        onSchedulingEngineChange(engine);
      }
    } catch (error) {
      console.error('Failed to save scheduling engine setting:', error);
    }
  };

  return (
    <div className="px-4">
      <div className="space-y-3">
        <div className="flex items-start space-x-3">
          <input
            type="radio"
            id="builtin-cron"
            name="schedulingEngine"
            value="builtin-cron"
            checked={schedulingEngine === 'builtin-cron'}
            onChange={() => handleEngineChange('builtin-cron')}
            className="mt-1 h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
          />
          <div className="flex-1">
            <label htmlFor="builtin-cron" className="block text-sm font-medium text-textStandard">
              Built-in Cron (Default)
            </label>
            <p className="text-xs text-textSubtle mt-1">
              Uses Goose's built-in cron scheduler. Simple and reliable for basic scheduling needs.
            </p>
          </div>
        </div>

        <div className="flex items-start space-x-3">
          <input
            type="radio"
            id="temporal"
            name="schedulingEngine"
            value="temporal"
            checked={schedulingEngine === 'temporal'}
            onChange={() => handleEngineChange('temporal')}
            className="mt-1 h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
          />
          <div className="flex-1">
            <label htmlFor="temporal" className="block text-sm font-medium text-textStandard">
              Temporal
            </label>
            <p className="text-xs text-textSubtle mt-1">
              Uses Temporal workflow engine for advanced scheduling features. Requires Temporal CLI
              to be installed.
            </p>
          </div>
        </div>
      </div>

      <div className="mt-4 p-3 bg-bgSubtle rounded-md">
        <p className="text-xs text-textSubtle">
          <strong>Note:</strong> Changing the scheduling engine will apply to new Goose sessions.
          You will need to restart Goose for the change to take full effect. <br />
          The scheduling engines do not share the list of schedules.
        </p>
      </div>
    </div>
  );
}
