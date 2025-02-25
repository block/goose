import { Model } from './ModelContext';

// TODO: move into backends / fetch dynamically
export const goose_models: Model[] = [
  { id: 1, name: 'gpt-4o-mini', provider: 'OpenAI' },
  { id: 2, name: 'gpt-4o', provider: 'OpenAI' },
  { id: 3, name: 'gpt-4-turbo', provider: 'OpenAI' },
  { id: 5, name: 'o1', provider: 'OpenAI' },
  { id: 7, name: 'claude-3-5-sonnet-latest', provider: 'Anthropic' },
  { id: 8, name: 'claude-3-5-haiku-latest', provider: 'Anthropic' },
  { id: 9, name: 'claude-3-opus-latest', provider: 'Anthropic' },
  { id: 10, name: 'gemini-1.5-pro', provider: 'Google' },
  { id: 11, name: 'gemini-1.5-flash', provider: 'Google' },
  { id: 12, name: 'gemini-2.0-flash', provider: 'Google' },
  { id: 13, name: 'gemini-2.0-flash-lite-preview-02-05', provider: 'Google' },
  { id: 14, name: 'gemini-2.0-flash-thinking-exp-01-21', provider: 'Google' },
  { id: 15, name: 'gemini-2.0-pro-exp-02-05', provider: 'Google' },
  { id: 16, name: 'llama-3.3-70b-versatile', provider: 'Groq' },
  { id: 17, name: 'qwen2.5', provider: 'Ollama' },
  { id: 18, name: 'anthropic/claude-3.5-sonnet', provider: 'OpenRouter' },
  { id: 19, name: 'gpt-4o', provider: 'Azure OpenAI' },
];

export const openai_models = ['gpt-4o-mini', 'gpt-4o', 'gpt-4-turbo', 'o1'];

export const anthropic_models = [
  'claude-3-5-sonnet-latest',
  'claude-3-5-sonnet-2',
  'claude-3-5-haiku-latest',
  'claude-3-opus-latest',
];

export const google_models = [
  'gemini-1.5-pro',
  'gemini-1.5-flash',
  'gemini-2.0-flash',
  'gemini-2.0-flash-lite-preview-02-05',
  'gemini-2.0-flash-thinking-exp-01-21',
  'gemini-2.0-pro-exp-02-05',
];

export const groq_models = ['llama-3.3-70b-versatile'];

export const ollama_mdoels = ['qwen2.5'];

export const openrouter_models = ['anthropic/claude-3.5-sonnet'];

export const azure_openai_models = ['gpt-4o'];

export const default_models = {
  openai: 'gpt-4o',
  anthropic: 'claude-3-5-sonnet-latest',
  databricks: 'goose',
  google: 'gemini-2.0-flash-exp',
  groq: 'llama-3.3-70b-versatile',
  openrouter: 'anthropic/claude-3.5-sonnet',
  ollama: 'qwen2.5',
  azure_openai: 'gpt-4o',
};

export function getDefaultModel(key: string): string | undefined {
  return default_models[key] || undefined;
}

export const short_list = ['gpt-4o', 'claude-3-5-sonnet-latest'];

export const required_keys = {
  OpenAI: ['OPENAI_API_KEY', 'OPENAI_HOST', 'OPENAI_BASE_PATH'],
  Anthropic: ['ANTHROPIC_API_KEY'],
  Databricks: ['DATABRICKS_HOST'],
  Groq: ['GROQ_API_KEY'],
  Ollama: ['OLLAMA_HOST'],
  Google: ['GOOGLE_API_KEY'],
  OpenRouter: ['OPENROUTER_API_KEY'],
  'Azure OpenAI': ['AZURE_OPENAI_API_KEY', 'AZURE_OPENAI_ENDPOINT', 'AZURE_OPENAI_DEPLOYMENT_NAME'],
};

export const default_key_value = {
  OPENAI_HOST: 'https://api.openai.com',
  OPENAI_BASE_PATH: 'v1/chat/completions',
  OLLAMA_HOST: 'localhost',
};

export const supported_providers = [
  'OpenAI',
  'Anthropic',
  'Databricks',
  'Groq',
  'Google',
  'Ollama',
  'OpenRouter',
  'Azure OpenAI',
];

export const model_docs_link = [
  { name: 'OpenAI', href: 'https://platform.openai.com/docs/models' },
  { name: 'Anthropic', href: 'https://docs.anthropic.com/en/docs/about-claude/models' },
  { name: 'Google', href: 'https://ai.google/get-started/our-models/' },
  { name: 'Groq', href: 'https://console.groq.com/docs/models' },
  {
    name: 'Databricks',
    href: 'https://docs.databricks.com/en/generative-ai/external-models/index.html',
  },
  { name: 'OpenRouter', href: 'https://openrouter.ai/models' },
  { name: 'Ollama', href: 'https://ollama.com/library' },
];

export const provider_aliases = [
  { provider: 'OpenAI', alias: 'openai' },
  { provider: 'Anthropic', alias: 'anthropic' },
  { provider: 'Ollama', alias: 'ollama' },
  { provider: 'Groq', alias: 'groq' },
  { provider: 'Databricks', alias: 'databricks' },
  { provider: 'OpenRouter', alias: 'openrouter' },
  { provider: 'Google', alias: 'google' },
  { provider: 'Azure OpenAI', alias: 'azure_openai' },
];
