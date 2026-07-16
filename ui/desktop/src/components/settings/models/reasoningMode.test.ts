import { describe, expect, it } from 'vitest';
import { parseReasoningMode, supportsReasoningMode } from './reasoningMode';

describe('GPT-5.6 reasoning mode', () => {
  it.each(['gpt-5.6', 'gpt-5.6-sol', 'gpt-5-6', 'gpt-5-6-sol-xhigh'])('supports %s', (model) => {
    expect(supportsReasoningMode('openai', model)).toBe(true);
  });

  it.each(['gpt-5.5', 'gpt-5.60', 'gpt-4o', null])('does not support %s', (model) => {
    expect(supportsReasoningMode('openai', model)).toBe(false);
  });

  it.each(['openai', 'databricks', 'databricks_v2', 'aws_bedrock', 'github_copilot'])(
    'supports provider %s when its request builder consumes reasoning_mode',
    (provider) => {
      expect(supportsReasoningMode(provider, 'gpt-5.6')).toBe(true);
    }
  );

  it('does not expose the mode for providers that ignore it', () => {
    expect(supportsReasoningMode('chatgpt_codex', 'gpt-5.6')).toBe(false);
    expect(supportsReasoningMode(null, 'gpt-5.6')).toBe(false);
  });

  it('accepts only standard and pro values', () => {
    expect(parseReasoningMode('standard')).toBe('standard');
    expect(parseReasoningMode('pro')).toBe('pro');
    expect(parseReasoningMode('turbo')).toBeNull();
  });
});
