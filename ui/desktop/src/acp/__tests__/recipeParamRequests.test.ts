import type { RequestRecipeParams_unstable } from '@aaif/goose-sdk';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  cancelAcpRecipeParamRequest,
  getAcpRecipeParamRequestsSnapshot,
  requestAcpRecipeParams,
  resolveAcpRecipeParamRequest,
} from '../recipeParamRequests';

vi.mock('../../acpChatFeatureFlag', () => ({
  USE_ACP_CHAT: true,
}));

function recipeParamRequest(): RequestRecipeParams_unstable {
  return {
    sessionId: 'session-1',
    parameters: [
      {
        key: 'topic',
        description: 'Topic',
        input_type: 'string',
        requirement: 'user_prompt',
      },
    ],
  };
}

function optionalRecipeParamRequest(): RequestRecipeParams_unstable {
  return {
    sessionId: 'session-1',
    parameters: [
      {
        key: 'tone',
        description: 'Tone',
        input_type: 'string',
        requirement: 'optional',
        default: 'concise',
      },
    ],
  };
}

function setRecipeParameters(values: Record<string, string>): void {
  Object.defineProperty(window, 'appConfig', {
    configurable: true,
    value: {
      get: vi.fn((key: string) => (key === 'recipeParameters' ? values : undefined)),
    },
  });
}

function cancelPendingRecipeParamRequests(): void {
  for (const request of getAcpRecipeParamRequestsSnapshot()) {
    cancelAcpRecipeParamRequest(request.id);
  }
}

describe('ACP recipe param requests', () => {
  beforeEach(() => {
    cancelPendingRecipeParamRequests();
  });

  afterEach(() => {
    cancelPendingRecipeParamRequests();
    Reflect.deleteProperty(window, 'appConfig');
  });

  it('keeps missing user_prompt parameters pending for user input', async () => {
    setRecipeParameters({});

    const response = requestAcpRecipeParams(recipeParamRequest());
    const [pendingRequest] = getAcpRecipeParamRequestsSnapshot();

    expect(pendingRequest).toMatchObject({
      sessionId: 'session-1',
      parameters: [
        {
          key: 'topic',
          requirement: 'user_prompt',
        },
      ],
      initialValues: {},
    });

    cancelAcpRecipeParamRequest(pendingRequest.id);
    await expect(response).resolves.toEqual({ action: 'cancel' });
  });

  it('keeps user_prompt parameters pending when configured values are available', async () => {
    setRecipeParameters({ topic: 'release notes' });

    const response = requestAcpRecipeParams(recipeParamRequest());
    const [pendingRequest] = getAcpRecipeParamRequestsSnapshot();

    expect(pendingRequest).toMatchObject({
      sessionId: 'session-1',
      parameters: [
        {
          key: 'topic',
          requirement: 'user_prompt',
        },
      ],
      initialValues: { topic: 'release notes' },
    });

    resolveAcpRecipeParamRequest(pendingRequest.id, { topic: 'release notes' });
    await expect(response).resolves.toEqual({
      action: 'submit',
      values: { topic: 'release notes' },
    });
  });

  it('keeps optional-only parameters pending for user confirmation', async () => {
    setRecipeParameters({});

    const response = requestAcpRecipeParams(optionalRecipeParamRequest());
    const [pendingRequest] = getAcpRecipeParamRequestsSnapshot();

    expect(pendingRequest).toMatchObject({
      sessionId: 'session-1',
      parameters: [
        {
          key: 'tone',
          requirement: 'optional',
          default: 'concise',
        },
      ],
      initialValues: {},
    });

    resolveAcpRecipeParamRequest(pendingRequest.id, { tone: 'detailed' });
    await expect(response).resolves.toEqual({
      action: 'submit',
      values: { tone: 'detailed' },
    });
  });
});
