import { DictationSettings, DictationProvider } from './useDictationSettings';

export const DICTATION_SETTINGS_KEY = 'dictation_settings';
export const ELEVENLABS_API_KEY = 'ELEVENLABS_API_KEY';

export const getDefaultDictationSettings = async (
  getProviders: (refresh: boolean) => Promise<Array<{ name: string; is_configured: boolean }>>
): Promise<DictationSettings> => {
  const providers = await getProviders(false);

  // Check if we have an OpenAI API key as primary default
  const openAIProvider = providers.find((p) => p.name === 'openai');

  if (openAIProvider && openAIProvider.is_configured) {
    return {
      enabled: true,
      provider: 'openai' as DictationProvider,
    };
  } else {
    return {
      enabled: false,
      provider: null as DictationProvider,
    };
  }
};
