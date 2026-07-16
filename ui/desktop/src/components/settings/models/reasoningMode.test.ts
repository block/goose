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
    expect(supportsReasoningMode('databricks_v2', model)).toBe(true);
  });

  it.each(['gpt-5.5', 'gpt-5.60', 'gpt-4o', null])('does not support %s', (model) => {
    expect(supportsReasoningMode('databricks_v2', model)).toBe(false);
  });

  it('does not strip lookalike prefixes', () => {
    expect(supportsReasoningMode('databricks_v2', 'my-openai.gpt-5.6')).toBe(false);
  });

  it.each(['databricks_v2', 'aws_bedrock', 'github_copilot'])(
    'supports provider %s when its request builder consumes reasoning_mode',
    (provider) => {
      expect(supportsReasoningMode(provider, 'gpt-5.6')).toBe(true);
    }
  );

  it('supports provider-prefixed catalog IDs', () => {
    expect(supportsReasoningMode('aws_bedrock', 'openai.gpt-5.6-sol')).toBe(true);
    expect(supportsReasoningMode('databricks', 'databricks-gpt-5.6-terra', true)).toBe(true);
    expect(supportsReasoningMode('databricks_v2', 'goose-gpt-5-6-sol')).toBe(true);
    expect(supportsReasoningMode('databricks', 'team-prod', true)).toBe(true);
    expect(supportsReasoningMode('databricks', 'team-prod', false)).toBe(false);
  });

  it('requires an explicit capability for route-dependent providers', () => {
    expect(supportsReasoningMode('openai', 'gpt-5.6', true)).toBe(true);
    expect(supportsReasoningMode('openai', 'gpt-5.6', false)).toBe(false);
    expect(supportsReasoningMode('openai', 'gpt-5.6')).toBe(false);
    expect(supportsReasoningMode('databricks', 'gpt-5.6')).toBe(false);
  });

  it('does not expose the mode for providers that ignore it', () => {
    expect(supportsReasoningMode('chatgpt_codex', 'gpt-5.6')).toBe(false);
    expect(supportsReasoningMode(null, 'gpt-5.6')).toBe(false);
    expect(supportsReasoningMode(null, 'gpt-5.6', true)).toBe(false);
  });

  it('accepts only standard and pro values', () => {
    expect(parseReasoningMode('standard')).toBe('standard');
    expect(parseReasoningMode('pro')).toBe('pro');
    expect(parseReasoningMode('turbo')).toBeNull();
  });
});
