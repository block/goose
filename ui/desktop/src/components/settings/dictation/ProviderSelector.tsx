import { useState, useEffect } from 'react';
import { ChevronDown, Check } from 'lucide-react';
import { DictationProvider, DictationSettings } from '../../../hooks/useDictationSettings';
import { DICTATION_PROVIDER_ELEVENLABS } from '../../../hooks/dictationConstants';
import { useConfig } from '../../ConfigContext';
import { ElevenLabsKeyInput } from './ElevenLabsKeyInput';
import { ProviderInfo } from './ProviderInfo';
import { VOICE_DICTATION_ELEVENLABS_ENABLED } from '../../../updates';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from '../../ui/dropdown-menu';

interface ProviderSelectorProps {
  settings: DictationSettings;
  onProviderChange: (provider: DictationProvider) => void;
}

export const ProviderSelector = ({ settings, onProviderChange }: ProviderSelectorProps) => {
  const [hasOpenAIKey, setHasOpenAIKey] = useState(false);
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

  const handleOpenChange = async (open: boolean) => {
    if (open) {
      try {
        const providers = await getProviders(true);
        const openAIProvider = providers.find((p) => p.name === 'openai');
        setHasOpenAIKey(!!openAIProvider?.is_configured);
      } catch (error) {
        console.error('Error checking OpenAI configuration:', error);
        setHasOpenAIKey(false);
      }
    }
  };

  const getProviderLabel = (provider: DictationProvider): string => {
    switch (provider) {
      case 'openai':
        return 'OpenAI Whisper';
      case DICTATION_PROVIDER_ELEVENLABS:
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
        <DropdownMenu onOpenChange={handleOpenChange}>
          <DropdownMenuTrigger asChild>
            <button className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border-subtle rounded-md hover:border-border-default transition-colors text-text-default bg-background-default">
              {getProviderLabel(settings.provider)}
              <ChevronDown className="w-4 h-4" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-48">
            <DropdownMenuItem onClick={() => onProviderChange('openai')}>
              <span className="flex-1">
                OpenAI Whisper
                {!hasOpenAIKey && <span className="text-xs ml-1">(not configured)</span>}
              </span>
              {settings.provider === 'openai' && (
                <Check className="w-4 h-4 ml-2 flex-shrink-0" />
              )}
            </DropdownMenuItem>

            {VOICE_DICTATION_ELEVENLABS_ENABLED && (
              <DropdownMenuItem
                onClick={() => onProviderChange(DICTATION_PROVIDER_ELEVENLABS)}
              >
                <span className="flex-1">ElevenLabs</span>
                {settings.provider === DICTATION_PROVIDER_ELEVENLABS && (
                  <Check className="w-4 h-4 ml-2 flex-shrink-0" />
                )}
              </DropdownMenuItem>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {VOICE_DICTATION_ELEVENLABS_ENABLED &&
        settings.provider === DICTATION_PROVIDER_ELEVENLABS && <ElevenLabsKeyInput />}

      <ProviderInfo provider={settings.provider} />
    </div>
  );
};
