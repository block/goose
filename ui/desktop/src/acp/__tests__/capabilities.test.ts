import type { InitializeResponse } from '@agentclientprotocol/sdk';
import { describe, expect, it } from 'vitest';
import { hasLocalInferenceCapability } from '../capabilities';

function initializeResponseWithMeta(meta?: unknown): Pick<InitializeResponse, 'agentCapabilities'> {
  return {
    agentCapabilities: {
      _meta: meta,
    },
  } as Pick<InitializeResponse, 'agentCapabilities'>;
}

describe('ACP capabilities', () => {
  it('detects local inference support from Goose metadata', () => {
    expect(
      hasLocalInferenceCapability(
        initializeResponseWithMeta({
          goose: {
            localInference: {},
          },
        })
      )
    ).toBe(true);
  });

  it('treats missing local inference metadata as unsupported', () => {
    expect(hasLocalInferenceCapability(initializeResponseWithMeta())).toBe(false);
    expect(hasLocalInferenceCapability(initializeResponseWithMeta({}))).toBe(false);
    expect(hasLocalInferenceCapability(initializeResponseWithMeta({ goose: {} }))).toBe(false);
  });

  it('ignores malformed Goose metadata', () => {
    expect(hasLocalInferenceCapability(initializeResponseWithMeta({ goose: true }))).toBe(false);
    expect(hasLocalInferenceCapability(initializeResponseWithMeta({ goose: null }))).toBe(false);
  });
});
