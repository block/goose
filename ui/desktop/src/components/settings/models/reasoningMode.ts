import type { ReasoningMode } from '../../../types/providers';

const REASONING_MODE_PROVIDERS = new Set([
  'openai',
  'databricks',
  'databricks_v2',
  'aws_bedrock',
  'github_copilot',
]);

export function supportsReasoningMode(
  providerName: string | null | undefined,
  modelName: string | null | undefined
): boolean {
  const lower = modelName?.toLowerCase();
  const normalized = lower?.startsWith('openai.')
    ? lower.slice('openai.'.length)
    : lower?.startsWith('databricks-')
      ? lower.slice('databricks-'.length)
      : lower?.startsWith('goose-')
        ? lower.slice('goose-'.length)
        : lower;
  return Boolean(
    providerName &&
    REASONING_MODE_PROVIDERS.has(providerName) &&
    normalized &&
    (normalized === 'gpt-5.6' ||
      normalized.startsWith('gpt-5.6-') ||
      normalized === 'gpt-5-6' ||
      normalized.startsWith('gpt-5-6-'))
  );
}

export function parseReasoningMode(value: unknown): ReasoningMode | null {
  return value === 'standard' || value === 'pro' ? value : null;
}
