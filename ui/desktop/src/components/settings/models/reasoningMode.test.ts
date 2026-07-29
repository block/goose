import { describe, expect, it } from 'vitest';
import {
  parseReasoningMode,
  reasoningModeForSelection,
  reasoningModeForSubmission,
  resolvedReasoningModeCapability,
  shouldSyncSessionReasoningMode,
  supportsReasoningMode,
} from './reasoningMode';

describe('GPT-5.6 reasoning mode', () => {
  it.each([
    'gpt-5.6',
    'gpt-5.6-sol',
    'gpt-5-6',
    'gpt-5-6-sol-xhigh',
    'openai.gpt-5.6-terra',
    'openai/gpt-5.6-terra',
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

  it('falls back to the model name for the default OpenAI route', () => {
    expect(supportsReasoningMode('openai', 'gpt-5.6', true)).toBe(true);
    expect(supportsReasoningMode('openai', 'gpt-5.6', false)).toBe(false);
    expect(supportsReasoningMode('openai', 'gpt-5.6-custom')).toBe(true);
    expect(supportsReasoningMode('openai', 'gpt-5.5')).toBe(false);
  });

  it('requires an explicit capability for route-dependent Databricks aliases', () => {
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

  it('prefers the resolved route capability over predefined metadata', () => {
    expect(resolvedReasoningModeCapability(false, true)).toBe(false);
    expect(resolvedReasoningModeCapability(true, false)).toBe(true);
    expect(resolvedReasoningModeCapability(null, true)).toBe(true);
    expect(resolvedReasoningModeCapability(undefined, undefined)).toBeNull();
  });

  it('preserves the session mode only for the current provider and model', () => {
    expect(reasoningModeForSelection('openai', 'gpt-5.6', 'openai', 'gpt-5.6', 'pro')).toBe('pro');
    expect(reasoningModeForSelection('databricks', 'gpt-5.6', 'openai', 'gpt-5.6', 'pro')).toBe(
      'standard'
    );
    expect(reasoningModeForSelection('openai', 'gpt-5.6-sol', 'openai', 'gpt-5.6', 'pro')).toBe(
      'standard'
    );
  });

  it('prefers the current session mode over a predefined model default', () => {
    expect(
      reasoningModeForSelection('openai', 'gpt-5.6', 'openai', 'gpt-5.6', 'pro', 'standard')
    ).toBe('pro');
    expect(
      reasoningModeForSelection('openai', 'gpt-5.6-sol', 'openai', 'gpt-5.6', 'pro', 'standard')
    ).toBe('standard');
  });

  it('preserves the current session mode while its capability is resolving', () => {
    expect(
      reasoningModeForSubmission(
        false,
        true,
        'databricks',
        'team-prod',
        'databricks',
        'team-prod',
        'pro',
        'standard'
      )
    ).toBe('pro');
    expect(
      reasoningModeForSubmission(
        false,
        true,
        'databricks',
        'team-next',
        'databricks',
        'team-prod',
        'pro',
        'standard'
      )
    ).toBeNull();
    expect(
      reasoningModeForSubmission(
        false,
        false,
        'databricks',
        'team-prod',
        'databricks',
        'team-prod',
        'pro',
        'standard'
      )
    ).toBeNull();
  });

  it('submits the selected mode when the control is available', () => {
    expect(
      reasoningModeForSubmission(
        true,
        false,
        'openai',
        'gpt-5.6',
        'openai',
        'gpt-5.6',
        'pro',
        'standard'
      )
    ).toBe('standard');
  });

  it('syncs a delayed session mode only for an untouched matching selection', () => {
    expect(
      shouldSyncSessionReasoningMode('openai', 'gpt-5.6', 'openai', 'gpt-5.6', 'pro', false)
    ).toBe(true);
    expect(
      shouldSyncSessionReasoningMode('openai', 'gpt-5.6', 'openai', 'gpt-5.6', 'pro', true)
    ).toBe(false);
    expect(
      shouldSyncSessionReasoningMode('openai', 'gpt-5.6-sol', 'openai', 'gpt-5.6', 'pro', false)
    ).toBe(false);
    expect(
      shouldSyncSessionReasoningMode('openai', 'gpt-5.6', 'openai', 'gpt-5.6', null, false)
    ).toBe(false);
  });
});
