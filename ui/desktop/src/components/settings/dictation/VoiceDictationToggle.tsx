import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { DictationProvider, DictationSettings } from '../../../hooks/useDictationSettings';
import {
  DICTATION_SETTINGS_KEY,
  DICTATION_PROVIDER_ELEVENLABS,
  getDefaultDictationSettings,
} from '../../../hooks/dictationConstants';
import { useConfig } from '../../ConfigContext';
import { ProviderSelector } from './ProviderSelector';
import { MicrophoneSelector } from './MicrophoneSelector';
import { VOICE_DICTATION_ELEVENLABS_ENABLED } from '../../../updates';
import { trackSettingToggled } from '../../../utils/analytics';

export const VoiceDictationToggle = () => {
  const [settings, setSettings] = useState<DictationSettings>({
    enabled: false,
    provider: null,
    preferredDeviceId: null,
  });
  const { getProviders } = useConfig();

  useEffect(() => {
    const loadSettings = async () => {
      const savedSettings = localStorage.getItem(DICTATION_SETTINGS_KEY);

      let loadedSettings: DictationSettings;

      if (savedSettings) {
        const parsed = JSON.parse(savedSettings);
        // Ensure backward compatibility: add preferredDeviceId if missing
        loadedSettings = { preferredDeviceId: null, ...parsed };

        // If ElevenLabs is disabled and user has it selected, reset to OpenAI
        if (
          !VOICE_DICTATION_ELEVENLABS_ENABLED &&
          loadedSettings.provider === DICTATION_PROVIDER_ELEVENLABS
        ) {
          loadedSettings = {
            ...loadedSettings,
            provider: 'openai',
          };
          localStorage.setItem(DICTATION_SETTINGS_KEY, JSON.stringify(loadedSettings));
        }
      } else {
        loadedSettings = await getDefaultDictationSettings(getProviders);
      }

      setSettings(loadedSettings);
    };

    loadSettings();
  }, [getProviders]);

  const saveSettings = (newSettings: DictationSettings) => {
    setSettings(newSettings);
    localStorage.setItem(DICTATION_SETTINGS_KEY, JSON.stringify(newSettings));
    window.dispatchEvent(new CustomEvent('dictation-settings-changed'));
  };

  const handleToggle = (enabled: boolean) => {
    saveSettings({
      ...settings,
      enabled,
      provider: settings.provider === null ? 'openai' : settings.provider,
    });
    trackSettingToggled('voice_dictation', enabled);
  };

  const handleProviderChange = (provider: DictationProvider) => {
    saveSettings({ ...settings, provider });
  };

  const handleDeviceChange = (deviceId: string | null) => {
    saveSettings({ ...settings, preferredDeviceId: deviceId });
  };

  return (
    <div className="space-y-1">
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

      <div
        className={`transition-all duration-300 ease-in-out ${
          settings.enabled
            ? 'max-h-[800px] opacity-100 mt-2 overflow-visible'
            : 'max-h-0 opacity-0 mt-0 overflow-hidden'
        }`}
      >
        <div className="space-y-3 pb-2">
          <ProviderSelector settings={settings} onProviderChange={handleProviderChange} />
          <MicrophoneSelector
            selectedDeviceId={settings.preferredDeviceId}
            onDeviceChange={handleDeviceChange}
          />
        </div>
      </div>
    </div>
  );
};
