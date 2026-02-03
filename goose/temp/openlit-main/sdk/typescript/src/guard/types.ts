// Types for Guard output and configuration

export interface GuardResult {
  score: number;
  verdict: 'yes' | 'no' | 'none';
  guard: string;
  classification: string;
  explanation: string;
}

export interface GuardConfig {
  provider?: 'openai' | 'anthropic';
  apiKey?: string;
  model?: string;
  baseUrl?: string;
  customRules?: Array<CustomRule>;
  validTopics?: string[];
  invalidTopics?: string[];
  thresholdScore?: number;
  collectMetrics?: boolean;
}

export interface CustomRule {
  pattern: string;
  classification: string;
  verdict?: 'yes' | 'no';
  guard?: string;
  score?: number;
  explanation?: string;
}
