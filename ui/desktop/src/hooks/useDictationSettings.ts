import { useState, useEffect } from 'react';
import { useConfig } from '../components/ConfigContext';
import {
  DICTATION_SETTINGS_KEY,
  ELEVENLABS_API_KEY,
  getDefaultDictationSettings,
} from './dictationConstants';

export type DictationProvider = 'openai' | 'elevenlabs' | null;

export interface DictationSettings {
  enabled: boolean;
  provider: DictationProvider;
}

export const useDictationSettings = () => {
  const [settings, setSettings] = useState<DictationSettings | null>(null);
  const [hasElevenLabsKey, setHasElevenLabsKey] = useState<boolean>(false);
  const { read, getProviders } = useConfig();

  useEffect(() => {
    const loadSettings = async () => {
      // Load settings from localStorage
      const saved = localStorage.getItem(DICTATION_SETTINGS_KEY);

      if (saved) {
        const parsedSettings = JSON.parse(saved);
        setSettings(parsedSettings);
      } else {
        const defaultSettings = await getDefaultDictationSettings(getProviders);
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
  }, [read, getProviders]);

  return { settings, hasElevenLabsKey };
};
