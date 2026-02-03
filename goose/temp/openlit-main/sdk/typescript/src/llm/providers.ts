import { llmResponseOpenAI } from './openai';
import { llmResponseAnthropic } from './anthropic';

export type LlmResponseFn = (args: {
  prompt: string;
  model?: string;
  apiKey?: string;
  baseUrl?: string;
}) => Promise<string>;

export const llmProviders: Record<string, LlmResponseFn> = {
  openai: llmResponseOpenAI,
  anthropic: llmResponseAnthropic,
};
