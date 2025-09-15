import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { useConfig } from '../../ConfigContext';

interface SecuritySettings {
  enabled: boolean;
  threshold: number;
}

export const SecurityToggle = () => {
  const { config, upsert } = useConfig();

  // Initialize settings with defaults
  const [settings, setSettings] = useState<SecuritySettings>({
    enabled: false,
    threshold: 0.7,
  });

  useEffect(() => {
    // Load security settings from config when config changes
    if (config && 'security' in config && config.security) {
      const securityConfig = config.security as { enabled?: boolean; threshold?: number };
      setSettings({
        enabled: securityConfig.enabled || false,
        threshold: securityConfig.threshold || 0.7,
      });
    }
  }, [config]);

  const handleToggle = async (enabled: boolean) => {
    console.log('Security toggle changed to:', enabled);
    const newSettings = { ...settings, enabled };
    setSettings(newSettings);

    try {
      // Update the config
      await upsert('security.enabled', enabled, false);
      console.log('Security config updated successfully');
    } catch (error) {
      console.error('Failed to update security config:', error);
    }
  };

  const handleThresholdChange = async (threshold: number) => {
    const newSettings = { ...settings, threshold };
    setSettings(newSettings);

    // Update the config
    await upsert('security.threshold', threshold, false);
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
          <Switch checked={settings.enabled} onCheckedChange={handleToggle} variant="mono" />
        </div>
      </div>

      <div
        className={`overflow-hidden transition-all duration-300 ease-in-out ${
          settings.enabled ? 'max-h-96 opacity-100' : 'max-h-0 opacity-0'
        }`}
      >
        <div className="space-y-3 px-2 pb-2">
          <div className={settings.enabled ? '' : 'opacity-50'}>
            <label
              className={`text-sm font-medium ${
                settings.enabled ? 'text-text-default' : 'text-text-muted'
              }`}
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
              value={settings.threshold}
              onChange={(e) => {
                // Allow any input during typing, update local state immediately
                const value = parseFloat(e.target.value);
                if (!isNaN(value)) {
                  setSettings((prev) => ({ ...prev, threshold: value }));
                } else if (e.target.value === '') {
                  // Allow empty field during editing
                  setSettings((prev) => ({ ...prev, threshold: 0 }));
                }
              }}
              onBlur={(e) => {
                // Validate and save to config on blur
                let value = parseFloat(e.target.value);
                if (isNaN(value) || value < 0.01) {
                  value = 0.01;
                } else if (value > 1.0) {
                  value = 1.0;
                }
                setSettings((prev) => ({ ...prev, threshold: value }));
                handleThresholdChange(value);
              }}
              disabled={!settings.enabled}
              className={`w-24 px-2 py-1 text-sm border rounded ${
                settings.enabled
                  ? 'border-gray-300 bg-white text-text-default'
                  : 'border-gray-200 bg-gray-100 text-text-muted cursor-not-allowed'
              }`}
              placeholder="0.70"
            />
          </div>
        </div>
      </div>
    </div>
  );
};
