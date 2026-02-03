export type EvalsProvider = 'openai' | 'anthropic';

export interface EvalsOptions {
  provider?: EvalsProvider;
  apiKey?: string;
  model?: string;
  baseUrl?: string;
  thresholdScore?: number;
  collectMetrics?: boolean;
  customCategories?: Record<string, string>;
}

export interface EvalsInput {
  prompt?: string;
  contexts?: string[];
  text: string;
}

export interface EvalsResult {
  verdict: 'yes' | 'no';
  evaluation: 'Hallucination' | 'Bias' | 'Toxicity';
  score: number;
  classification: string;
  explanation: string;
}
