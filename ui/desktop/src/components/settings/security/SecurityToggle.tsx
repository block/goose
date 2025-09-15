import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { useConfig } from '../../ConfigContext';

interface SecuritySettings {
  enabled: boolean;
  threshold: number;
}

export const SecurityToggle = () => {
  const [settings, setSettings] = useState<SecuritySettings>({
    enabled: false,
    threshold: 0.7,
  });
  const { config, upsert } = useConfig();

  useEffect(() => {
    // Load security settings from config
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
              Detection Threshold: {settings.threshold.toFixed(2)}
            </label>
            <p className="text-xs text-text-muted mb-2">
              Higher values are more strict (0.1 = lenient, 0.9 = strict)
            </p>
            <input
              type="range"
              min={0.1}
              max={0.9}
              step={0.1}
              value={settings.threshold}
              onChange={(e) => handleThresholdChange(parseFloat(e.target.value))}
              disabled={!settings.enabled}
              className={`w-full h-2 rounded-lg appearance-none cursor-pointer ${
                settings.enabled ? 'bg-gray-200' : 'bg-gray-100 opacity-50 cursor-not-allowed'
              }`}
            />
          </div>
        </div>
      </div>
    </div>
  );
};
