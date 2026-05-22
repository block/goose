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

    expect(acpLoadSessionMeta(response)).toEqual({
      extensionResults,
      recipe: undefined,
      userRecipeValues: undefined,
      workingDir: undefined,
    });
  });

  it('extracts recipe session metadata from ACP response metadata', () => {
    const recipe = {
      title: 'Recipe Session',
      description: 'test recipe',
      instructions: 'Do the recipe',
    };
    const userRecipeValues = { target: 'desktop' };

    const response = {
      sessionId: 'session-1',
      _meta: {
        recipe,
        userRecipeValues,
        workingDir: '/tmp/project',
      },
    } as unknown as LoadSessionResponse;

    expect(acpLoadSessionMeta(response)).toEqual({
      extensionResults: undefined,
      recipe,
      userRecipeValues,
      workingDir: '/tmp/project',
    });
  });

  it('handles responses without metadata', () => {
    const response = { sessionId: 'session-1' } as unknown as LoadSessionResponse;

    expect(acpLoadSessionMeta(response)).toEqual({
      extensionResults: undefined,
      recipe: undefined,
      userRecipeValues: undefined,
      workingDir: undefined,
    });
  });
});
