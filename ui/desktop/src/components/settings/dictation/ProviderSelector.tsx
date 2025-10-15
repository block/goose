import { useState, useEffect } from 'react';
import { ChevronDown } from 'lucide-react';
import { DictationProvider, DictationSettings } from '../../../hooks/useDictationSettings';
import { useConfig } from '../../ConfigContext';
import { ElevenLabsKeyInput } from './ElevenLabsKeyInput';
import { ProviderInfo } from './ProviderInfo';
import { VOICE_DICTATION_ELEVENLABS_ENABLED } from '../../../updates';

interface ProviderSelectorProps {
  settings: DictationSettings;
  onProviderChange: (provider: DictationProvider) => void;
}

export const ProviderSelector = ({ settings, onProviderChange }: ProviderSelectorProps) => {
  const [hasOpenAIKey, setHasOpenAIKey] = useState(false);
  const [showProviderDropdown, setShowProviderDropdown] = useState(false);
  const { getProviders } = useConfig();

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

  const handleProviderChange = (provider: DictationProvider) => {
    onProviderChange(provider);
    setShowProviderDropdown(false);
  };

  const getProviderLabel = (provider: DictationProvider): string => {
    switch (provider) {
      case 'openai':
        return 'OpenAI Whisper';
      case 'elevenlabs':
        return 'ElevenLabs';
      default:
        return 'None (disabled)';
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between py-2 px-2 hover:bg-background-muted rounded-lg transition-all">
        <div>
          <h3 className="text-text-default">Dictation Provider</h3>
          <p className="text-xs text-text-muted max-w-md mt-[2px]">
            Choose how voice is converted to text
          </p>
        </div>
        <div className="relative">
          <button
            onClick={handleDropdownToggle}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border-subtle rounded-md hover:border-border-default transition-colors text-text-default bg-background-default"
          >
            {getProviderLabel(settings.provider)}
            <ChevronDown className="w-4 h-4" />
          </button>

          {showProviderDropdown && (
            <div className="absolute right-0 mt-1 w-48 bg-background-default border border-border-default rounded-md shadow-lg z-10">
              <button
                onClick={() => handleProviderChange('openai')}
                className={`w-full px-3 py-2 text-left text-sm transition-colors hover:bg-background-subtle text-text-default ${!VOICE_DICTATION_ELEVENLABS_ENABLED ? 'first:rounded-t-md last:rounded-b-md' : 'first:rounded-t-md'}`}
              >
                OpenAI Whisper
                {!hasOpenAIKey && <span className="text-xs ml-1">(not configured)</span>}
                {settings.provider === 'openai' && <span className="float-right">✓</span>}
              </button>

              {VOICE_DICTATION_ELEVENLABS_ENABLED && (
                <button
                  onClick={() => handleProviderChange('elevenlabs')}
                  className="w-full px-3 py-2 text-left text-sm hover:bg-background-subtle transition-colors text-text-default last:rounded-b-md"
                >
                  ElevenLabs
                  {settings.provider === 'elevenlabs' && <span className="float-right">✓</span>}
                </button>
              )}
            </div>
          )}
        </div>
      </div>

      {VOICE_DICTATION_ELEVENLABS_ENABLED && settings.provider === 'elevenlabs' && (
        <ElevenLabsKeyInput />
      )}

      <ProviderInfo provider={settings.provider} />
    </div>
  );
};
