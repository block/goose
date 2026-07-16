import type { ReasoningMode } from '../../../types/providers';

export function supportsReasoningMode(modelName: string | null | undefined): boolean {
  const normalized = modelName?.toLowerCase();
  return Boolean(
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
