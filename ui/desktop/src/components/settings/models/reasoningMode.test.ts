import { describe, expect, it } from 'vitest';
import { parseReasoningMode, supportsReasoningMode } from './reasoningMode';

describe('GPT-5.6 reasoning mode', () => {
  it.each(['gpt-5.6', 'gpt-5.6-sol', 'gpt-5-6', 'gpt-5-6-sol-xhigh'])('supports %s', (model) => {
    expect(supportsReasoningMode(model)).toBe(true);
  });

  it.each(['gpt-5.5', 'gpt-5.60', 'gpt-4o', null])('does not support %s', (model) => {
    expect(supportsReasoningMode(model)).toBe(false);
  });

  it('accepts only standard and pro values', () => {
    expect(parseReasoningMode('standard')).toBe('standard');
    expect(parseReasoningMode('pro')).toBe('pro');
    expect(parseReasoningMode('turbo')).toBeNull();
  });
});
