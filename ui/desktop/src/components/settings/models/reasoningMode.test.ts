import { describe, expect, it } from 'vitest';
import { parseReasoningMode, supportsReasoningMode } from './reasoningMode';

describe('GPT-5.6 reasoning mode', () => {
  it.each([
    'gpt-5.6',
    'gpt-5.6-sol',
    'gpt-5-6',
    'gpt-5-6-sol-xhigh',
    'openai.gpt-5.6-terra',
    'databricks-gpt-5.6-luna',
    'goose-gpt-5-6-sol',
  ])('supports %s', (model) => {
    expect(supportsReasoningMode('openai', model)).toBe(true);
  });

  it.each(['gpt-5.5', 'gpt-5.60', 'gpt-4o', null])('does not support %s', (model) => {
    expect(supportsReasoningMode('openai', model)).toBe(false);
  });

  it('does not strip lookalike prefixes', () => {
    expect(supportsReasoningMode('openai', 'my-openai.gpt-5.6')).toBe(false);
  });

  it.each(['openai', 'databricks', 'databricks_v2', 'aws_bedrock', 'github_copilot'])(
    'supports provider %s when its request builder consumes reasoning_mode',
    (provider) => {
      expect(supportsReasoningMode(provider, 'gpt-5.6')).toBe(true);
    }
  );

  it('supports provider-prefixed catalog IDs', () => {
    expect(supportsReasoningMode('aws_bedrock', 'openai.gpt-5.6-sol')).toBe(true);
    expect(supportsReasoningMode('databricks', 'databricks-gpt-5.6-terra')).toBe(true);
    expect(supportsReasoningMode('databricks_v2', 'goose-gpt-5-6-sol')).toBe(true);
  });

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
