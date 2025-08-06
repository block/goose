import { useState, useEffect, useRef } from 'react';
import { Switch } from '../../ui/switch';
import { ChevronDown } from 'lucide-react';
import { Input } from '../../ui/input';
import { useConfig } from '../../ConfigContext';

type DictationProvider = 'openai' | 'elevenlabs';

interface DictationSettings {
  enabled: boolean;
  provider: DictationProvider;
}

const DICTATION_SETTINGS_KEY = 'dictation_settings';
const ELEVENLABS_API_KEY = 'ELEVENLABS_API_KEY';

export default function DictationSection() {
  const [settings, setSettings] = useState<DictationSettings>({
    enabled: true,
    provider: 'openai',
  });
  const [hasOpenAIKey, setHasOpenAIKey] = useState(false);
  const [showProviderDropdown, setShowProviderDropdown] = useState(false);
  const [showElevenLabsKey, setShowElevenLabsKey] = useState(false);
  const [elevenLabsApiKey, setElevenLabsApiKey] = useState('');
  const [isLoadingKey, setIsLoadingKey] = useState(false);
  const [hasElevenLabsKey, setHasElevenLabsKey] = useState(false);
  const elevenLabsApiKeyRef = useRef('');

  const { getProviders, upsert, read } = useConfig();

  // Load settings from localStorage and ElevenLabs API key from secure storage
  useEffect(() => {
    const loadSettings = async () => {
      const savedSettings = localStorage.getItem(DICTATION_SETTINGS_KEY);
      if (savedSettings) {
        const parsed = JSON.parse(savedSettings);
        setSettings(parsed);
        setShowElevenLabsKey(parsed.provider === 'elevenlabs');
      } else {
        // Default settings
        const defaultSettings: DictationSettings = {
          enabled: true,
          provider: 'openai',
        };
        setSettings(defaultSettings);
        localStorage.setItem(DICTATION_SETTINGS_KEY, JSON.stringify(defaultSettings));
      }

      // Load ElevenLabs API key from storage
      setIsLoadingKey(true);
      try {
        // Try reading as secret - will return true if exists
        const keyExists = await read(ELEVENLABS_API_KEY, true);
        if (keyExists === true) {
          setHasElevenLabsKey(true);
          // Don't set the actual key since we can't read secrets
        }
      } catch (error) {
        console.error('Error checking ElevenLabs API key:', error);
      } finally {
        setIsLoadingKey(false);
      }
    };

    loadSettings();
  }, [read]);

  // Save ElevenLabs key on unmount if it has changed
  useEffect(() => {
    return () => {
      if (showElevenLabsKey && elevenLabsApiKeyRef.current) {
        // We can't use async in cleanup, so we'll use the promise directly
        const keyToSave = elevenLabsApiKeyRef.current;
        if (keyToSave.trim()) {
          upsert(ELEVENLABS_API_KEY, keyToSave, true).catch((error) => {
            console.error('Error saving ElevenLabs API key on unmount:', error);
          });
        }
      }
    };
  }, [showElevenLabsKey, upsert]);

  // Check if OpenAI is configured
  useEffect(() => {
    const checkOpenAIKey = async () => {
      try {
        const providers = await getProviders(false);
        const openAIProvider = providers.find((p) => p.name === 'openai');
        setHasOpenAIKey(openAIProvider?.is_configured || false);
      } catch (error) {
        console.error('Error checking OpenAI configuration:', error);
        setHasOpenAIKey(false);
      }
    };

    checkOpenAIKey();
  }, [getProviders]);

  const handleDropdownToggle = async () => {
    const newShowState = !showProviderDropdown;
    setShowProviderDropdown(newShowState);

    if (newShowState) {
      try {
        const providers = await getProviders(true);
        const openAIProvider = providers.find((p) => p.name === 'openai');
        const isConfigured = !!openAIProvider?.is_configured;
        setHasOpenAIKey(isConfigured);
      } catch (error) {
        console.error('Error checking OpenAI configuration:', error);
        setHasOpenAIKey(false);
      }
    }
  };

  const saveSettings = (newSettings: DictationSettings) => {
    setSettings(newSettings);
    localStorage.setItem(DICTATION_SETTINGS_KEY, JSON.stringify(newSettings));
  };

  const handleToggle = (enabled: boolean) => {
    saveSettings({ ...settings, enabled });
  };

  const handleProviderChange = (provider: DictationProvider) => {
    saveSettings({ ...settings, provider });
    setShowProviderDropdown(false);
    setShowElevenLabsKey(provider === 'elevenlabs');
  };

  const handleElevenLabsKeyChange = (key: string) => {
    setElevenLabsApiKey(key);
    elevenLabsApiKeyRef.current = key;
  };

  const saveElevenLabsKey = async () => {
    // Save to secure storage
    try {
      if (elevenLabsApiKey.trim()) {
        await upsert(ELEVENLABS_API_KEY, elevenLabsApiKey, true);
        setHasElevenLabsKey(true);
      } else {
        // If key is empty, remove it from storage
        await upsert(ELEVENLABS_API_KEY, null, true);
        setHasElevenLabsKey(false);
      }
    } catch (error) {
      console.error('Error saving ElevenLabs API key:', error);
    }
  };

  const getProviderLabel = (provider: DictationProvider): string => {
    switch (provider) {
      case 'openai':
        return 'OpenAI Whisper';
      case 'elevenlabs':
        return 'ElevenLabs';
      default:
        return provider;
    }
  };

  return (
    <section id="dictation" className="px-4">
      {/* Enable/Disable Toggle */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-textStandard">Enable Voice Dictation</h3>
          <p className="text-xs text-textSubtle max-w-md mt-[2px]">
            Show microphone button for voice input
          </p>
        </div>
        <div className="flex items-center">
          <Switch checked={settings.enabled} onCheckedChange={handleToggle} variant="mono" />
        </div>
      </div>

      {/* Provider Selection */}
      {settings.enabled && (
        <>
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-textStandard">Dictation Provider</h3>
              <p className="text-xs text-textSubtle max-w-md mt-[2px]">
                Choose how voice is converted to text
              </p>
            </div>
            <div className="relative">
              <button
                onClick={handleDropdownToggle}
                className="flex items-center gap-2 px-3 py-1.5 text-sm border border-borderSubtle rounded-md hover:border-borderStandard transition-colors text-textStandard bg-background-default"
              >
                {getProviderLabel(settings.provider)}
                <ChevronDown className="w-4 h-4" />
              </button>

              {showProviderDropdown && (
                <div className="absolute right-0 mt-1 w-48 bg-background-default border border-borderStandard rounded-md shadow-lg z-10">
                  <button
                    onClick={() => handleProviderChange('openai')}
                    className="w-full px-3 py-2 text-left text-sm transition-colors first:rounded-t-md hover:bg-bgSubtle text-textStandard"
                  >
                    OpenAI Whisper
                    {!hasOpenAIKey && <span className="text-xs ml-1">(not configured)</span>}
                    {settings.provider === 'openai' && <span className="float-right">✓</span>}
                  </button>

                  {/* ElevenLabs option */}
                  <button
                    onClick={() => handleProviderChange('elevenlabs')}
                    className="w-full px-3 py-2 text-left text-sm hover:bg-bgSubtle transition-colors text-textStandard last:rounded-b-md"
                  >
                    ElevenLabs
                    {settings.provider === 'elevenlabs' && <span className="float-right">✓</span>}
                  </button>
                </div>
              )}
            </div>
          </div>

          {/* ElevenLabs API Key */}
          {showElevenLabsKey && (
            <div className="mb-4">
              <div className="mb-2">
                <h3 className="text-textStandard">ElevenLabs API Key</h3>
                <p className="text-xs text-textSubtle max-w-md mt-[2px]">
                  Required for ElevenLabs voice recognition
                  {hasElevenLabsKey && <span className="text-green-600 ml-2">(Configured)</span>}
                </p>
              </div>
              <Input
                type="password"
                value={elevenLabsApiKey}
                onChange={(e) => handleElevenLabsKeyChange(e.target.value)}
                onBlur={saveElevenLabsKey}
                placeholder={
                  hasElevenLabsKey ? 'Enter new API key to update' : 'Enter your ElevenLabs API key'
                }
                className="max-w-md"
                disabled={isLoadingKey}
              />
            </div>
          )}

          {/* Provider-specific information */}
          <div className="mt-4 p-3 bg-bgSubtle rounded-md">
            {settings.provider === 'openai' && (
              <p className="text-xs text-textSubtle">
                Uses OpenAI's Whisper API for high-quality transcription. Requires an OpenAI API key
                configured in the Models section.
              </p>
            )}
            {settings.provider === 'elevenlabs' && (
              <div>
                <p className="text-xs text-textSubtle">
                  Uses ElevenLabs speech-to-text API for high-quality transcription.
                </p>
                <p className="text-xs text-textSubtle mt-2">
                  <strong>Features:</strong>
                </p>
                <ul className="text-xs text-textSubtle ml-4 mt-1 list-disc">
                  <li>Advanced voice processing</li>
                  <li>High accuracy transcription</li>
                  <li>Multiple language support</li>
                  <li>Fast processing</li>
                </ul>
                <p className="text-xs text-textSubtle mt-2">
                  <strong>Note:</strong> Requires an ElevenLabs API key with speech-to-text access.
                </p>
              </div>
            )}
          </div>
        </>
      )}
    </section>
  );
}
