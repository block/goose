import { useState, useEffect } from 'react';
import { useConfig } from '../components/ConfigContext';
import {
  DICTATION_SETTINGS_KEY,
  ELEVENLABS_API_KEY,
  DICTATION_PROVIDER_ELEVENLABS,
  getDefaultDictationSettings,
} from './dictationConstants';

export type DictationProvider = 'openai' | typeof DICTATION_PROVIDER_ELEVENLABS | null;

export interface DictationSettings {
  enabled: boolean;
  provider: DictationProvider;
}

let elevenLabsKeyCache: boolean | null = null;

export const setElevenLabsKeyCache = (value: boolean) => {
  elevenLabsKeyCache = value;
};

export const useDictationSettings = () => {
  const [settings, setSettings] = useState<DictationSettings | null>(null);
  const [hasElevenLabsKey, setHasElevenLabsKey] = useState<boolean>(elevenLabsKeyCache ?? false);
  const { read, getProviders } = useConfig();

  useEffect(() => {
    const loadSettings = async () => {
      // Load settings from localStorage
      const saved = localStorage.getItem(DICTATION_SETTINGS_KEY);

      let currentSettings: DictationSettings;
      if (saved) {
        currentSettings = JSON.parse(saved);
      } else {
        currentSettings = await getDefaultDictationSettings(getProviders);
      }
      setSettings(currentSettings);

      if (
        currentSettings.provider === DICTATION_PROVIDER_ELEVENLABS &&
        elevenLabsKeyCache === null
      ) {
        try {
          const keyExists = await read(ELEVENLABS_API_KEY, true);
          const hasKey = keyExists === true;
          elevenLabsKeyCache = hasKey;
          setHasElevenLabsKey(hasKey);
        } catch (error) {
          elevenLabsKeyCache = false;
          setHasElevenLabsKey(false);
          console.error('[useDictationSettings] Error loading ElevenLabs API key:', error);
        }
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
