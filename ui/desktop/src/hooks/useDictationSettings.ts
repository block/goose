import { useState, useEffect } from 'react';
import { useConfig } from '../components/ConfigContext';

export type DictationProvider = 'openai' | 'elevenlabs';

export interface DictationSettings {
  enabled: boolean;
  provider: DictationProvider;
}

const DICTATION_SETTINGS_KEY = 'dictation_settings';
const ELEVENLABS_API_KEY = 'ELEVENLABS_API_KEY';

export const useDictationSettings = () => {
  const [settings, setSettings] = useState<DictationSettings | null>(null);
  const [hasElevenLabsKey, setHasElevenLabsKey] = useState<boolean>(false);
  const { read } = useConfig();

  useEffect(() => {
    const loadSettings = async () => {
      // Load settings from localStorage
      const saved = localStorage.getItem(DICTATION_SETTINGS_KEY);
      if (saved) {
        setSettings(JSON.parse(saved));
      } else {
        // Default settings
        const defaultSettings: DictationSettings = {
          enabled: true,
          provider: 'openai',
        };
        setSettings(defaultSettings);
      }

      // Load ElevenLabs API key from storage (non-secret for frontend access)
      try {
        const keyExists = await read(ELEVENLABS_API_KEY, true);
        if (keyExists === true) {
          setHasElevenLabsKey(true);
        }
      } catch (error) {
        console.error('[useDictationSettings] Error loading ElevenLabs API key:', error);
      }
    };

    loadSettings();

    // Listen for storage changes from other tabs/windows
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const handleStorageChange = (e: any) => {
      if (e.key === DICTATION_SETTINGS_KEY && e.newValue) {
        setSettings(JSON.parse(e.newValue));
      }
    };

    window.addEventListener('storage', handleStorageChange);
    return () => window.removeEventListener('storage', handleStorageChange);
  }, [read]);

  return { settings, hasElevenLabsKey };
};
