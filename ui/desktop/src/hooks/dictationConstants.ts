import { DictationSettings, DictationProvider } from './useDictationSettings';

export const DICTATION_SETTINGS_KEY = 'dictation_settings';
export const ELEVENLABS_API_KEY = 'ELEVENLABS_API_KEY';
export const DICTATION_PROVIDER_ELEVENLABS = 'elevenlabs' as const;

export const isSecretKeyConfigured = (response: unknown): boolean =>
  typeof response === 'object' &&
  response !== null &&
  'maskedValue' in response &&
  !!(response as { maskedValue: string }).maskedValue;

export type SecretKeyResponse =
  | string
  | boolean
  | null
  | { masked_value?: string; maskedValue?: string };

export const isSecretKeyConfigured = (response: SecretKeyResponse): boolean => {
  if (response === null || response === '' || response === false) {
    return false;
  }

  if (typeof response === 'object') {
    const maskedValue = 'maskedValue' in response ? response.maskedValue : response.masked_value;
    return !!maskedValue;
  }

  return typeof response === 'string' && response.length > 0;
};

export const getDefaultDictationSettings = async (
  getProviders: (refresh: boolean) => Promise<Array<{ name: string; is_configured: boolean }>>
): Promise<DictationSettings> => {
  const providers = await getProviders(false);
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
