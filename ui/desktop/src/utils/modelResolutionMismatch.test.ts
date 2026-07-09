import { describe, expect, it } from 'vitest';
import { getModelResolutionMismatch } from './modelResolutionMismatch';

describe('getModelResolutionMismatch', () => {
  it('returns a mismatch when the provider resolves the current model to a different model', () => {
    expect(
      getModelResolutionMismatch({
        latestInference: {
          provider: 'claude-acp',
          requestedModel: 'claude-fable-5',
          resolvedModel: 'claude-opus-4-8',
        },
        currentProvider: 'claude-acp',
        currentModel: 'claude-fable-5',
        sessionId: 'session-1',
        latestInferenceMessageId: 'assistant-1',
        getDisplayName: (model) => `Display ${model}`,
      })
    ).toEqual({
      key: 'session-1:assistant-1',
      provider: 'claude-acp',
      requestedModel: 'claude-fable-5',
      requestedDisplayName: 'Display claude-fable-5',
      resolvedModel: 'claude-opus-4-8',
      resolvedDisplayName: 'Display claude-opus-4-8',
    });
  });

  it('ignores matching model resolution', () => {
    expect(
      getModelResolutionMismatch({
        latestInference: {
          provider: 'claude-acp',
          requestedModel: 'claude-fable-5',
          resolvedModel: 'claude-fable-5',
        },
        currentProvider: 'claude-acp',
        currentModel: 'claude-fable-5',
      })
    ).toBeNull();
  });

  it('ignores inference metadata for a previous provider or model', () => {
    expect(
      getModelResolutionMismatch({
        latestInference: {
          provider: 'claude-acp',
          requestedModel: 'claude-fable-5',
          resolvedModel: 'claude-opus-4-8',
        },
        currentProvider: 'openai',
        currentModel: 'claude-fable-5',
      })
    ).toBeNull();

    expect(
      getModelResolutionMismatch({
        latestInference: {
          provider: 'claude-acp',
          requestedModel: 'claude-fable-5',
          resolvedModel: 'claude-opus-4-8',
        },
        currentProvider: 'claude-acp',
        currentModel: 'claude-sonnet-5',
      })
    ).toBeNull();
  });

  it('ignores incomplete or loading model state', () => {
    expect(
      getModelResolutionMismatch({
        latestInference: {
          provider: 'claude-acp',
          requestedModel: 'claude-fable-5',
          resolvedModel: 'claude-opus-4-8',
        },
        currentProvider: 'claude-acp',
        currentModel: 'claude-fable-5',
        isModelLoading: true,
      })
    ).toBeNull();

    expect(
      getModelResolutionMismatch({
        latestInference: {
          provider: 'claude-acp',
          requestedModel: 'claude-fable-5',
          resolvedModel: null,
        },
        currentProvider: 'claude-acp',
        currentModel: 'claude-fable-5',
      })
    ).toBeNull();
  });
});
