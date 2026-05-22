import type { LoadSessionResponse } from '@agentclientprotocol/sdk';
import { describe, expect, it } from 'vitest';
import { acpLoadSessionMeta } from '../sessions';

describe('acpLoadSessionMeta', () => {
  it('extracts extension results from ACP response metadata', () => {
    const extensionResults = [
      {
        name: 'developer',
        status: 'success',
      },
    ];

    const response = {
      sessionId: 'session-1',
      _meta: {
        extensionResults,
      },
    } as unknown as LoadSessionResponse;

    expect(acpLoadSessionMeta(response)).toEqual({ extensionResults });
  });

  it('handles responses without metadata', () => {
    const response = { sessionId: 'session-1' } as unknown as LoadSessionResponse;

    expect(acpLoadSessionMeta(response)).toEqual({ extensionResults: undefined });
  });
});
