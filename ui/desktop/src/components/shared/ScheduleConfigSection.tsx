import React, { useState, useEffect } from 'react';
import { Input } from '../ui/input';
import { CronExpressionBuilder } from './CronExpressionBuilder';
import { Clock, Calendar, Play, Monitor } from 'lucide-react';

export type ExecutionMode = 'background' | 'foreground';

export interface ScheduleConfig {
  id: string;
  cron: string;
  cronReadable: string;
  execution_mode: ExecutionMode;
  enabled: boolean;
}

interface ScheduleConfigSectionProps {
  recipeTitle: string;
  value?: ScheduleConfig;
  onChange: (config: ScheduleConfig | null) => void;
  className?: string;
}

const labelClassName = 'block text-sm font-medium text-text-prominent mb-1';

/**
 * Schedule configuration section for recipe modal
 * Includes schedule ID, cron builder, and execution mode
 */
export const ScheduleConfigSection: React.FC<ScheduleConfigSectionProps> = ({
  recipeTitle,
  value,
  onChange,
  className = '',
}) => {
  // Generate schedule ID from recipe title
  const generateScheduleId = (title: string): string => {
    return title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .substring(0, 50);
  };

  const [enabled, setEnabled] = useState(value?.enabled || false);
  const [scheduleId, setScheduleId] = useState(
    value?.id || generateScheduleId(recipeTitle)
  );
  const [cron, setCron] = useState(value?.cron || '0 9 * * *');
  const [cronReadable, setCronReadable] = useState(
    value?.cronReadable || 'At 09:00 AM'
  );
  const [cronValid, setCronValid] = useState(true);
  const [executionMode, setExecutionMode] = useState<ExecutionMode>(
    value?.execution_mode || 'background'
  );

  // Update schedule ID when recipe title changes
  useEffect(() => {
    if (!value?.id && recipeTitle) {
      setScheduleId(generateScheduleId(recipeTitle));
    }
  }, [recipeTitle, value?.id]);

  // Update parent when any value changes
  useEffect(() => {
    if (enabled) {
      onChange({
        id: scheduleId,
        cron,
        cronReadable,
        execution_mode: executionMode,
        enabled: true,
      });
    } else {
      onChange(null);
    }
  }, [enabled, scheduleId, cron, cronReadable, executionMode, onChange]);

  const handleCronChange = (
    newCron: string,
    readable: string,
    isValid: boolean
  ) => {
    setCron(newCron);
    setCronReadable(readable);
    setCronValid(isValid);
  };

  const handleToggle = () => {
    setEnabled(!enabled);
  };

  return (
    <div className={`border border-border-subtle rounded-lg p-4 ${className}`}>
      {/* Header with toggle */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Clock className="w-5 h-5 text-text-muted" />
          <h3 className="text-base font-semibold text-text-prominent">
            Schedule Configuration
          </h3>
        </div>
        <button
          type="button"
          onClick={handleToggle}
          className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
            enabled ? 'bg-accent-default' : 'bg-background-medium'
          }`}
          aria-label="Enable scheduling"
        >
          <span
            className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
              enabled ? 'translate-x-6' : 'translate-x-1'
            }`}
          />
        </button>
      </div>

      {/* Collapsible content */}
      {enabled && (
        <div className="space-y-4 animate-in fade-in slide-in-from-top-2 duration-200">
          {/* Schedule ID */}
          <div>
            <label htmlFor="schedule-id" className={labelClassName}>
              Schedule ID
              <span className="text-xs text-text-muted ml-2">
                (unique identifier for this schedule)
              </span>
            </label>
            <Input
              id="schedule-id"
              type="text"
              value={scheduleId}
              onChange={(e) => setScheduleId(e.target.value)}
              placeholder="e.g., daily-report"
              className="font-mono"
              required={enabled}
            />
          </div>

          {/* Cron Expression Builder */}
          <div>
            <label className={labelClassName}>
              <Calendar className="inline w-4 h-4 mr-1" />
              Schedule Frequency
            </label>
            <CronExpressionBuilder
              value={cron}
              onChange={handleCronChange}
              defaultFrequency="daily"
              defaultTime="09:00"
            />
          </div>

          {/* Execution Mode */}
          <div>
            <label className={labelClassName}>
              <Play className="inline w-4 h-4 mr-1" />
              Execution Mode
            </label>
            <div className="space-y-2 mt-2">
              <label className="flex items-start gap-3 p-3 border border-border-subtle rounded-lg cursor-pointer hover:bg-background-muted transition-colors">
                <input
                  type="radio"
                  name="execution-mode"
                  value="background"
                  checked={executionMode === 'background'}
                  onChange={(e) =>
                    setExecutionMode(e.target.value as ExecutionMode)
                  }
                  className="mt-0.5"
                />
                <div className="flex-1">
                  <div className="font-medium text-text-prominent">
                    Background
                  </div>
                  <div className="text-sm text-text-muted">
                    Runs silently in the background without opening a window
                  </div>
                </div>
              </label>
              <label className="flex items-start gap-3 p-3 border border-border-subtle rounded-lg cursor-pointer hover:bg-background-muted transition-colors">
                <input
                  type="radio"
                  name="execution-mode"
                  value="foreground"
                  checked={executionMode === 'foreground'}
                  onChange={(e) =>
                    setExecutionMode(e.target.value as ExecutionMode)
                  }
                  className="mt-0.5"
                />
                <div className="flex-1">
                  <div className="font-medium text-text-prominent">
                    <Monitor className="inline w-4 h-4 mr-1" />
                    Foreground
                  </div>
                  <div className="text-sm text-text-muted">
                    Opens in a dedicated window with full UI interaction
                  </div>
                </div>
              </label>
            </div>
          </div>

          {/* Schedule Summary */}
          {cronValid && (
            <div className="p-3 bg-background-muted rounded-lg">
              <div className="text-sm text-text-muted">Schedule Summary:</div>
              <div className="text-sm font-medium text-text-prominent mt-1">
                "{scheduleId}" will run {cronReadable.toLowerCase()} in{' '}
                {executionMode} mode
              </div>
            </div>
          )}

          {/* Validation Warning */}
          {!cronValid && (
            <div className="p-3 bg-background-error border border-border-error rounded-lg">
              <div className="text-sm text-text-error">
                Please configure a valid schedule frequency
              </div>
            </div>
          )}
        </div>
      )}

      {/* Disabled state message */}
      {!enabled && (
        <div className="text-sm text-text-muted">
          Enable scheduling to run this recipe automatically on a schedule
        </div>
      )}
    </div>
  );
};

export default ScheduleConfigSection;
