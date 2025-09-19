import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { useConfig } from '../../ConfigContext';

interface SecurityConfig {
  security_enabled?: boolean;
  security_threshold?: number;
}

export const SecurityToggle = () => {
  const { config, upsert } = useConfig();

  const { security_enabled: enabled = false, security_threshold: configThreshold = 0.7 } =
    (config as SecurityConfig) ?? {};

  const [thresholdInput, setThresholdInput] = useState(configThreshold.toString());

  useEffect(() => {
    setThresholdInput(configThreshold.toString());
  }, [configThreshold]);

  const handleToggle = async (enabled: boolean) => {
    try {
      await upsert('security_enabled', enabled, false);
    } catch (error) {
      console.error('Failed to update security config:', error);
    }
  };

  const handleThresholdChange = async (threshold: number) => {
    const validThreshold = Math.max(0, Math.min(1, threshold));
    try {
      await upsert('security_threshold', validThreshold, false);
    } catch (error) {
      console.error('Failed to update threshold:', error);
    }
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
