import { Model } from './ModelContext';

// TODO: move into backends / fetch dynamically
// this is used by ModelContext
export const gooseModels: Model[] = [
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
  { id: 16, name: 'gemini-2.5-pro-exp-03-25', provider: 'Google' },
  { id: 17, name: 'llama-3.3-70b-versatile', provider: 'Groq' },
  { id: 18, name: 'qwen2.5', provider: 'Ollama' },
  { id: 19, name: 'anthropic/claude-3.5-sonnet', provider: 'OpenRouter' },
  { id: 20, name: 'gpt-4o', provider: 'Azure OpenAI' },
  { id: 21, name: 'claude-3-7-sonnet@20250219', provider: 'GCP Vertex AI' },
  { id: 22, name: 'claude-3-5-sonnet-v2@20241022', provider: 'GCP Vertex AI' },
  { id: 23, name: 'claude-3-5-sonnet@20240620', provider: 'GCP Vertex AI' },
  { id: 24, name: 'claude-3-5-haiku@20241022', provider: 'GCP Vertex AI' },
  { id: 25, name: 'gemini-2.0-pro-exp-02-05', provider: 'GCP Vertex AI' },
  { id: 26, name: 'gemini-2.0-flash-001', provider: 'GCP Vertex AI' },
  { id: 27, name: 'gemini-1.5-pro-002', provider: 'GCP Vertex AI' },
  { id: 28, name: 'gemini-2.5-pro-exp-03-25', provider: 'GCP Vertex AI' },
];
