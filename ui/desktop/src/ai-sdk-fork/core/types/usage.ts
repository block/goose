export interface LanguageModelUsage {
  completionTokens: number;
  promptTokens: number;
  totalTokens: number;
}

export function calculateLanguageModelUsage(usage: LanguageModelUsage): LanguageModelUsage {
  return {
    completionTokens: usage.completionTokens,
    promptTokens: usage.promptTokens,
    totalTokens: usage.totalTokens,
  };
}
