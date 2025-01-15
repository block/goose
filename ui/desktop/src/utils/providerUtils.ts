export const SELECTED_PROVIDER_KEY = "GOOSE_PROVIDER__API_KEY"

export interface ProviderOption {
  id: 'openai' | 'anthropic';
  name: string;
  description: string;
  modelExample: string;
}

export const OPENAI_ENDPOINT_PLACEHOLDER = "https://api.openai.com";
export const ANTHROPIC_ENDPOINT_PLACEHOLDER = "https://api.anthropic.com";
export const OPENAI_DEFAULT_MODEL = "gpt-4"
export const ANTHROPIC_DEFAULT_MODEL = "claude-3-sonnet"

export const providers: ProviderOption[] = [
  {
    id: 'openai',
    name: 'OpenAI',
    description: 'Use GPT-4 and other OpenAI models',
    modelExample: 'gpt-4-turbo'
  },
  {
    id: 'anthropic',
    name: 'Anthropic',
    description: 'Use Claude and other Anthropic models',
    modelExample: 'claude-3-sonnet'
  }
];

export const getCurrentProvider = (): string => {
  const provider = localStorage.getItem(SELECTED_PROVIDER_KEY);
  console.log('Getting current provider:', provider || 'none');
  return provider || 'openai'; // default to OpenAI if none selected
};

export const getProviderKeyName = (providerId: string): string => {
  return providerId === 'openai' ? 'OPENAI_API_KEY' : 'ANTHROPIC_API_KEY';
}; 