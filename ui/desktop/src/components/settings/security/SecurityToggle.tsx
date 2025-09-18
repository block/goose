import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { useConfig } from '../../ConfigContext';

export const SecurityToggle = () => {
  const { config, upsert } = useConfig();

  // Derive enabled directly from config
  const configRecord = config as Record<string, unknown>;
  const enabled = (configRecord?.['security_enabled'] as boolean) ?? false;
  const configThreshold = (configRecord?.['security_threshold'] as number) ?? 0.7;

  // Keep local state only for threshold input to handle typing
  const [thresholdInput, setThresholdInput] = useState(configThreshold.toString());

  // Sync local threshold input with config changes
  useEffect(() => {
    setThresholdInput(configThreshold.toString());
  }, [configThreshold]);

  const handleToggle = async (enabled: boolean) => {
    console.log('Security toggle changed to:', enabled);

    try {
      // Update the config
      await upsert('security_enabled', enabled, false);
      console.log('Security config updated successfully');
    } catch (error) {
      console.error('Failed to update security config:', error);
    }
  };

  const handleThresholdChange = async (threshold: number) => {
    // Update the config
    await upsert('security_threshold', threshold, false);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between py-2 px-2 hover:bg-background-muted rounded-lg transition-all">
        <div>
          <h3 className="text-text-default">Enable Prompt Injection Detection</h3>
          <p className="text-xs text-text-muted max-w-md mt-[2px]">
            Detect and prevent potential prompt injection attacks
          </p>
        </div>
        <div className="flex items-center">
          <Switch checked={enabled} onCheckedChange={handleToggle} variant="mono" />
        </div>
      </div>

      <div
        className={`overflow-hidden transition-all duration-300 ease-in-out ${
          enabled ? 'max-h-96 opacity-100' : 'max-h-0 opacity-0'
        }`}
      >
        <div className="space-y-3 px-2 pb-2">
          <div className={enabled ? '' : 'opacity-50'}>
            <label
              className={`text-sm font-medium ${enabled ? 'text-text-default' : 'text-text-muted'}`}
            >
              Detection Threshold
            </label>
            <p className="text-xs text-text-muted mb-2">
              Higher values are more strict (0.01 = very lenient, 1.0 = maximum strict)
            </p>
            <input
              type="number"
              min={0.01}
              max={1.0}
              step={0.01}
              value={thresholdInput}
              onChange={(e) => {
                // Update local input state immediately for responsive typing
                setThresholdInput(e.target.value);
              }}
              onBlur={(e) => {
                // Validate and save to config on blur
                let value = parseFloat(e.target.value);
                if (isNaN(value) || value < 0.01) {
                  value = 0.01;
                } else if (value > 1.0) {
                  value = 1.0;
                }
                // Update both local state and config
                setThresholdInput(value.toString());
                handleThresholdChange(value);
              }}
              disabled={!enabled}
              className={`w-24 px-2 py-1 text-sm border rounded ${
                enabled
                  ? 'border-border-default bg-background-default text-text-default'
                  : 'border-border-muted bg-background-muted text-text-muted cursor-not-allowed'
              }`}
              placeholder="0.70"
            />
          </div>
        </div>
      </div>
    </div>
  );
};
