import { EvalsOptions, EvalsInput, EvalsResult } from './types';
import { formatPrompt, parseLlmResponse } from './utils';
import { llmProviders } from '../llm/providers';
import { recordEvalMetrics } from './metrics';

export abstract class BaseEval {
  protected provider: EvalsOptions['provider'];
  protected apiKey?: string;
  protected model?: string;
  protected baseUrl?: string;
  protected thresholdScore: number;
  protected collectMetrics: boolean;
  protected customCategories?: Record<string, string>;

  constructor(options: EvalsOptions = {}) {
    this.provider = options.provider || 'openai';
    this.apiKey = options.apiKey;
    this.model = options.model;
    this.baseUrl = options.baseUrl;
    this.thresholdScore = options.thresholdScore ?? 0.5;
    this.collectMetrics = options.collectMetrics ?? false;
    this.customCategories = options.customCategories;
  }

  abstract getSystemPrompt(): string;

  async measure(input: EvalsInput): Promise<EvalsResult> {
    const systemPrompt = this.getSystemPrompt();
    const prompt = formatPrompt(systemPrompt, input);
    const response = await this.llmResponse(prompt);
    const result = parseLlmResponse(response);
    if (this.collectMetrics) {
      this.recordMetrics(result);
    }
    return result;
  }

  protected async llmResponse(prompt: string): Promise<string> {
    const providerFn = llmProviders[this.provider as keyof typeof llmProviders];
    if (!providerFn) {
      throw new Error(`Unsupported provider: ${this.provider}`);
    }
    // Use a union type for options
    const options: { prompt: string; model?: string; apiKey?: string; baseUrl?: string } = {
      prompt,
      model: this.model,
      apiKey: this.apiKey,
    };
    if (this.provider === 'openai') {
      options.baseUrl = this.baseUrl;
    } else {
      delete options.baseUrl;
    }
    return providerFn(options);
  }

  protected recordMetrics(result: EvalsResult) {
    recordEvalMetrics(result, this.provider || 'unknown');
  }
}
