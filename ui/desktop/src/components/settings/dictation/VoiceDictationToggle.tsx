import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { DictationProvider, DictationSettings } from '../../../hooks/useDictationSettings';
import {
  DICTATION_SETTINGS_KEY,
  getDefaultDictationSettings,
} from '../../../hooks/dictationConstants';
import { useConfig } from '../../ConfigContext';
import { ProviderSelector } from './ProviderSelector';

export const VoiceDictationToggle = () => {
  const [settings, setSettings] = useState<DictationSettings>({
    enabled: false,
    provider: null,
  });
  const { getProviders } = useConfig();

  // Load settings from localStorage
  useEffect(() => {
    const loadSettings = async () => {
      const savedSettings = localStorage.getItem(DICTATION_SETTINGS_KEY);

      let loadedSettings: DictationSettings;

      if (savedSettings) {
        const parsed = JSON.parse(savedSettings);
        loadedSettings = parsed;
      } else {
        loadedSettings = await getDefaultDictationSettings(getProviders);
      }

      setSettings(loadedSettings);
    };

    loadSettings();
  }, [getProviders]);

  const saveSettings = (newSettings: DictationSettings) => {
    console.log('Saving dictation settings to localStorage:', newSettings);
    setSettings(newSettings);
    localStorage.setItem(DICTATION_SETTINGS_KEY, JSON.stringify(newSettings));
  };

  const handleToggle = (enabled: boolean) => {
    saveSettings({
      ...settings,
      enabled,
      provider: settings.provider === null ? 'openai' : settings.provider,
    });
  };

  const handleProviderChange = (provider: DictationProvider) => {
    saveSettings({ ...settings, provider });
  };

  return (
    <div className="space-y-4">
      {/* Enable/Disable Toggle */}
      <div className="flex items-center justify-between py-2 px-2 hover:bg-background-muted rounded-lg transition-all">
        <div>
          <h3 className="text-text-default">Enable Voice Dictation</h3>
          <p className="text-xs text-text-muted max-w-md mt-[2px]">
            Show microphone button for voice input
          </p>
        </div>
        <div className="flex items-center">
          <Switch checked={settings.enabled} onCheckedChange={handleToggle} variant="mono" />
        </div>
      </div>

      {/* Provider Selection and Configuration (conditional) */}
      {settings.enabled && (
        <ProviderSelector 
          settings={settings}
          onProviderChange={handleProviderChange}
        />
      )}
    </div>
  );
};
